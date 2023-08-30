use clap::{Parser, ValueEnum};
use std::path::PathBuf;
use std::fmt::{Display, Formatter};

/// NLWKN Water Right File Adapter
#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Args {
    /// Path to reports JSON file
    pub reports_json: PathBuf,

    /// Language for the field names
    ///
    /// `De` will use the names from the original reports
    #[arg(value_enum, long, short, default_value = "en")]
    pub lang: Lang,

    /// Output format
    #[arg(value_enum, long, short, default_value = "csv")]
    pub format: Format,

    /// Output file path
    #[arg(long, short)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum Lang {
    De,
    En
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum Format {
    Csv
}

impl Display for Format {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Format::Csv => write!(f, "csv")
        }
    }
}
