use std::pin::Pin;
use std::task::{Context, Poll};

use tokio_stream::Stream;

use crate::model::Document;
use crate::sources::DocumentSource;

pub struct StaticDocumentSource {
    documents: Vec<Document>,
}

impl<'a> DocumentSource<StaticDocumentSourceStream<'a>> for &'a StaticDocumentSource {
    fn fetch(self) -> StaticDocumentSourceStream<'a> {
        StaticDocumentSourceStream {
            documents: self.documents.as_slice(),
            current_position: 0,
        }
    }
}

struct StaticDocumentSourceStream<'a> {
    documents: &'a [Document],
    current_position: usize,
}

impl<'a> Stream for StaticDocumentSourceStream<'a> {
    type Item = anyhow::Result<Document>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        return if self.current_position >= self.documents.len() {
            Poll::Ready(None)
        } else {
            self.current_position = self.current_position + 1;
            Poll::Ready(Some(anyhow::Ok(self.documents[self.current_position - 1].clone())))
        };
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