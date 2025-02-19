use crate::config::Config;
use crate::task::Task;
use crate::taskwarrior::Taskwarrior;
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Local, Utc};
use crossterm::event::{Event, KeyCode};
use itertools::Itertools;
use rand::prelude::*;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Flex, Layout},
    style::{palette::tailwind, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Gauge, Paragraph, Wrap},
    Frame,
};

#[derive(Debug)]
pub struct App {
    tw: Taskwarrior,
    config: Config,

    /// This is the thing we're doing *right now*
    doing: Activity,

    /// The main loop uses this as a signal that it should exit.
    should_quit: bool,
}

impl App {
    pub fn new(tw: Taskwarrior, config: Config) -> Self {
        Self {
            tw,
            config,

            doing: Activity::Nothing,

            should_quit: false,
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let [app_area, status_line_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());

        let app_box_vert = Layout::vertical([Constraint::Length(7)]).flex(Flex::Center);
        let app_box_horiz = Layout::horizontal([Constraint::Percentage(50)]).flex(Flex::Center);

        let [app_box_area] = app_box_vert.areas(app_area);
        let [app_box_area] = app_box_horiz.areas(app_box_area);

        let gauge_style = Style::new().fg(tailwind::BLUE.c800).bg(tailwind::BLUE.c400);

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
            } => {
                let time_remaining = *length - (Utc::now() - started);

                let percent_remaining =
                    time_remaining.num_seconds() as f64 / length.num_seconds() as f64;

                let mut sections = vec![
                    Span::styled(format!("{}", task.id), Style::default().bold()),
                    Span::from(": "),
                    Span::from(&task.description),
                ];

                if !task.tags.is_empty() {
                    sections.push(Span::from(" "));
                    sections.push(Span::styled(
                        task.tags.iter().map(|tag| format!("+{}", tag)).join(" "),
                        Style::default().bold(),
                    ));
                }

                if let Some(project) = &task.project {
                    sections.push(Span::from(" "));
                    sections.push(Span::styled("pro:", Style::default().bold()));
                    sections.push(Span::from(project));
                }

                sections.push(Span::from(" "));
                sections.push(Span::styled(
                    format!(
                        "until {}",
                        (*started + *length)
                            .with_timezone(&Local)
                            .format("%-I:%M %P")
                    ),
                    Style::default().italic().dim(),
                ));

                (
                    Paragraph::new(Line::from(sections))
                        .centered()
                        .wrap(Wrap { trim: false }),
                    Gauge::default()
                        .label(format!(
                            "{}:{:02}",
                            time_remaining.num_minutes(),
                            time_remaining.num_seconds() % 60
                        ))
                        .gauge_style(gauge_style)
                        .ratio(percent_remaining)
                        .use_unicode(true),
                )
            }
            Activity::Break { started, length } => {
                let time_remaining = *length - (Utc::now() - started);

                let percent_remaining =
                    time_remaining.num_seconds() as f64 / length.num_seconds() as f64;

                (
                    Paragraph::new(format!(
                        "Taking a break until {}",
                        (*started + *length)
                            .with_timezone(&Local)
                            .format("%-I:%M %p")
                    ))
                    .centered(),
                    Gauge::default()
                        .label(format!(
                            "{}:{:02}",
                            time_remaining.num_minutes(),
                            time_remaining.num_seconds() % 60
                        ))
                        .gauge_style(gauge_style)
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
                Span::styled("<d>", Style::default().bold()),
                Span::from("one "),
                Span::styled("<m>", Style::default().bold()),
                Span::from("ore time "),
                Span::styled("<r>", Style::default().bold()),
                Span::from("eroll "),
                Span::styled("<q>", Style::default().bold()),
                Span::from("uit"),
            ])
            .alignment(Alignment::Center)
            .style(gauge_style.reversed()),
            status_line_area,
        );
    }

    pub async fn handle_input(&mut self, event: Event) -> Result<()> {
        if let Event::Key(key_event) = event {
            match key_event.code {
                KeyCode::Char('q') => {
                    self.should_quit = true;
                }
                KeyCode::Char('c') => {
                    self.doing
                        .mark_done(&self.tw)
                        .await
                        .context("could not mark task done")?;

                    self.doing = self.choose_next_task().await?;
                }
                KeyCode::Char('r') => {
                    self.doing = self.choose_next_task().await?;
                }
                KeyCode::Char('m') => {
                    self.doing.extend();
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
            Ok(Activity::Break {
                started: now,
                length: Duration::minutes(10),
            })
        } else {
            let target_duration = Duration::minutes(minutes.max(1) * 10);

            let tasks = self.available_tasks().await?;

            let task = tasks
                .choose_weighted(&mut rand::rng(), |task| task.urgency_at(now, &self.config))
                .context("could not choose a task")?;

            Ok(Activity::Task {
                task: task.clone(),
                started: now,
                length: task
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
    Task {
        task: Task,
        started: DateTime<Utc>,
        length: Duration,
    },
    Break {
        started: DateTime<Utc>,
        length: Duration,
    },
}

impl Activity {
    pub fn is_nothing(&self) -> bool {
        matches!(self, Self::Nothing)
    }

    pub fn is_break(&self) -> bool {
        matches!(self, Self::Break { .. })
    }

    pub async fn mark_done(&self, tw: &Taskwarrior) -> Result<()> {
        if let Self::Task { task, .. } = self {
            tw.mark_done(&task.uuid).await?;
        }

        Ok(())
    }

    pub fn extend(&mut self) {
        let extension = Duration::minutes(5 * rand::random_range(1..=5));

        match self {
            Self::Task { length, .. } => {
                *length += extension;
            }
            Self::Break { length, .. } => {
                *length += extension;
            }
            Self::Nothing => {}
        }
    }
}
