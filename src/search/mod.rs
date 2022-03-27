use crate::model::Document;
use anyhow;
use tokio_stream::Stream;
use async_trait::async_trait;

#[async_trait]
trait SearchEngine<T: Stream<Item = Document>> {
	async fn index(document: Document) -> anyhow::Result<()>;
	async fn search(query: String) -> T;
}
