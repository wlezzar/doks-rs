use anyhow;
use async_trait::async_trait;
use tokio_stream::Stream;

use crate::model::Document;

#[async_trait]
trait SearchEngine<T: Stream<Item=Document>> {
    async fn index<I: IntoIterator<Item=Document> + Send + 'static>(&self, documents: I) -> anyhow::Result<()>;
    fn search(&self, query: String) -> anyhow::Result<T>;
}

mod tantivy_impl;