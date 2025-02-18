use crate::config::Config;
use crate::task::Task;
use crate::taskwarrior::Taskwarrior;
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use crossterm::event::{Event, KeyCode};
use rand::prelude::*;
use ratatui::Frame;

#[derive(Debug)]
pub struct App {
    tw: Taskwarrior,
    config: Config,

    now: DateTime<Utc>,
    doing: Activity,

    should_quit: bool,
}

impl App {
    pub fn new(tw: Taskwarrior, config: Config) -> Self {
        Self {
            tw,
            config,

            now: Utc::now(),
            doing: Activity::Nothing,

            should_quit: false,
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        frame.render_widget("Hello, World!", frame.area());
    }

    pub async fn handle_input(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Key(key_event) => match key_event.code {
                KeyCode::Char('q') => {
                    self.should_quit = true;

                    Ok(())
                }
                _ => Ok(()),
            },
            _ => Ok(()),
        }
    }

    pub async fn handle_tick(&mut self, now: DateTime<Utc>) -> Result<()> {
        if self.doing.is_nothing() {
            self.doing = self
                .choose_next_task(now)
                .await
                .context("could not set a task")?;
        }

        Ok(())
    }

    async fn available_tasks(&self) -> Result<Vec<Task>> {
        self.tw
            .export()
            .with_urgency_coefficient("due", 0.0)
            .with_urgency_coefficient("age", 0.0)
            .with_urgency_coefficient("blocked", 0.0)
            .with_urgency_coefficient("blocking", 0.0)
            .with_filter("jirastatus.not:backlog")
            .with_filter("+READY")
            .call()
            .await
            .context("could not get tasks")
    }

    async fn choose_next_task(&self, now: DateTime<Utc>) -> Result<Activity> {
        // This is inspired by the Gladden Design Paper Apps TO•DO, where you
        // roll a d6 to decide how long you're going to work. You take a break
        // if you roll a 6, and work for `roll*10` minutes otherwise. We use `0`
        // as our sentinel value instead.
        let minutes = rand::random_range(0..=5);

        if minutes == 0 {
            Ok(Activity::Break {
                until: now + Duration::minutes(10),
            })
        } else {
            let target_duration = Duration::minutes(minutes * 10);

            let tasks = self.available_tasks().await?;

            let task = tasks
                .choose_weighted(&mut rand::rng(), |task| task.urgency_at(now, &self.config))
                .context("could not choose a task")?;

            Ok(Activity::Task {
                task: task.clone(),
                until: now
                    + task
                        .estimate
                        .unwrap_or(target_duration)
                        .min(target_duration),
            })
        }
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }
}

#[derive(Debug)]
pub enum Activity {
    Nothing,
    Task { task: Task, until: DateTime<Utc> },
    Break { until: DateTime<Utc> },
}

impl Activity {
    pub fn is_nothing(&self) -> bool {
        matches!(self, Self::Nothing)
    }

    pub async fn mark_done(self, tw: &Taskwarrior) -> Result<Self> {
        match self {
            Self::Task { task, .. } => {
                tw.mark_done(&task.uuid).await?;

                Ok(Self::Nothing)
            }
            _ => Ok(Self::Nothing),
        }
    }
}
