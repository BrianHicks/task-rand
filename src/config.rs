use anyhow::{anyhow, Context, Result};

#[derive(Debug, PartialEq)]
pub struct Config {
    pub urgency_due_coefficient: f64,
    pub urgency_age_coefficient: f64,
    pub urgency_age_max: f64,
}

impl Config {
    pub fn new() -> Self {
        Self {
            urgency_age_coefficient: 1.0,
            urgency_age_max: 365.0,
            urgency_due_coefficient: 1.0,
        }
    }

    pub fn parse(input: &str) -> Result<Self> {
        let mut out = Config::default();

        for line in input.split('\n') {
            if !line.contains('=') {
                continue;
            }

            let mut kv = line.splitn(2, '=');
            let k = kv.next().ok_or(anyhow!("Expected a key"))?;
            let v = kv.next().ok_or(anyhow!("Expected a value"))?;

            match k {
                "urgency.age.coefficient" => {
                    out.urgency_age_coefficient =
                        v.parse().context("could not parse age coefficient")?
                }
                "urgency.age.max" => {
                    out.urgency_age_max = v.parse().context("could not parse age max")?
                }
                "urgency.due.coefficient" => {
                    out.urgency_due_coefficient =
                        v.parse().context("could not parse due coefficient")?
                }

                _ => continue,
            }
        }

        Ok(out)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_empty_string() {
        assert_eq!(Config::parse("").unwrap(), Config::default())
    }

    #[test]
    fn parse_urgency_age_coefficient() {
        let config = Config::parse("urgency.age.coefficient=2.5").unwrap();

        assert_eq!(config.urgency_age_coefficient, 2.5);
    }

    #[test]
    fn parse_urgency_max_age() {
        let config = Config::parse("urgency.age.max=123.4").unwrap();

        assert_eq!(config.urgency_age_max, 123.4);
    }

    #[test]
    fn parse_urgency_due_coefficient() {
        let config = Config::parse("urgency.due.coefficient=2.5").unwrap();

        assert_eq!(config.urgency_due_coefficient, 2.5);
    }
}
