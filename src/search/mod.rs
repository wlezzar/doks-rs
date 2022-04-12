
use async_trait::async_trait;

use crate::model::Document;
use crate::sources::DocStream;

#[async_trait]
pub trait SearchEngine {
    async fn index(&self, documents: Vec<Document>) -> anyhow::Result<()>;
    fn search(&self, query: &str) -> anyhow::Result<DocStream>;
}

pub mod tantivy_impl;