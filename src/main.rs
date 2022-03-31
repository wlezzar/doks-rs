extern crate core;

use structopt::StructOpt;

use cli::DoksOpts;

use crate::cli::cli_main;

mod model;
mod sources;
mod search;
mod cli;
mod utils;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    cli_main(DoksOpts::from_args()).await
}
