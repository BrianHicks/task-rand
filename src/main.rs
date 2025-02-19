mod app;
mod config;
mod dates;
mod task;
mod taskwarrior;

use crate::app::App;
use anyhow::{bail, Context, Result};
use clap::Parser;
use futures::StreamExt;
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

    async fn run_ui(&self, mut app: App, mut terminal: DefaultTerminal) -> Result<()> {
        let mut events = crossterm::event::EventStream::new();
        let mut ticks = tokio::time::interval(tokio::time::Duration::from_secs(1));

        app.handle_tick()
            .await
            .context("could not handle initial tick")?;

        loop {
            terminal.draw(|frame| app.render(frame))?;

            tokio::select! {
                Some(Ok(event)) = events.next() => {
                    app.handle_input(event)
                        .await
                        .context("could not handle event")?;
                }

                _ = ticks.tick() => {
                    app.handle_tick()
                        .await
                        .context("could not handle tick")?;
                }
            }

            if app.should_quit() {
                break Ok(());
            }

            if let Some(mut command) = app.take_interactive() {
                ratatui::restore();

                let status = command.status().await.context("could not run command")?;

                terminal = ratatui::init();

                if !status.success() {
                    bail!("command failed with exit code {:?}", status.code())
                }

                app.refresh_doing()
                    .await
                    .context("could not refresh task after interactive session")?;
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
