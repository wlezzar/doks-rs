use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Context, Poll};

use async_stream::try_stream;
use async_walkdir::WalkDir;
use regex::Regex;
use tokio_stream::Stream;

use crate::model::Document;

use super::DocumentSource;

pub struct FileSystemDocumentSource {
    source_id: String,
    paths: Vec<String>,
    include: Vec<Regex>,
    exclude: Vec<Regex>,
}

impl<'a> DocumentSource<FileSystemDocumentSourceStream<'a>> for &'a FileSystemDocumentSource {
    fn fetch(self) -> FileSystemDocumentSourceStream<'a> {
        FileSystemDocumentSourceStream::new(
            &self.source_id,
            self.paths.as_slice(),
            self.include.as_slice(),
            self.exclude.as_slice(),
        )
    }
}

struct FileSystemDocumentSourceStream<'a> {
    stream: Pin<Box<dyn Stream<Item=anyhow::Result<Document>> + 'a>>,
}

impl<'a> FileSystemDocumentSourceStream<'a> {
    fn new(
        source_id: &'a str,
        paths: &'a [String],
        include: &'a [Regex],
        exclude: &'a [Regex],
    ) -> FileSystemDocumentSourceStream<'a> {
        let stream = try_stream! {
            for path in paths {
                for await file in WalkDir::new(path) {
                    let file = file?;
                    let path = file.path().to_string_lossy().to_string();

                    let matching = include
                        .into_iter()
                        .all(|r| r.is_match(path.as_ref()));

                    let matching = matching && exclude
                        .into_iter()
                        .all(|r| !r.is_match(path.as_ref()));

                    if !matching {
                        continue;
                    }

                    let content = tokio::fs::read_to_string(file.path()).await?;

                    yield Document {
                        id: path.clone(),
                        source: source_id.to_string(),
                        title: file.file_name().to_string_lossy().to_string(),
                        link: path,
                        content,
                        metadata: HashMap::default(),
                    }
                }
            }
        };

        FileSystemDocumentSourceStream { stream: Box::pin(stream) }
    }
}

impl<'a> Stream for FileSystemDocumentSourceStream<'a> {
    type Item = anyhow::Result<Document>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.stream.as_mut().poll_next(cx)
    }
}

#[cfg(test)]
mod tests {
    use regex::Regex;
    use tempdir::TempDir;
    use tokio_stream::StreamExt;

    use crate::sources::fs::FileSystemDocumentSource;

    use super::DocumentSource;

    #[tokio::test]
    async fn first_test() -> anyhow::Result<()> {
        let root = TempDir::new("doks-tests")?;

        let files = vec![
            (root.path().join("file1.txt").to_string_lossy().to_string(), "content file1".to_string()),
            (root.path().join("file2.txt").to_string_lossy().to_string(), "content file 2".to_string()),
            (root.path().join("nested/file3.txt").to_string_lossy().to_string(), "content file 3".to_string()),
        ];

        for (path, content) in &files {
            let path = root.path().join(path);

            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    tokio::fs::create_dir_all(parent).await?;
                }
            }

            tokio::fs::write(path, content).await?;
        }

        let source = FileSystemDocumentSource {
            include: vec![Regex::new(".*.txt")?],
            exclude: vec![],
            paths: vec![root.path().to_string_lossy().to_string()],
            source_id: String::from("source1"),
        };

        let mut collected = (&source).fetch()
            .map(|file| file.map(|file| (file.link, file.content)))
            .collect::<anyhow::Result<Vec<_>>>()
            .await?;

        collected.sort();

        assert_eq!(collected, files);

        Ok(())
    }
}