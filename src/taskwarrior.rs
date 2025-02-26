use crate::{config::Config, task::Task};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::process::Command;

#[derive(Debug)]
pub struct Taskwarrior {
    pub binary: PathBuf,
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

    pub fn mark_done_command(&self, id: &str) -> Command {
        let mut command = Command::new(&self.binary);
        command.args([id, "done"]);

        command
    }

    #[tracing::instrument]
    pub async fn mark_done(&self, id: &str) -> Result<()> {
        let mut command = self.mark_done_command(id);

        tracing::trace!(?command, "marking task as done");

        let out = command
            .output()
            .await
            .context("could not mark task as done")?;

        if !out.status.success() {
            return Err(anyhow::anyhow!(
                "could not mark task as done. Exit code {:?}",
                out.status
            ));
        }

        Ok(())
    }

    #[tracing::instrument]
    pub fn modify(&self) -> ModifyBuilder {
        ModifyBuilder {
            binary: self.binary.clone(),
            subjects: Vec::new(),
            mods: Vec::new(),
        }
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

pub struct ModifyBuilder {
    binary: PathBuf,
    subjects: Vec<String>,
    mods: Vec<String>,
}

impl ModifyBuilder {
    pub fn with_subject(mut self, subject: &str) -> Self {
        self.subjects.push(subject.to_owned());

        self
    }

    pub fn with_mod(mut self, mod_: &str) -> Self {
        self.mods.push(mod_.to_owned());

        self
    }

    pub fn command(self) -> Command {
        let mut command = Command::new(self.binary);

        command.args(self.subjects);
        command.arg("modify");
        command.args(self.mods);

        command
    }

    #[tracing::instrument("modify", skip(self))]
    pub async fn call(self) -> Result<()> {
        let mut command = self.command();

        tracing::trace!(?command, "calling taskwarrior for modify");

        let out = command.output().await.context("could not modify tasks")?;

        if !out.status.success() {
            return Err(anyhow::anyhow!(
                "could not modify task. Exit code {:?}",
                out.status
            ));
        }

        Ok(())
    }
}
