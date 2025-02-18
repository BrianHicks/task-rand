use std::path::PathBuf;

mod config;
mod dates;
mod task;
mod taskwarrior;

use anyhow::{Context, Result};
use clap::Parser;

#[derive(Debug, Parser)]
struct App {
    #[clap(long, default_value = "task")]
    task_bin: PathBuf,
}

impl App {
    async fn run(&self) -> Result<()> {
        let tw = taskwarrior::Taskwarrior::new(self.task_bin.clone());

        let config = tw
            .config()
            .await
            .context("could not get taskwarrior config")?;

        println!("{:#?}", config);

        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let app = App::parse();

    if let Err(err) = app.run().await {
        eprintln!("{:#}", err);
        std::process::exit(1);
    }
}
