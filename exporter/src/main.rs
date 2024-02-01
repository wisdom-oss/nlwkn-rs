use std::path::PathBuf;
use clap::Parser;

/// NLWKN Water Right DB Exporter
#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// Path to reports JSON file
    pub reports_json: PathBuf
}

fn main() {
    let Args {reports_json} = Args::parse();
    println!("{:?}", reports_json);
}
