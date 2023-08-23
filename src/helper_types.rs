use std::borrow::Cow;
use std::cmp::Ordering;
use std::str::FromStr;

use lazy_static::lazy_static;
use regex::Regex;
use serde::de::{DeserializeOwned, Error};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::util::Near;

#[derive(Debug)]
pub struct Rate<T> {
    pub value: T,
    pub measurement: String,
    pub time: TimeDimension
}

impl<T> PartialEq for Rate<T>
where
    T: PartialEq
{
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time && self.value == other.value
    }
}

impl<T> Eq for Rate<T> where T: PartialEq {}

impl<T> PartialOrd<Self> for Rate<T>
where
    T: PartialEq
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Rate<T>
where
    T: PartialEq
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.time.cmp(&other.time)
    }
}

impl<T> Serialize for Rate<T>
where
    T: Serialize
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        (&self.value, &self.measurement, &self.time).serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for Rate<T>
where
    T: Deserialize<'de>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        let (value, measurement, time) = <(T, String, TimeDimension)>::deserialize(deserializer)?;
        Ok(Rate {
            value,
            measurement,
            time
        })
    }
}

lazy_static! {
    static ref UNIT_RE: Regex =
        Regex::new(r"^(?<measurement>[^/]+)/(?<factor>[\d\.,]*)(?<time>\w+)$")
            .expect("valid regex");
}

// TODO: make this more generic
impl FromStr for Rate<f64> {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.splitn(2, ' ');
        let value = split.next().expect("split never empty");
        let unit =
            split.next().ok_or_else(|| anyhow::Error::msg(format!("rate has no unit: {s}")))?;

        let value: f64 = value.parse()?;

        let unit_capture = UNIT_RE.captures(unit).ok_or(anyhow::Error::msg(format!(
            "unit {unit:?} has invalid format"
        )))?;
        let measurement = unit_capture["measurement"].to_string();
        let factor: f64 = unit_capture["factor"].parse().unwrap_or(1f64);
        let time = match &unit_capture["time"] {
            "s" => TimeDimension::Seconds(factor),
            "m" | "min" => TimeDimension::Minutes(factor),
            "h" => TimeDimension::Hours(factor),
            "d" => TimeDimension::Days(factor),
            "w" | "wo" => TimeDimension::Weeks(factor),
            "M" | "mo" => TimeDimension::Months(factor),
            "a" | "y" => TimeDimension::Years(factor),
            unit => {
                return Err(anyhow::Error::msg(format!(
                    "{unit} is a unknown time dimension"
                )))
            }
        };

        Ok(Rate {
            value,
            measurement,
            time
        })
    }
}

#[derive(Debug)]
pub enum TimeDimension {
    Seconds(f64),
    Minutes(f64),
    Hours(f64),
    Days(f64),
    Weeks(f64),
    Months(f64),
    Years(f64)
}

impl TimeDimension {
    /// Rough conversion to seconds.
    ///
    /// Imprecise for dimensions larger than weeks.
    pub fn as_secs(&self) -> f64 {
        use TimeDimension::*;

        match self {
            Seconds(s) => *s,
            Minutes(m) => *m * 60.0,
            Hours(h) => *h * 60.0 * 60.0,
            Days(d) => *d * 24.0 * 60.0 * 60.0,
            Weeks(w) => *w * 7.0 * 24.0 * 60.0 * 60.0,
            Months(m) => *m * 30.0 * 24.0 * 60.0 * 60.0,
            Years(y) => *y * 365.0 * 24.0 * 60.0 * 60.0
        }
    }
}

impl Serialize for TimeDimension {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        let s: Cow<'_, str> = match self {
            TimeDimension::Seconds(v) if v.is_near(&1.0) => "s".into(),
            TimeDimension::Seconds(v) => format!("{v}s").into(),

            TimeDimension::Minutes(v) if v.is_near(&1.0) => "m".into(),
            TimeDimension::Minutes(v) => format!("{v}m").into(),

            TimeDimension::Hours(v) if v.is_near(&1.0) => "h".into(),
            TimeDimension::Hours(v) => format!("{v}h").into(),

            TimeDimension::Days(v) if v.is_near(&1.0) => "d".into(),
            TimeDimension::Days(v) => format!("{v}d").into(),

            TimeDimension::Weeks(v) if v.is_near(&1.0) => "w".into(),
            TimeDimension::Weeks(v) => format!("{v}wo").into(),

            TimeDimension::Months(v) if v.is_near(&1.0) => "mo".into(),
            TimeDimension::Months(v) => format!("{v}mo").into(),

            TimeDimension::Years(v) if v.is_near(&1.0) => "a".into(),
            TimeDimension::Years(v) => format!("{v}a").into()
        };

        s.serialize(serializer)
    }
}

