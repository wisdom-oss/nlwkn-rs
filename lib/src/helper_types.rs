use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::io::stderr;
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
    pub time: Duration
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
        let (value, measurement, time) = <(T, String, Duration)>::deserialize(deserializer)?;
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
            "s" => Duration::Seconds(factor),
            "m" | "min" => Duration::Minutes(factor),
            "h" => Duration::Hours(factor),
            "d" => Duration::Days(factor),
            "w" | "wo" => Duration::Weeks(factor),
            "M" | "mo" => Duration::Months(factor),
            "a" | "y" => Duration::Years(factor),
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
pub enum Duration {
    Seconds(f64),
    Minutes(f64),
    Hours(f64),
    Days(f64),
    Weeks(f64),
    Months(f64),
    Years(f64)
}

impl Duration {
    /// Rough conversion to seconds.
    ///
    /// Imprecise for dimensions larger than weeks.
    pub fn as_secs(&self) -> f64 {
        use Duration::*;

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

impl Serialize for Duration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        let s: Cow<'_, str> = match self {
            Duration::Seconds(v) if v.is_near(&1.0) => "s".into(),
            Duration::Seconds(v) => format!("{v}s").into(),

            Duration::Minutes(v) if v.is_near(&1.0) => "m".into(),
            Duration::Minutes(v) => format!("{v}m").into(),

            Duration::Hours(v) if v.is_near(&1.0) => "h".into(),
            Duration::Hours(v) => format!("{v}h").into(),

            Duration::Days(v) if v.is_near(&1.0) => "d".into(),
            Duration::Days(v) => format!("{v}d").into(),

            Duration::Weeks(v) if v.is_near(&1.0) => "w".into(),
            Duration::Weeks(v) => format!("{v}wo").into(),

            Duration::Months(v) if v.is_near(&1.0) => "mo".into(),
            Duration::Months(v) => format!("{v}mo").into(),

            Duration::Years(v) if v.is_near(&1.0) => "a".into(),
            Duration::Years(v) => format!("{v}a").into()
        };

        s.serialize(serializer)
    }
}

impl Display for Duration {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.serialize(f)
    }
}

lazy_static! {
    static ref TIME_RE: Regex =
        Regex::new(r"^(?<value>\d*)(?<duration>\w+)$").expect("valid regex");
}

impl<'de> Deserialize<'de> for Duration {
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
            "s" => Duration::Seconds(value),
            "m" | "min" => Duration::Minutes(value),
            "h" => Duration::Hours(value),
            "d" => Duration::Days(value),
            "w" | "wo" => Duration::Weeks(value),
            "M" | "mo" => Duration::Months(value),
            "a" | "y" => Duration::Years(value),
            d => return Err(D::Error::custom(format!("unknown date duration: {d}")))
        })
    }
}

impl PartialEq for Duration {
    fn eq(&self, other: &Self) -> bool {
        self.as_secs() == other.as_secs()
    }
}

impl Eq for Duration {}

impl PartialOrd for Duration {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Duration {
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

impl Display for Quantity {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.value, self.unit)
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

impl<P0, P1, S> Display for SingleOrPair<P0, P1, S>
where
    P0: Display,
    P1: Display,
    S: Display
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SingleOrPair::Single(s) => write!(f, "{s}"),
            SingleOrPair::Pair(p0, p1) => write!(f, "{p0} {p1}")
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
        let any = Value::deserialize(deserializer)?;
        match serde_json::from_value::<T>(any.clone()) {
            Ok(value) => Ok(OrFallback::Expected(value)),
            Err(e) => match any {
                Value::String(s) => Ok(OrFallback::Fallback(s)),
                Value::Null => Err(D::Error::custom("expected string, got null")),
                Value::Bool(b) => Err(D::Error::custom(format!("expected string, got {b}"))),
                Value::Number(n) => Err(D::Error::custom(format!("expected string, got {n}"))),
                Value::Array(_) => Err(D::Error::custom("expected string, got an array")),
                Value::Object(_) => Err(D::Error::custom("expected string, got an object"))
            }
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
