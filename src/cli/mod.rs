use std::convert::TryInto;
use std::path::PathBuf;

use anyhow::Context;
use structopt::StructOpt;
use tokio_stream::StreamExt;

use crate::cli::config::DoksConfig;
use crate::search::SearchEngine;
use crate::sources::DocumentSource;
use crate::utils::StreamUtils;

pub mod config;

#[derive(Debug, StructOpt)]
#[structopt(name = "doks")]
pub struct DoksOpts {
    #[structopt(short = "-n", default_value = "default")]
    pub namespace: String,

    #[structopt(parse(from_os_str), short = "-c", long = "--config")]
    pub config_file: PathBuf,

    #[structopt(subcommand)]
    pub cmd: DoksCommand,
}

#[derive(Debug, StructOpt)]
pub enum DoksCommand {
    Index,
    Search {
        query: String
    },
    Purge,
}

pub async fn cli_main(opts: DoksOpts) -> anyhow::Result<()> {
    let config = tokio::fs::read_to_string(&opts.config_file).await?;
    let config: DoksConfig = serde_json::from_str(config.as_str())?;

    match &opts.cmd {
        DoksCommand::Index => {
            let search: Box<dyn SearchEngine> = (&config.engine).try_into()?;
            for source in &config.sources {
                let source: Box<dyn DocumentSource> = source.try_into()?;
                let mut stream = source.fetch().batched(10);

                while let Some(documents) = stream.next().await {
                    let collected = documents
                        .into_iter()
                        .collect::<anyhow::Result<Vec<_>>>()
                        .context("Error occurred while fetching documents from source")?;

                    search.index(collected).await?;
                }
            }
        }
        DoksCommand::Search { query } => {
            let search: Box<dyn SearchEngine> = (&config.engine).try_into()?;
            let mut results = search.search(query)?;

            while let Some(result) = results.next().await {
                let document = result?;
                let json = serde_json::to_string(&document)?;

                println!("{}", json)
            }
        }
        DoksCommand::Purge => {}
    }

    Ok(())
}