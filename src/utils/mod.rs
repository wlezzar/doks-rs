pub mod json;
pub mod streams;

use std::mem;

use futures::pin_mut;
use tokio_stream::{Stream, StreamExt};
use tokio_stream::wrappers::ReceiverStream;

pub trait StreamUtils: Stream {
    fn batched(self, size: usize) -> ReceiverStream<Vec<Self::Item>>
        where
            Self: Sized + Send + 'static,
            Self::Item: Send,
    {
        let (tx, rx) = tokio::sync::mpsc::channel(1);

        tokio::task::spawn(async move {
            let tx = tx;
            let size = size;
            let stream = self;

            pin_mut!(stream);

            let mut batch = Vec::<Self::Item>::new();

            while let Some(item) = stream.next().await {
                batch.push(item);

                if batch.len() >= size {
                    if let Err(_) = tx.send(mem::take(&mut batch)).await {
                        log::warn!("Sender closed!");
                    };
                }
            }

            if !batch.is_empty() {
                if let Err(_) = tx.send(batch).await {
                    log::warn!("Couldn't send batch downstream!");
                };
            }
        });

        tokio_stream::wrappers::ReceiverStream::new(rx)
    }
}

impl<St> StreamUtils for St where St: Stream {}

#[cfg(test)]
mod tests {
    use futures::StreamExt;

    use super::StreamUtils;

    #[tokio::test]
    async fn stream_utils_batched_test() -> anyhow::Result<()> {
        let stream = tokio_stream::iter(1..13).batched(5);
        let collected = stream.collect::<Vec<_>>().await;

        assert_eq!(
            collected,
            vec![(1..6).collect::<Vec<_>>(), (6..11).collect::<Vec<_>>(), (11..13).collect::<Vec<_>>()]
        );

        Ok(())
    }
}