lazy_static! {
    static ref TIME_RE: Regex =
        Regex::new(r"^(?<value>\d*)(?<duration>\w+)$").expect("valid regex");
}

impl<'de> Deserialize<'de> for TimeDimension {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        let captured = TIME_RE.captures(s.as_str()).ok_or(D::Error::custom(format!(
            "time duration has invalid format: {s}"
        )))?;

        let value = &captured["value"];
        let value = match value.is_empty() {
            true => 1f64,
            false => value.parse().expect("only digits in here")
        };

        let duration = &captured["duration"];
        Ok(match duration {
            "s" => TimeDimension::Seconds(value),
            "m" => TimeDimension::Minutes(value),
            "h" => TimeDimension::Hours(value),
            "d" => TimeDimension::Days(value),
            "w" => TimeDimension::Weeks(value),
            "M" => TimeDimension::Months(value),
            "a" => TimeDimension::Years(value),
            d => return Err(D::Error::custom(format!("unknown date duration: {d}")))
        })
    }
}

impl PartialEq for TimeDimension {
    fn eq(&self, other: &Self) -> bool {
        self.as_secs() == other.as_secs()
    }
}

impl Eq for TimeDimension {}

impl PartialOrd for TimeDimension {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TimeDimension {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_secs().partial_cmp(&other.as_secs()).expect("should never be NaN")
    }
}

/// A number that has a unit.
#[derive(Debug, Deserialize)]
pub struct Quantity {
    pub value: f64,
    pub unit: String
}

impl Serialize for Quantity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        (&self.value, &self.unit).serialize(serializer)
    }
}

impl From<(f64, String)> for Quantity {
    fn from((value, unit): (f64, String)) -> Self {
        Quantity { value, unit }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum SingleOrPair<P0, P1 = P0, S = P0> {
    Single(S),
    Pair(P0, P1)
}

impl<P0, P1, S> Serialize for SingleOrPair<P0, P1, S>
where
    P0: Serialize,
    P1: Serialize,
    S: Serialize
{
    fn serialize<SE>(&self, serializer: SE) -> Result<SE::Ok, SE::Error>
    where
        SE: Serializer
    {
        use SingleOrPair::*;

        match self {
            Single(v) => [v].serialize(serializer),
            Pair(a, b) => (a, b).serialize(serializer)
        }
    }
}

impl<'de, P0, P1, S> Deserialize<'de> for SingleOrPair<P0, P1, S>
where
    S: DeserializeOwned,
    P0: DeserializeOwned,
    P1: DeserializeOwned
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        let items: Vec<serde_json::Value> = Vec::deserialize(deserializer)?;
        let mut items = items.into_iter();
        match (items.next(), items.next(), items.next()) {
            (Some(s), None, None) => Ok(SingleOrPair::Single(
                serde_json::from_value(s).map_err(D::Error::custom)?
            )),
            (Some(p0), Some(p1), None) => Ok(SingleOrPair::Pair(
                serde_json::from_value(p0).map_err(D::Error::custom)?,
                serde_json::from_value(p1).map_err(D::Error::custom)?
            )),
            _ => Err(D::Error::custom("must be either a single value or a pair"))
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OrFallback<T> {
    Expected(T),
    Fallback(String)
}

impl<T> From<T> for OrFallback<T> {
    fn from(value: T) -> Self {
        OrFallback::Expected(value)
    }
}

impl<T> Serialize for OrFallback<T>
where
    T: Serialize
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        match self {
            OrFallback::Expected(expected) => expected.serialize(serializer),
            OrFallback::Fallback(fallback) => fallback.serialize(serializer)
        }
    }
}

impl<'de, T> Deserialize<'de> for OrFallback<T>
where
    T: DeserializeOwned
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        let fallback: String = String::deserialize(deserializer)?;
        match serde_json::from_value::<T>(Value::String(fallback.clone())) {
            Ok(value) => Ok(OrFallback::Expected(value)),
            Err(_) => Ok(OrFallback::Fallback(fallback))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SINGLE_DE: SingleOrPair<u32> = SingleOrPair::Single(69);
    const PAIR_DE: SingleOrPair<u32> = SingleOrPair::Pair(69, 420);

    const SINGLE_SER: &str = "[69]";
    const PAIR_SER: &str = "[69,420]";

    type T = SingleOrPair<u32>;

    #[test]
    fn serde_optional_pair() {
        assert_eq!(serde_json::to_string(&SINGLE_DE).unwrap(), SINGLE_SER);
        assert_eq!(serde_json::to_string(&PAIR_DE).unwrap(), PAIR_SER);

        assert_eq!(serde_json::from_str::<T>(SINGLE_SER).unwrap(), SINGLE_DE);
        assert_eq!(serde_json::from_str::<T>(PAIR_SER).unwrap(), PAIR_DE);
    }
}
