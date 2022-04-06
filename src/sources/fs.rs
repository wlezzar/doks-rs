use std::collections::HashMap;

use async_walkdir::WalkDir;
use regex::Regex;
use tokio_stream::StreamExt;

use crate::model::Document;
use crate::sources::DocStream;
use crate::utils::streams::channel_stream;

use super::DocumentSource;

pub struct FileSystemDocumentSource {
    pub source_id: String,
    pub paths: Vec<String>,
    pub include: Vec<Regex>,
    pub exclude: Vec<Regex>,
}

impl DocumentSource for FileSystemDocumentSource {
    fn fetch(&self) -> DocStream {
        let paths = self.paths.clone();
        let source_id = self.source_id.clone();
        let include = self.include.clone();
        let exclude = self.exclude.clone();

        let stream = channel_stream(|tx| async move {
            for path in paths {
                let mut files = WalkDir::new(path);

                while let Some(file) = files.next().await {
                    let file = file?;
                    let path = file.path().to_string_lossy().to_string();

                    if file.metadata().await?.is_dir() {
                        continue;
                    }

                    log::debug!("Processing: {}", &path);

                    let matching = (&include)
                        .into_iter()
                        .all(|r| r.is_match(path.as_ref()));

                    let matching = matching && (&exclude)
                        .into_iter()
                        .all(|r| !r.is_match(path.as_ref()));

                    if !matching {
                        log::debug!("Ignoring file: {}", &path);
                        continue;
                    }

                    let content = tokio::fs::read_to_string(file.path()).await?;

                    tx.send(Ok(Document {
                        id: path.clone(),
                        source: source_id.to_string(),
                        title: file.file_name().to_string_lossy().to_string(),
                        link: path,
                        content,
                        metadata: HashMap::default(),
                    })).await?;
                }
            }

            Ok(())
        });

        Box::pin(stream)
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