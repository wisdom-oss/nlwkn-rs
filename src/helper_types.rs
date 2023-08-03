use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter, Write};
use std::str::FromStr;

use lazy_static::lazy_static;
use regex::Regex;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{DeserializeAs, OneOrMany, Same};

use crate::util::data_structs;

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
        Regex::new(r"^(?<measurement>[^/]+)/(?<factor>\d*)(?<time>\w+)$").expect("valid regex");
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
        let factor: u64 = unit_capture["factor"].parse().unwrap_or(1);
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

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum TimeDimension {
    Seconds(u64),
    Minutes(u64),
    Hours(u64),
    Days(u64),
    Weeks(u64),
    Months(u64),
    Years(u64)
}

impl TimeDimension {
    /// Rough conversion to seconds.
    ///
    /// Imprecise for dimensions larger than weeks.
    pub fn as_secs(&self) -> u64 {
        use TimeDimension::*;

        match self {
            Seconds(s) => *s,
            Minutes(m) => *m * 60,
            Hours(h) => *h * 60 * 60,
            Days(d) => *d * 24 * 60 * 60,
            Weeks(w) => *w * 7 * 24 * 60 * 60,
            Months(m) => *m * 30 * 24 * 60 * 60,
            Years(y) => *y * 365 * 24 * 60 * 60
        }
    }
}

impl Serialize for TimeDimension {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        let s: Cow<'_, str> = match self {
            TimeDimension::Seconds(1) => "s".into(),
            TimeDimension::Seconds(v) => format!("{v}s").into(),

            TimeDimension::Minutes(1) => "m".into(),
            TimeDimension::Minutes(v) => format!("{v}m").into(),

            TimeDimension::Hours(1) => "h".into(),
            TimeDimension::Hours(v) => format!("{v}h").into(),

            TimeDimension::Days(1) => "d".into(),
            TimeDimension::Days(v) => format!("{v}d").into(),

            TimeDimension::Weeks(1) => "w".into(),
            TimeDimension::Weeks(v) => format!("{v}wo").into(),

            TimeDimension::Months(1) => "mo".into(),
            TimeDimension::Months(v) => format!("{v}mo").into(),

            TimeDimension::Years(1) => "a".into(),
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
            true => 1,
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

impl PartialOrd for TimeDimension {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TimeDimension {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_secs().cmp(&other.as_secs())
    }
}

data_structs! {
    /// A number that has a unit.
    struct DimensionedNumber {
        value: f64,
        unit: String,
    }

    /// A number that has a unit and a description.
    struct DescriptiveNumber {
        value: f64,
        unit: String,
        description?: String,
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum OptionalPair<T> {
    Single(T),
    Pair(T, T)
}

impl<T> Serialize for OptionalPair<T>
where
    T: Serialize
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        use OptionalPair::*;

        match self {
            Single(v) => v.serialize(serializer),
            Pair(a, b) => (a, b).serialize(serializer)
        }
    }
}

impl<'de, T> Deserialize<'de> for OptionalPair<T>
where
    T: Deserialize<'de>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        let items: Vec<T> = OneOrMany::<Same>::deserialize_as(deserializer)?;
        let mut items = items.into_iter();
        let first = items.next();
        let second = items.next();
        let third = items.next();

        match (first, second, third) {
            (None, _, _) => Err(serde::de::Error::custom("pair must not be empty")),
            (_, _, Some(_)) => Err(serde::de::Error::custom("pairs mut not exceed 2 elements")),
            (Some(v), None, _) => Ok(OptionalPair::Single(v)),
            (Some(a), Some(b), _) => Ok(OptionalPair::Pair(a, b))
        }
    }
}

#[cfg(test)]
mod tests {
    use const_format::formatcp;

    use crate::helper_types::{OptionalPair, TimeDimension};

    const TIME_DIMENSION_MULTI_VALUE: u64 = 69;
    const TIME_DIMENSION_DE: [TimeDimension; 14] = [
        TimeDimension::Seconds(1),
        TimeDimension::Seconds(TIME_DIMENSION_MULTI_VALUE),
        TimeDimension::Minutes(1),
        TimeDimension::Minutes(TIME_DIMENSION_MULTI_VALUE),
        TimeDimension::Hours(1),
        TimeDimension::Hours(TIME_DIMENSION_MULTI_VALUE),
        TimeDimension::Days(1),
        TimeDimension::Days(TIME_DIMENSION_MULTI_VALUE),
        TimeDimension::Weeks(1),
        TimeDimension::Weeks(TIME_DIMENSION_MULTI_VALUE),
        TimeDimension::Months(1),
        TimeDimension::Months(TIME_DIMENSION_MULTI_VALUE),
        TimeDimension::Years(1),
        TimeDimension::Years(TIME_DIMENSION_MULTI_VALUE)
    ];
    const TIME_DIMENSION_SER: &str = formatcp!(
        "[{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?}]",
        "perSecond",
        "per69Seconds",
        "perMinute",
        "per69Minutes",
        "perHour",
        "per69Hours",
        "perDay",
        "per69Days",
        "perWeek",
        "per69Weeks",
        "perMonth",
        "per69Months",
        "perYear",
        "per69Years"
    );

    #[test]
    fn serde_time_dimension() {
        // serialize
        assert_eq!(
            serde_json::to_string(&TIME_DIMENSION_DE).unwrap(),
            TIME_DIMENSION_SER
        );

        // deserialize
        assert_eq!(
            serde_json::from_str::<Vec<TimeDimension>>(TIME_DIMENSION_SER).unwrap(),
            TIME_DIMENSION_DE
        );
    }

    const SINGLE_DE: OptionalPair<u32> = OptionalPair::Single(69);
    const PAIR_DE: OptionalPair<u32> = OptionalPair::Pair(69, 420);

    const SINGLE_SER: &str = "69";
    const PAIR_SER: &str = "[69,420]";

    type T = OptionalPair<u32>;

    #[test]
    fn serde_optional_pair() {
        assert_eq!(serde_json::to_string(&SINGLE_DE).unwrap(), SINGLE_SER);
        assert_eq!(serde_json::to_string(&PAIR_DE).unwrap(), PAIR_SER);

        assert_eq!(serde_json::from_str::<T>(SINGLE_SER).unwrap(), SINGLE_DE);
        assert_eq!(serde_json::from_str::<T>(PAIR_SER).unwrap(), PAIR_DE);
    }
}
