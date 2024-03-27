use std::fmt::{Display, Formatter};

use itertools::Itertools;

pub enum FlatTableValue {
    String(String),
    I64(i64),
    U64(u64),
    F64(f64),
    Bool(bool)
}

impl From<String> for FlatTableValue {
    fn from(value: String) -> Self {
        FlatTableValue::String(value)
    }
}

impl From<i64> for FlatTableValue {
    fn from(value: i64) -> Self {
        FlatTableValue::I64(value)
    }
}

impl From<u64> for FlatTableValue {
    fn from(value: u64) -> Self {
        FlatTableValue::U64(value)
    }
}

impl From<f64> for FlatTableValue {
    fn from(value: f64) -> Self {
        FlatTableValue::F64(value)
    }
}

impl From<bool> for FlatTableValue {
    fn from(value: bool) -> Self {
        FlatTableValue::Bool(value)
    }
}

impl Display for FlatTableValue {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            // FlatTableValue::String(s) => write!(fmt, "\"{}\"", s.replace("\"", "\"\"")),
            FlatTableValue::I64(i) => write!(fmt, "{i}"),
            FlatTableValue::U64(u) => write!(fmt, "{u}"),
            FlatTableValue::F64(f) => write!(fmt, "{f}"),
            FlatTableValue::Bool(b) => write!(fmt, "{b}"),

            FlatTableValue::String(s) => {
                write!(fmt, "\"")?;
                for line in Itertools::intersperse(s.lines(), "\n") {
                    fmt.write_str(&line.replace('\"', "\"\""))?;
                }
                write!(fmt, "\"")
            }
        }
    }
}
