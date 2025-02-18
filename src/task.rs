use crate::config::Config;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashSet;

#[derive(Debug, serde::Deserialize)]
pub struct Task {
    pub uuid: String,

    pub description: String,

    pub urgency: f64,

    pub status: Status,

    #[serde(deserialize_with = "crate::dates::tw_datetime")]
    pub entry: DateTime<Utc>,

    #[serde(default, deserialize_with = "crate::dates::tw_datetime_opt")]
    pub wait: Option<DateTime<Utc>>,

    #[serde(default, deserialize_with = "crate::dates::tw_datetime_opt")]
    pub due: Option<DateTime<Utc>>,

    #[serde(default, deserialize_with = "crate::dates::tw_datetime_opt")]
    pub target: Option<DateTime<Utc>>,

    #[serde(default, deserialize_with = "crate::dates::duration")]
    pub estimate: Option<Duration>,

    #[serde(default)]
    pub depends: HashSet<String>,

    #[serde(default)]
    pub tags: HashSet<String>,
}

impl Task {
    pub fn available_at(&self, when: DateTime<Utc>) -> bool {
        match self.wait {
            None => true,
            Some(wait) => wait <= when,
        }
    }

    pub fn urgency_at(&self, when: DateTime<Utc>, config: &Config) -> f64 {
        self.urgency
            + self.base_due_urgency_at(when) * config.urgency_due_coefficient
            + self.base_age_urgency_at(when, config) * config.urgency_age_coefficient
    }

    fn base_due_urgency_at(&self, when: DateTime<Utc>) -> f64 {
        // We build on the base Taskwarrior due date calculation by adding a
        // couple of dates (listed above the cases.)
        let target = match (self.due, self.target) {
            // If we only have a due date, use it.
            (Some(due), None) => due,

            // If we only have a target date, use it.
            (None, Some(target)) => target,

            // If we have both a due and target date, use whichever is soonest.
            // If we have schedule pressure, our urgency needs to reflect the
            // maximum pressure.
            (Some(due), Some(target)) => due.min(target),

            // Tasks that have either due or target dates will dominate the
            // urgency calculation. To balance that out, we give tasks with
            // neither a due date nor a target a fake target that's perpetually
            // some time into the future, with urgency increasing the longer ago
            // it was added.
            (None, None) => (self.entry + Duration::weeks(4)).max(when + Duration::weeks(1)),
        };

        // We're OK with the conversion being naive here. We're pretty
        // unlikely to enounter high enough numbers that we couldn't
        // convert with `as`.
        let days_overdue = (when - target).num_seconds() as f64 / 86_400.0;

        if days_overdue >= 7.0 {
            1.0
        } else if days_overdue >= -14.0 {
            ((days_overdue + 14.0) * 0.8 / 21.0) + 0.2
        } else {
            0.2
        }
    }

    fn base_age_urgency_at(&self, when: DateTime<Utc>, config: &Config) -> f64 {
        // We're OK with the conversion being naive here. We're pretty unlikely
        // to enounter high enough numbers that we couldn't convert with `as`.
        let age = (when - self.entry).num_seconds() as f64 / 86_400.0;

        if config.urgency_age_max == 0.0 || age > config.urgency_age_max {
            1.0
        } else {
            age / config.urgency_age_max
        }
    }
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Pending,
    Completed,
    Deleted,
    Recurring,
}
