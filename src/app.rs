use crate::config::Config;
use crate::task::Task;
use crate::taskwarrior::Taskwarrior;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};

#[derive(Debug)]
pub struct App {
    tw: Taskwarrior,
    config: Config,

    current_task: Option<CurrentTask>,
    all_tasks: Vec<Task>,
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
            all_tasks: Vec::new(),
        }
    }

    pub async fn refresh_tasks(&mut self) -> Result<()> {
        self.all_tasks = self
            .tw
            .export()
            .with_urgency_coefficient("due", 0.0)
            .with_urgency_coefficient("age", 0.0)
            .with_urgency_coefficient("blocked", 0.0)
            .with_urgency_coefficient("blocking", 0.0)
            .with_filter("jirastatus.not:backlog")
            .call()
            .await
            .context("could not export tasks")?;

        Ok(())
    }
}
