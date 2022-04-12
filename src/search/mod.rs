use std::pin::Pin;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio_stream::Stream;

use crate::model::Document;
use crate::sources::DocStream;

#[derive(Serialize, Deserialize, Debug)]
pub struct FoundItem {
    pub id: String,
    pub score: f32,
    pub source: String,
    pub title: String,
    pub link: String,
    pub snippet: String,
}

type SearchResult = anyhow::Result<Pin<Box<dyn Stream<Item=anyhow::Result<FoundItem>> + Send>>>;

#[async_trait]
pub trait SearchEngine {
    async fn index(&self, documents: Vec<Document>) -> anyhow::Result<()>;
    fn search(&self, query: &str) -> SearchResult;
}

pub mod tantivy_impl;