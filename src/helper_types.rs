use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{DeserializeAs, OneOrMany, Same};
use std::cmp::Ordering;

use crate::util::data_structs;

#[derive(Debug, Eq, PartialEq, Hash, Ord)]
pub enum TimeDimension {
    Seconds(u64),
    Minutes(u64),
    Hours(u64),
    Days(u64),
    Weeks(u64),
    Months(u64),
    Years(u64),
}

impl TimeDimension {
    /// Rough conversion to seconds.
    ///
    /// Imprecise for dimensions larger than weeks.
    pub fn as_seconds(&self) -> u64 {
        use TimeDimension::*;

        match self {
            Seconds(s) => *s,
            Minutes(m) => *m * 60,
            Hours(h) => *h * 60 * 60,
            Days(d) => *d * 24 * 60 * 60,
            Weeks(w) => *w * 7 * 24 * 60 * 60,
            Months(m) => *m * 30 * 24 * 60 * 60,
            Years(y) => *y * 365 * 24 * 60 * 60,
        }
    }
}

impl Serialize for TimeDimension {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use TimeDimension::*;

        let as_str = match self {
            Seconds(1) => String::from("perSecond"),
            Seconds(s) => format!("per{}Seconds", s),

            Minutes(1) => String::from("perMinute"),
            Minutes(m) => format!("per{}Minutes", m),

            Hours(1) => String::from("perHour"),
            Hours(h) => format!("per{}Hours", h),

            Days(1) => String::from("perDay"),
            Days(d) => format!("per{}Days", d),

            Weeks(1) => String::from("perWeek"),
            Weeks(w) => format!("per{}Weeks", w),

            Months(1) => String::from("perMonth"),
            Months(m) => format!("per{}Months", m),

            Years(1) => String::from("perYear"),
            Years(y) => format!("per{}Years", y),
        };

        as_str.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TimeDimension {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use TimeDimension::*;

        let from_str = String::deserialize(deserializer)?;
        let digits: String = from_str.matches(char::is_numeric).collect();
        let digits: Result<u64, _> = digits.parse().map_err(|_| {
            serde::de::Error::custom(format!("could not parse {} as numeric value", digits))
        });

        match from_str {
            s if s.contains("Seconds") => Ok(Seconds(digits?)),
            s if s.contains("Second") => Ok(Seconds(1)),

            s if s.contains("Minutes") => Ok(Minutes(digits?)),
            s if s.contains("Minute") => Ok(Minutes(1)),

            s if s.contains("Hours") => Ok(Hours(digits?)),
            s if s.contains("Hour") => Ok(Hours(1)),

            s if s.contains("Days") => Ok(Days(digits?)),
            s if s.contains("Day") => Ok(Days(1)),

            s if s.contains("Weeks") => Ok(Weeks(digits?)),
            s if s.contains("Week") => Ok(Weeks(1)),

            s if s.contains("Months") => Ok(Months(digits?)),
            s if s.contains("Month") => Ok(Months(1)),

            s if s.contains("Years") => Ok(Years(digits?)),
            s if s.contains("Year") => Ok(Years(1)),

            _ => Err(serde::de::Error::custom(format!(
                "could not parse {} as time dimension",
                from_str
            ))),
        }
    }
}

impl PartialOrd for TimeDimension {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_seconds().partial_cmp(&other.as_seconds())
    }
}

data_structs! {
    /// A number that has a unit.
    struct DimensionedNumber {
        value: i64,
        unit: String,
    }

    /// A number that has a unit and a description.
    struct DescriptiveNumber {
        value: i64,
        unit: String,
        description?: String,
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum OptionalPair<T> {
    Single(T),
    Pair(T, T),
}

impl<T> Serialize for OptionalPair<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use OptionalPair::*;

        match self {
            Single(v) => v.serialize(serializer),
            Pair(a, b) => (a, b).serialize(serializer),
        }
    }
}

impl<'de, T> Deserialize<'de> for OptionalPair<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
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
            (Some(a), Some(b), _) => Ok(OptionalPair::Pair(a, b)),
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
        TimeDimension::Years(TIME_DIMENSION_MULTI_VALUE),
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
