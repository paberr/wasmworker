use std::{cell::RefCell, rc::Rc};

use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::mpsc;
use wasm_bindgen::prelude::*;
use web_sys::{MessageChannel, MessageEvent, MessagePort};

use crate::{
    convert::{from_bytes, to_bytes},
    error::InitError,
};

/// A bidirectional communication channel between the main thread and a WebWorker.
///
/// Channels allow workers to send messages back to the main thread during execution,
/// not just when returning results. This enables use cases like progress reporting,
/// DOM manipulation requests, and other interactive patterns.
#[derive(Clone)]
pub struct Channel {
    /// The message queue to await / incoming messages
    messages: Rc<RefCell<mpsc::UnboundedReceiver<JsValue>>>,
    /// The internal message port to send and receive data
    port: MessagePort,
}

impl Channel {
    /// Create two Channels to communicate between the WebWorker and the main application.
    ///
    /// The first channel is supposed to be used by the main application, the second one for the WebWorker.
    /// When a message is sent to one channel, it can be read from the second one and vice versa.
    pub fn new() -> Result<(Self, MessagePort), InitError> {
        let channel = MessageChannel::new().map_err(InitError::ChannelCreation)?;
        Ok((Self::from(channel.port1()), channel.port2()))
    }

    /// Create a Channel from pre-built components.
    ///
    /// This is used internally when the onmessage callback needs custom routing
    /// (e.g., to split user messages from result messages on the same port).
    pub(crate) fn from_parts(
        messages: mpsc::UnboundedReceiver<JsValue>,
        port: MessagePort,
    ) -> Self {
        Self {
            messages: Rc::new(RefCell::new(messages)),
            port,
        }
    }

    /// Handle messages received by the port and forwards them into the message stream
    fn on_message_callback(
        sender: mpsc::UnboundedSender<JsValue>,
    ) -> Closure<dyn FnMut(MessageEvent)> {
        Closure::new(move |event: MessageEvent| {
            let _ = sender.send(event.data());
        })
    }

    /// Receives the next value for this receiver.
    ///
    /// This method returns `None` if the channel has been closed and there are no remaining
    /// messages in the channel's buffer. If there are no messages in the channel's buffer,
    /// but the channel has not yet been closed, this method will sleep until a message is
    /// sent or the channel is closed.
    pub async fn recv<T: DeserializeOwned>(&self) -> Option<T> {
        let bytes = self.recv_bytes().await?;
        Some(from_bytes(&bytes))
    }

    /// Receives the next raw byte value for this receiver.
    #[allow(clippy::await_holding_refcell_ref)]
    pub async fn recv_bytes(&self) -> Option<Box<[u8]>> {
        // Note: Holding RefCell across await is safe in single-threaded WASM
        let mut messages = self.messages.borrow_mut();
        let value = messages.recv().await?;
        drop(messages);
        let array = js_sys::Uint8Array::new(&value);
        Some(array.to_vec().into_boxed_slice())
    }

    /// Send a value to the receiver.
    pub fn send<T: Serialize>(&self, msg: &T) {
        let bytes = to_bytes(msg);
        self.send_bytes(&bytes);
    }

    /// Send raw byte values to the receiver.
    pub fn send_bytes(&self, bytes: &[u8]) {
        let array = js_sys::Uint8Array::new_with_length(bytes.len() as u32);
        array.copy_from(bytes);
        self.port
            .post_message(&array)
            .expect("Channel is already closed");
    }
}

impl From<MessagePort> for Channel {
    /// Create a new Channel from a MessagePort
    fn from(port: MessagePort) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();

        let callback_handle = Self::on_message_callback(sender);
        port.set_onmessage(Some(callback_handle.as_ref().unchecked_ref()));
        callback_handle.forget();

        Self {
            messages: Rc::new(RefCell::new(receiver)),
            port,
        }
    }
}
