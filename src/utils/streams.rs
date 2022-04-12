use std::future::Future;

use anyhow::anyhow;
use tokio::sync::mpsc::Sender;
use tokio_stream::Stream;

pub fn channel_stream<R, Fut>(
    action: impl FnOnce(Sender<anyhow::Result<R>>) -> Fut
) -> impl Stream<Item=anyhow::Result<R>>
    where R: Send + 'static,
          Fut: Future<Output=anyhow::Result<()>> + Send + 'static,
{
    let (tx, rx) = tokio::sync::mpsc::channel::<anyhow::Result<R>>(1);

    let manager_tx = tx.clone();
    let stream_tx = tx;

    let fut = action(stream_tx);

    tokio::spawn(async move {
        let mut error: Option<anyhow::Error> = None;

        match tokio::spawn(fut).await {
            Ok(Ok(_)) => log::debug!("Stream completed successfully!"),
            Ok(Err(err)) => {
                log::error!("stream had an error! Sending through the channel: {}", err);
                error.replace(err);
            }
            Err(err) => {
                log::error!("stream panicked! Sending through the channel");
                error.replace(anyhow!("Stream panicked: {}", err));
            }
        }

        if let Some(err) = error {
            if manager_tx.send(Err(err)).await.is_err() {
                log::error!("couldn't send error through channel (likely closed)");
            }
        }
    });

    tokio_stream::wrappers::ReceiverStream::new(rx)
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;

    use anyhow::bail;
    use tokio_stream::StreamExt;

    use crate::utils::streams::channel_stream;

    #[tokio::test]
    async fn test_stream_async_successful() -> anyhow::Result<()> {
        let stream = channel_stream(|tx| {
            async move {
                for i in 1..10 {
                    tx.send(Ok(i)).await?;
                }

                Ok(())
            }
        });

        let collected = stream.collect::<anyhow::Result<Vec<_>>>().await?;

        assert_eq!(
            collected,
            (1..10).collect::<Vec<_>>(),
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_stream_async_failure() -> anyhow::Result<()> {
        let stream = channel_stream(|tx| {
            async move {
                for i in 1..10 {
                    if i == 5 {
                        bail!("Deliberate failure!");
                    }
                    tx.send(Ok(i)).await?;
                }

                Ok(())
            }
        });

        let mut collected = stream.collect::<Vec<_>>().await;
        let error = collected.split_off(4);

        assert_eq!(
            collected.into_iter().collect::<anyhow::Result<Vec<_>>>()?,
            vec![1, 2, 3, 4],
        );

        assert_matches!(
            error[0],
            Err(_),
        );

        Ok(())
    }
}