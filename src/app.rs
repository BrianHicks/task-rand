use crate::config::Config;
use crate::task::Task;
use crate::taskwarrior::Taskwarrior;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use ratatui::Frame;

#[derive(Debug)]
pub struct App {
    tw: Taskwarrior,
    config: Config,

    current_task: Option<CurrentTask>,
}

#[derive(Debug)]
pub struct CurrentTask {
    task: Task,
    until: DateTime<Utc>,
}

impl App {
    pub fn new(tw: Taskwarrior, config: Config) -> Self {
        Self {
            tw,
            config,

            current_task: None,
        }
    }

    async fn latest_tasks(&mut self) -> Result<Vec<Task>> {
        self.tw
            .export()
            .with_urgency_coefficient("due", 0.0)
            .with_urgency_coefficient("age", 0.0)
            .with_urgency_coefficient("blocked", 0.0)
            .with_urgency_coefficient("blocking", 0.0)
            .with_filter("jirastatus.not:backlog")
            .call()
            .await
            .context("could not export tasks")
    }

    pub fn render(&self, frame: &mut Frame) {
        frame.render_widget("Hello, World!", frame.area());
    }
}
