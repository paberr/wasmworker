use std::{cell::RefCell, rc::Rc};

use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::mpsc;
use wasm_bindgen::prelude::*;
use web_sys::{MessageChannel, MessageEvent, MessagePort};

use crate::{
    convert::{from_bytes, to_bytes},
    error::InitError,
};

/// An internal type for the callback.
type Callback = dyn FnMut(MessageEvent);

#[derive(Clone)]
pub struct Channel {
    /// The message queue to await / incoming messages
    messages: Rc<RefCell<mpsc::UnboundedReceiver<JsValue>>>,
    /// The internal message port to send and receive data
    port: MessagePort,
    // The callback handle for the messages
    //_callback: Closure<Callback>,
}

impl Channel {
    /// Create two Channels to communicate between the WebWorker and the main application
    /// The first channel is supposed to be used by the main application, the second one for the WebWorker.
    /// When a message is send to one channel, it can be read from the second one and vice versa.
    pub fn new() -> Result<(Self, MessagePort), InitError> {
        // the message channel which creates two ports, which can be transfered to a WebWorker
        let channel = MessageChannel::new().map_err(|e| InitError::ChannelCreation(e))?;
        Ok((Self::from(channel.port1()), channel.port2()))
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
    /// This method returns None if the channel has been closed and there are no remaining messages in the channel’s buffer. This indicates that no further values can ever be received from this Receiver. The channel is closed when all senders have been dropped, or when [close] is called.
    /// If there are no messages in the channel’s buffer, but the channel has not yet been closed, this method will sleep until a message is sent or the channel is closed.
    pub async fn recv<T: DeserializeOwned>(&self) -> Option<T> {
        let bytes = self.recv_bytes().await?;
        Some(from_bytes(&bytes))
    }

    /// Receives the next value for this receiver.
    pub async fn recv_bytes(&self) -> Option<Box<[u8]>> {
        let mut messages = self.messages.borrow_mut();
        let value = messages.recv().await?;
        let array = js_sys::Uint8Array::new(&value);
        Some(array.to_vec().into_boxed_slice())
    }

    /// Send a value to the receiver
    pub fn send<T: Serialize>(&self, msg: &T) {
        let bytes = to_bytes(msg);
        self.send_bytes(&bytes);
    }

    /// Send byte values to the receiver
    pub fn send_bytes(&self, bytes: &Box<[u8]>) {
        let array = js_sys::Uint8Array::new_with_length(bytes.len() as u32);
        array.copy_from(&bytes);
        self.port
            .post_message(&array)
            .expect("Channel is already closed");
    }
}

impl From<MessagePort> for Channel {
    /// Create a new Channel from a MessagePort
    fn from(port: MessagePort) -> Self {
        // the internal message sender / receiver to have the messages easy available in rust
        let (sender, receiver) = mpsc::unbounded_channel();

        // handle messages which have been received by the other port
        let callback_handle = Self::on_message_callback(sender);
        port.set_onmessage(Some(callback_handle.as_ref().unchecked_ref()));
        callback_handle.forget();

        // Return the channel
        Self {
            messages: Rc::new(RefCell::new(receiver)),
            port,
            //_callback: callback_handle,
        }
    }
}
