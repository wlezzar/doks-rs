use std::pin::Pin;

use tokio_stream::Stream;

use crate::model::Document;

pub mod static_list;
pub mod fs;

// Send is required to use `batched(...)` on the stream.
pub type DocStream = Pin<Box<dyn Stream<Item=anyhow::Result<Document>> + Send>>;

pub trait DocumentSource {
    fn fetch(&self) -> DocStream;
}