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

        let tasks = tw
            .export()
            .with_urgency_coefficient("due", 0.0)
            .with_urgency_coefficient("age", 0.0)
            .with_urgency_coefficient("blocked", 0.0)
            .with_urgency_coefficient("blocking", 0.0)
            .with_filter("jirastatus.not:backlog")
            .call()
            .await
            .context("could not export tasks")?;

        println!("{:#?}", config);
        println!("{:#?}", tasks);

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
