use std::convert::TryInto;
use std::path::PathBuf;

use anyhow::bail;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::search::SearchEngine;
use crate::search::tantivy_impl::TantivySearchEngine;
use crate::sources::DocumentSource;
use crate::sources::fs::FileSystemDocumentSource;
use crate::sources::gh::{GithubRepoStaticList, GithubSource, GitRepositoryLister, RepositoryInfo};

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct DoksConfig {
    pub sources: Vec<SourceConfig>,
    #[serde(default)]
    pub engine: SearchEngineConfig,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(tag = "source")]
pub enum SourceConfig {
    #[serde(alias = "github")]
    Github {
        id: String,
        repositories: GithubRepositoriesConfig,
        #[serde(default)]
        include: Vec<String>,
        #[serde(default)]
        exclude: Vec<String>,
    },
    #[serde(alias = "fs")]
    FileSystem {
        id: String,
        paths: Vec<String>,
        #[serde(default)]
        include: Vec<String>,
        #[serde(default)]
        exclude: Vec<String>,
    },
}

impl SourceConfig {
    pub fn id(&self) -> &str {
        match self {
            SourceConfig::Github { ref id, .. } => id.as_str(),
            SourceConfig::FileSystem { ref id, .. } => id.as_str(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(tag = "from")]
pub enum GithubRepositoriesConfig {
    #[serde(alias = "list")]
    FromList {
        server: Option<String>,
        #[serde(default)]
        transport: GitCloneTransport,
        list: Vec<GithubRepo>,
    },

    #[serde(alias = "api")]
    FromApi {
        search: Option<String>,
        starred_by: Option<Vec<String>>,
        endpoint: Option<String>,
        token_file: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct GithubRepo {
    name: String,
    folder: Option<String>,
    branch: Option<String>,
    #[serde(default)]
    include: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum GitCloneTransport {
    Ssh,
    Https,
}

impl Default for GitCloneTransport {
    fn default() -> Self {
        GitCloneTransport::Ssh
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(tag = "use")]
pub enum SearchEngineConfig {
    #[serde(alias = "tantivy")]
    Tantivy { path: PathBuf }
}

impl Default for SearchEngineConfig {
    fn default() -> Self {
        SearchEngineConfig::Tantivy { path: PathBuf::from("/tmp/doks_index") }
    }
}

impl TryInto<Box<dyn SearchEngine>> for &SearchEngineConfig {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Box<dyn SearchEngine>, Self::Error> {
        match self {
            SearchEngineConfig::Tantivy { path } => {
                Ok(Box::new(TantivySearchEngine::new(path)?))
            }
        }
    }
}

impl TryInto<Box<dyn GitRepositoryLister>> for &GithubRepositoriesConfig {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Box<dyn GitRepositoryLister>, Self::Error> {
        match self {
            GithubRepositoriesConfig::FromList { server, transport, list } => {
                let server = server.as_ref()
                    .map(|s| s.to_string())
                    .unwrap_or("github.com".to_string());

                Ok(
                    Box::new(GithubRepoStaticList {
                        list: list
                            .iter()
                            .map(|repo| RepositoryInfo {
                                name: repo.name.clone(),
                                clone_url: match transport {
                                    GitCloneTransport::Ssh => format!("git@{}:{}.git", server, repo.name),
                                    GitCloneTransport::Https => format!("https://{}/{}.git", server, repo.name),
                                },
                            })
                            .collect()
                    })
                )
            }
            GithubRepositoriesConfig::FromApi { .. } => {
                bail!("Not yet supported");
            }
        }
    }
}

impl TryInto<Box<dyn DocumentSource>> for &SourceConfig {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Box<dyn DocumentSource>, Self::Error> {
        match self {
            SourceConfig::Github { id, repositories, include, exclude } => {
                let lister: Box<dyn GitRepositoryLister> = repositories.try_into()?;

                Ok(
                    Box::new(
                        GithubSource {
                            source_id: id.to_string(),
                            lister,
                            include: include.iter()
                                .map(|e| Regex::new(e.as_str()))
                                .collect::<Result<_, _>>()?,
                            exclude: exclude.iter()
                                .map(|e| Regex::new(e.as_str()))
                                .collect::<Result<_, _>>()?,
                        }
                    )
                )
            }
            SourceConfig::FileSystem { id, include, exclude, paths } => {
                Ok(
                    Box::new(
                        FileSystemDocumentSource {
                            source_id: id.to_string(),
                            include: include.iter().map(|e| Regex::new(e.as_str())).collect::<Result<_, _>>()?,
                            exclude: exclude.iter().map(|e| Regex::new(e.as_str())).collect::<Result<_, _>>()?,
                            paths: paths.to_vec(),
                        }
                    )
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::cli::config::{DoksConfig, GitCloneTransport, GithubRepo};
    use crate::cli::config::GithubRepositoriesConfig::FromList;
    use crate::cli::config::SearchEngineConfig::Tantivy;
    use crate::cli::config::SourceConfig::Github;

    #[test]
    fn test_config_parse() -> anyhow::Result<()> {
        let config = r#"
            {
              "sources": [{
                  "id": "github",
                  "source": "github",
                  "repositories": {
                    "from": "list",
                    "list": [
                      { "name": "wlezzar/jtab" },
                      { "name": "wlezzar/doks" },
                      { "name": "adevinta/zoe" }
                    ]
                  }
              }],
              "engine": {"use": "tantivy", "path": "/tmp/doks_index" }
            }
        "#;

        let parsed = serde_json::from_str::<DoksConfig>(config)?;
        let expected = DoksConfig {
            sources: vec![
                Github {
                    id: "github".to_string(),
                    repositories: FromList {
                        server: None,
                        transport: GitCloneTransport::Ssh,
                        list: vec![
                            GithubRepo {
                                name: "wlezzar/jtab".to_string(),
                                folder: None,
                                branch: None,
                                include: Vec::default(),
                                exclude: Vec::default(),
                            },
                            GithubRepo {
                                name: "wlezzar/doks".to_string(),
                                folder: None,
                                branch: None,
                                include: Vec::default(),
                                exclude: Vec::default(),
                            },
                            GithubRepo {
                                name: "adevinta/zoe".to_string(),
                                folder: None,
                                branch: None,
                                include: Vec::default(),
                                exclude: Vec::default(),
                            }],
                    },
                    include: Vec::default(),
                    exclude: Vec::default(),
                }],
            engine: Tantivy { path: PathBuf::from("/tmp/doks_index") },
        };

        assert_eq!(parsed, expected);

        Ok(())
    }
}