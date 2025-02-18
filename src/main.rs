mod app;
mod config;
mod dates;
mod task;
mod taskwarrior;

use crate::app::App;
use anyhow::{Context, Result};
use clap::Parser;
use crossterm::event::{self, Event};
use ratatui::DefaultTerminal;
use std::path::PathBuf;

#[derive(Debug, Parser)]
struct Cli {
    #[clap(long, default_value = "task")]
    task_bin: PathBuf,
}

impl Cli {
    async fn run(&self) -> Result<()> {
        let tw = taskwarrior::Taskwarrior::new(self.task_bin.clone());

        let config = tw
            .config()
            .await
            .context("could not get taskwarrior config")?;

        let app = App::new(tw, config);

        let terminal = ratatui::init();
        let result = self.run_ui(app, terminal).await;
        ratatui::restore();

        result
    }

    async fn run_ui(&self, app: App, mut terminal: DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|frame| app.render(frame))?;

            if matches!(event::read()?, Event::Key(_)) {
                break Ok(());
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let app = Cli::parse();

    if let Err(err) = app.run().await {
        eprintln!("{:#}", err);
        std::process::exit(1);
    }
}
