use std::pin::Pin;

use anyhow::Context;
use git2::build::RepoBuilder;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tempdir::TempDir;
use tokio::task::JoinHandle;
use tokio_stream::{Stream, StreamExt};

use fs::FileSystemDocumentSource;

use crate::sources::{DocStream, DocumentSource, fs};
use crate::utils::json::get_array;
use crate::utils::json::parse_json;
use crate::utils::streams::channel_stream;

pub struct GithubSource {
    pub source_id: String,
    pub lister: Box<dyn GitRepositoryLister>,
    pub include: Vec<Regex>,
    pub exclude: Vec<Regex>,
}

impl DocumentSource for GithubSource {
    fn fetch(&self) -> DocStream {
        let mut repositories = self.lister.list();
        let source_id = self.source_id.clone();
        let include = self.include.clone();
        let exclude = self.include.clone();

        Box::pin(
            channel_stream(|tx| async move {
                while let Some(repository) = repositories.next().await {
                    // Clone the repo
                    let repository = repository?;
                    let dest = TempDir::new("cloned")?;

                    let path = dest.path().to_owned();
                    let clone_task: JoinHandle<anyhow::Result<_>> = tokio::task::spawn_blocking(move || {
                        log::info!("Cloning repository '{}' into {:?}", &repository.clone_url, &path);
                        RepoBuilder::default().clone(&repository.clone_url, &path)?;
                        std::fs::remove_dir_all(path.join(".git"))?;
                        Ok(())
                    });

                    clone_task
                        .await
                        .context("Clone task panicked!")?
                        .context("Error while cloning repository")?;


                    let source = FileSystemDocumentSource {
                        source_id: source_id.clone(),
                        paths: vec![dest.path().to_string_lossy().to_string()],
                        include: include.clone(),
                        exclude: exclude.clone(),
                    };

                    let mut documents = source.fetch();

                    while let Some(document) = documents.next().await {
                        tx.send(document).await?;
                    }
                }

                Ok(())
            })
        )
    }
}

pub trait GitRepositoryLister {
    fn list(&self) -> Pin<Box<dyn Stream<Item=anyhow::Result<RepositoryInfo>> + Send>>;
}

pub struct GithubStarsLister {
    client: octocrab::Octocrab,
    starred_by: String,
}

impl GitRepositoryLister for GithubStarsLister {
    fn list(&self) -> Pin<Box<dyn Stream<Item=anyhow::Result<RepositoryInfo>> + Send>> {
        let client = self.client.clone();
        let starred_by = self.starred_by.clone();

        let stream = channel_stream(|tx| async move {
            let mut page_info: Option<PageInfo> = None;

            loop {
                let query = gh_starred_gql_query(
                    starred_by.as_str(),
                    page_info.take().map(|v| v.end_cursor),
                );

                let page: Value = client.graphql(&query).await?;

                let nodes = get_array(&page, &["data", "user", "starredRepositories", "nodes"])?;
                let current_page_info: PageInfo = parse_json(
                    &page, &["data", "user", "starredRepositories", "pageInfo"],
                )?;

                for item in nodes {
                    let parsed = serde_json::from_value::<RepositoryInfo>(item.clone())
                        .with_context(|| format!("Couldn't parse json into a repository: {}", item));

                    tx.send(parsed).await?;
                }

                if !&current_page_info.has_next_page {
                    break;
                }

                page_info.replace(current_page_info);
            }

            Ok(())
        });

        Box::pin(stream)
    }
}

fn gh_starred_gql_query(starred_by: &str, start_cursor: Option<String>) -> String {
    format!(
        r#"query {{
          user(login:"{}") {{
            starredRepositories(
                first: 2,
                orderBy: {{ field: STARRED_AT, direction:DESC }},
                after: "{}"
            ) {{
              pageInfo {{
                startCursor
                endCursor
                hasNextPage
              }}
              nodes {{
                sshUrl
                url
                name
              }}
            }}
          }}
        }}
        "#,
        starred_by,
        start_cursor.unwrap_or_default(),
    )
}

#[derive(Clone)]
pub struct GithubRepoStaticList {
    pub list: Vec<RepositoryInfo>,
}

impl GitRepositoryLister for GithubRepoStaticList {
    fn list(&self) -> Pin<Box<dyn Stream<Item=anyhow::Result<RepositoryInfo>> + Send>> {
        Box::pin(tokio_stream::iter(
            self.list
                .iter()
                .map(|e| Ok(e.clone()))
                .collect::<Vec<_>>()
        ))
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    has_next_page: bool,
    start_cursor: String,
    end_cursor: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryInfo {
    pub name: String,
    #[serde(alias = "url")]
    pub clone_url: String,
}
