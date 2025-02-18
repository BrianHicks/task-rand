use crate::{config::Config, task::Task};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::process::Command;

#[derive(Debug)]
pub struct Taskwarrior {
    binary: PathBuf,
}

impl Taskwarrior {
    pub fn new(binary: PathBuf) -> Self {
        Self { binary }
    }

    #[tracing::instrument]
    pub fn export(&self) -> ExportBuilder {
        ExportBuilder {
            binary: self.binary.clone(),
            filters: Vec::new(),
            urgency_coefficients: HashMap::new(),
        }
    }

    #[tracing::instrument]
    pub async fn config(&self) -> Result<Config> {
        let mut command = Command::new(&self.binary);
        command.arg("_show");

        tracing::trace!(?command, "getting config from taskwarrior");

        let output = command
            .output()
            .await
            .context("could not call Taskwarrior")?;

        let config_text =
            String::from_utf8(output.stdout).context("config contained invalid UTF-8")?;

        Config::parse(&config_text).context("could not parse config")
    }
}

pub struct ExportBuilder {
    binary: PathBuf,
    filters: Vec<String>,
    urgency_coefficients: HashMap<String, f64>,
}

impl ExportBuilder {
    pub fn with_urgency_coefficient(mut self, key: &str, value: f64) -> Self {
        self.urgency_coefficients.insert(key.to_owned(), value);

        self
    }

    pub fn with_filter(mut self, filter: &str) -> Self {
        self.filters.push(filter.to_owned());

        self
    }

    #[tracing::instrument("export", skip(self))]
    pub async fn call(self) -> Result<Vec<Task>> {
        let mut command = Command::new(self.binary);

        for (key, coefficient) in self.urgency_coefficients {
            command.arg(format!("rc.urgency.{}.coefficient={}", key, coefficient));
        }

        command.args(self.filters);

        command.arg("export");

        tracing::trace!(?command, "calling taskwarrior for export");

        let output = command.output().await.context("could not retrieve tasks")?;

        serde_json::from_slice(&output.stdout).context("could not deserialize tasks")
    }
}
