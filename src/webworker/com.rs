use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub(super) struct PostInit {
    pub(crate) success: bool,
    #[serde(default)]
    pub(crate) message: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Request {
    pub(crate) id: usize,
    pub(crate) func_name: &'static str,
    #[serde(with = "serde_bytes")]
    pub(crate) arg: Box<[u8]>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Response {
    pub(crate) id: usize,
    #[serde(with = "serde_bytes")]
    pub(crate) response: Option<Vec<u8>>,
}
