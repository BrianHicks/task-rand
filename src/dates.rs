use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use serde::de::{self, Visitor};
use std::fmt;

struct TWDateTimeVisitor;

impl Visitor<'_> for TWDateTimeVisitor {
    type Value = DateTime<Utc>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string in the format YYYYMMDDTHHMMSSZ")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let naive =
            NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%SZ").map_err(de::Error::custom)?;

        Ok(naive.and_utc())
    }
}

pub fn tw_datetime<'de, D>(d: D) -> Result<DateTime<Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    d.deserialize_str(TWDateTimeVisitor)
}

struct OptionalTWDateTimeVisitor;

impl<'de> Visitor<'de> for OptionalTWDateTimeVisitor {
    type Value = Option<DateTime<Utc>>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string in the format YYYYMMDDTHHMMSSZ or null")
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Delegate to the existing datetime deserialization logic
        tw_datetime(deserializer).map(Some)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }
}

pub fn tw_datetime_opt<'de, D>(d: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    d.deserialize_option(OptionalTWDateTimeVisitor)
}

struct DurationVisitor;

impl Visitor<'_> for DurationVisitor {
    type Value = Option<Duration>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string in ISO8601 duration format or null")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let parsed = iso8601_duration::Duration::parse(value)
            .map_err(|error| de::Error::custom(format!("{error:?}")))?;

        Ok(parsed.to_chrono())
    }
}

pub fn duration<'de, D>(d: D) -> Result<Option<Duration>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    d.deserialize_str(DurationVisitor)
}
