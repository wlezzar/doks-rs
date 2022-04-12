use crate::model::Document;
use crate::sources::{DocStream, DocumentSource};

pub struct StaticDocumentSource {
    documents: Vec<Document>,
}

impl DocumentSource for StaticDocumentSource {
    fn fetch(&self) -> DocStream {
        Box::pin(tokio_stream::iter(self.documents.clone().into_iter().map(Ok)))
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Borrow;
    use std::collections::HashMap;

    use tokio::test;
    use tokio_stream::StreamExt;

    use crate::model::Document;

    use super::DocumentSource;
    use super::StaticDocumentSource;

    #[test]
    async fn test_stream_when_not_empty() -> anyhow::Result<()> {
        let documents = vec![
            Document {
                id: "doc1".to_string(),
                source: "source".to_string(),
                link: "link1".to_string(),
                content: "content1".to_string(),
                title: "title1".to_string(),
                metadata: HashMap::new(),
            },
            Document {
                id: "doc2".to_string(),
                source: "source".to_string(),
                link: "link2".to_string(),
                content: "content2".to_string(),
                title: "title2".to_string(),
                metadata: HashMap::new(),
            },
        ];

        let source = StaticDocumentSource { documents: documents.clone() };

        let stream = source.borrow().fetch();

        let collected = stream.collect::<anyhow::Result<Vec<Document>>>().await?;

        assert_eq!(collected, documents);

        anyhow::Ok(())
    }

    #[test]
    async fn test_when_stream_is_empty() -> anyhow::Result<()> {
        let source = StaticDocumentSource { documents: vec![] };
        let stream = source.borrow().fetch();
        let collected = stream.collect::<anyhow::Result<Vec<Document>>>().await?;

        assert_eq!(collected.len(), 0);

        anyhow::Ok(())
    }
}