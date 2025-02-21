use crate::config::Config;
use crate::task::Task;
use crate::taskwarrior::Taskwarrior;
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Local, Utc};
use crossterm::event::{Event, KeyCode};
use itertools::Itertools;
use rand::prelude::*;
use ratatui::{
    layout::{Constraint, Flex, Layout},
    style::{palette::tailwind, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Gauge, Paragraph, Wrap},
    Frame,
};
use tokio::process::Command;

#[derive(Debug)]
pub struct App {
    tw: Taskwarrior,
    config: Config,

    /// This is the thing we're doing *right now*
    doing: Activity,

    /// If we need to do interactive work (e.g. editing a task) we need to get
    /// out of the interactive terminal temporarily. We signal to the main loop
    /// that we need to do this by setting this field to `Some(Command)`. The
    /// main loop will run the command and then set this field back to `None`.
    interactive: Option<Command>,

    /// The main loop uses this as a signal that it should exit.
    should_quit: bool,
}

impl App {
    pub fn new(tw: Taskwarrior, config: Config) -> Self {
        Self {
            tw,
            config,

            doing: Activity::Nothing,
            interactive: None,
            should_quit: false,
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let [app_area, status_line_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());

        let app_box_vert = Layout::vertical([Constraint::Length(7)]).flex(Flex::Center);
        let app_box_horiz = Layout::horizontal([Constraint::Percentage(75)]).flex(Flex::Center);

        let [app_box_area] = app_box_vert.areas(app_area);
        let [app_box_area] = app_box_horiz.areas(app_box_area);

        let (title, gauge) = match &self.doing {
            Activity::Nothing => (
                Paragraph::new(Text::from("Nothing to do right now")),
                Gauge::default()
                    .label("0:00")
                    .ratio(100.0)
                    .use_unicode(true),
            ),
            Activity::Task {
                task,
                started,
                length,
                ..
            } => {
                let time_remaining = *length - (Utc::now() - started);

                let percent_elapsed = 1.0
                    - (time_remaining.num_seconds() as f64 / length.num_seconds() as f64)
                        .clamp(0.0, 1.0);

                let mut sections = vec![Span::from(format!("{}", task.id)).bold()];

                if let Some(jira) = &task.jira {
                    sections.push(Span::from(" / "));
                    sections.push(Span::from(jira).bold());
                }

                sections.push(Span::from(":").bold());
                sections.push(Span::from(" "));
                sections.push(Span::from(&task.description));

                if !task.tags.is_empty() {
                    sections.push(Span::from(" "));
                    sections.push(
                        Span::from(task.tags.iter().map(|tag| format!("+{}", tag)).join(" "))
                            .bold(),
                    );
                }

                if let Some(project) = &task.project {
                    sections.push(Span::from(" "));
                    sections.push(Span::from("pro:").bold());
                    sections.push(Span::from(project));
                }

                if let Some(due) = &task.due {
                    let remaining = *due - Utc::now();

                    let remaining_display = if remaining.num_seconds().abs() < 60 {
                        format!("{}s", remaining.num_seconds())
                    } else if remaining.num_minutes().abs() < 60 {
                        format!("{}m", remaining.num_minutes())
                    } else if remaining.num_hours().abs() < 24 {
                        format!("{}h", remaining.num_hours())
                    } else if remaining.num_days().abs() < 14 {
                        format!("{}d", remaining.num_days())
                    } else {
                        due.format("%Y-%m-%d").to_string()
                    };

                    let remaining_style = if remaining.num_seconds() < 0 {
                        Style::default().fg(tailwind::RED.c800)
                    } else {
                        Style::default()
                    };

                    sections.push(Span::from(" "));
                    sections.push(Span::from("due:").bold());
                    sections.push(Span::styled(remaining_display, remaining_style));
                }

                if !task.annotations.is_empty() {
                    sections.push(Span::from(" [A]").bold());
                }

                sections.push(Span::from(" "));
                sections.push(
                    Span::from(format!(
                        "until {}",
                        (*started + *length)
                            .with_timezone(&Local)
                            .format("%-I:%M %P")
                    ))
                    .italic()
                    .dim(),
                );

                (
                    Paragraph::new(Line::from(sections))
                        .centered()
                        .wrap(Wrap { trim: false }),
                    Gauge::default()
                        .label(format_remaining(time_remaining))
                        .gauge_style(gauge_style(time_remaining < Duration::zero()))
                        .ratio(percent_elapsed)
                        .use_unicode(true),
                )
            }
            Activity::Break {
                started, length, ..
            } => {
                let time_remaining = *length - (Utc::now() - started);

                let percent_remaining = (time_remaining.num_seconds() as f64
                    / length.num_seconds() as f64)
                    .clamp(0.0, 1.0);

                (
                    Paragraph::new(format!(
                        "Taking a break until {}",
                        (*started + *length)
                            .with_timezone(&Local)
                            .format("%-I:%M %p")
                    ))
                    .centered(),
                    Gauge::default()
                        .label(format_remaining(time_remaining))
                        .gauge_style(gauge_style(time_remaining < Duration::zero()))
                        .ratio(percent_remaining)
                        .use_unicode(true),
                )
            }
        };

        let [title_area, gauge_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).areas(app_box_area);

        frame.render_widget(title, title_area);
        frame.render_widget(gauge, gauge_area);

        frame.render_widget(
            Line::from(vec![
                Span::from("d").bold(),
                Span::from("one "),
                Span::from("e").bold(),
                Span::from("dit "),
                Span::from("f").bold(),
                Span::from("ocus "),
                Span::from("m").bold(),
                Span::from("ore time "),
                Span::from("r").bold(),
                Span::from("eroll "),
                Span::from("q").bold(),
                Span::from("uit "),
                Span::from("w").bold(),
                Span::from("ait 1h "),
                // TODO: these could be sourced from config
                Span::from("o").bold(),
                Span::from("pen "),
                Span::from("b").bold(),
                Span::from("reakdown"),
            ])
            .centered()
            .style(gauge_style(false).reversed()),
            status_line_area,
        );
    }

    pub async fn handle_input(&mut self, event: Event) -> Result<()> {
        if let Event::Key(key_event) = event {
            match key_event.code {
                KeyCode::Char('q') => {
                    self.should_quit = true;
                }
                KeyCode::Char('d') => {
                    self.interactive = self.doing.mark_done_command(&self.tw);

                    // TODO: possible race condition here. It's possible to
                    // choose the same task again. Should interactive maybe
                    // take some kind of callback so that this can't happen?
                    self.doing = self.choose_next_task().await?;
                }
                KeyCode::Char('r') => {
                    self.doing = self.choose_next_task().await?;
                }
                KeyCode::Char('m') => {
                    self.doing.extend();
                }
                KeyCode::Char('e') => {
                    if let Activity::Task { task, .. } = &self.doing {
                        let mut command = Command::new(&self.tw.binary);
                        command.arg(&task.uuid);
                        command.arg("edit");

                        self.interactive = Some(command)
                    };
                }
                KeyCode::Char('w') => {
                    if let Activity::Task { task, .. } = &self.doing {
                        self.interactive = Some(
                            self.tw
                                .modify()
                                .with_subject(&task.uuid)
                                .with_mod("wait:1h")
                                .command(),
                        );

                        // TODO: possible race condition here. It's possible to
                        // choose the same task again. Should interactive maybe
                        // take some kind of callback so that this can't happen?
                        self.doing = self.choose_next_task().await?;
                    };
                }

                // TODO: source these from config
                KeyCode::Char('o') => {
                    if let Activity::Task { task, .. } = &self.doing {
                        let mut command = Command::new("tw-open");
                        command.arg(&task.uuid);

                        self.interactive = Some(command)
                    };
                }
                KeyCode::Char('b') => {
                    if let Activity::Task { task, .. } = &self.doing {
                        let mut command = Command::new("tw-breakdown");
                        command.arg("--seq");
                        command.arg(&task.uuid);

                        self.interactive = Some(command)
                    };
                }
                KeyCode::Char('f') => {
                    if let Activity::Task {
                        task,
                        started,
                        length,
                        ..
                    } = &self.doing
                    {
                        let remaining_seconds = (*length - (Utc::now() - started)).num_seconds();

                        if remaining_seconds > 0 {
                            open::that(format!(
                                "raycast://focus/start?goal={}&categories=messaging,social,news&duration={}",
                                urlencoding::encode(&task.description),
                                remaining_seconds,
                            ))
                            .context("could not start focus session")?;
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub async fn handle_tick(&mut self) -> Result<()> {
        if self.doing.is_nothing() {
            self.doing = self
                .choose_next_task()
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

    async fn choose_next_task(&self) -> Result<Activity> {
        let now = Utc::now();

        // This is inspired by the Gladden Design Paper Apps TO•DO, where you
        // roll a d6 to decide how long you're going to work. You take a break
        // if you roll a 6, and work for `roll*10` minutes otherwise. We use `0`
        // as our sentinel value instead.
        let minutes = rand::random_range(0..=5);

        if minutes == 0 && !self.doing.is_break() {
            let length = Duration::minutes(10);

            Ok(Activity::Break {
                started: now,
                length,
                original_length: length,
            })
        } else {
            let target_duration = Duration::minutes(minutes.max(1) * 10);

            let tasks = self.available_tasks().await?;

            let task = tasks
                .choose_weighted(&mut rand::rng(), |task| task.urgency_at(now, &self.config))
                .context("could not choose a task")?;

            let length = task
                .estimate
                .unwrap_or(target_duration)
                .min(target_duration);

            Ok(Activity::Task {
                task: task.clone(),
                started: now,
                length,
                original_length: length,
            })
        }
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn take_interactive(&mut self) -> Option<Command> {
        self.interactive.take()
    }

    pub async fn refresh_doing(&mut self) -> Result<()> {
        self.doing.refresh_task(&self.tw).await
    }
}

#[expect(
    clippy::large_enum_variant,
    reason = "only one is used at a time; it does not dominate memory usage"
)]
#[derive(Debug)]
pub enum Activity {
    Nothing,
    Task {
        task: Task,
        started: DateTime<Utc>,
        length: Duration,
        original_length: Duration,
    },
    Break {
        started: DateTime<Utc>,
        length: Duration,
        original_length: Duration,
    },
}

impl Activity {
    pub fn is_nothing(&self) -> bool {
        matches!(self, Self::Nothing)
    }

    pub fn is_break(&self) -> bool {
        matches!(self, Self::Break { .. })
    }

    pub fn mark_done_command(&self, tw: &Taskwarrior) -> Option<Command> {
        if let Self::Task { task, .. } = self {
            Some(tw.mark_done_command(&task.uuid))
        } else {
            None
        }
    }

    pub fn extend(&mut self) {
        match self {
            Self::Task {
                length,
                original_length,
                ..
            }
            | Self::Break {
                length,
                original_length,
                ..
            } => {
                *length += *original_length;
            }
            Self::Nothing => {}
        }
    }

    pub async fn refresh_task(&mut self, tw: &Taskwarrior) -> Result<()> {
        if let Self::Task { task, .. } = self {
            *task = tw
                .export()
                .with_filter(&task.uuid)
                .with_filter("limit:1")
                .call()
                .await
                .context("could not refresh task")?
                .pop()
                .context("could not find task")?;
        }

        Ok(())
    }
}

fn gauge_style(completed_time: bool) -> Style {
    if completed_time {
        Style::new()
            .fg(tailwind::GREEN.c800)
            .bg(tailwind::GREEN.c400)
    } else {
        Style::new().fg(tailwind::BLUE.c800).bg(tailwind::BLUE.c400)
    }
}

fn format_remaining(remaining: Duration) -> String {
    format!(
        "{}{}:{:02}",
        if remaining < Duration::zero() {
            "-"
        } else {
            ""
        },
        remaining.abs().num_minutes(),
        remaining.abs().num_seconds() % 60
    )
}
