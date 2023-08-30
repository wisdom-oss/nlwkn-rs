use std::fmt::Display;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use args::{Args, Format, Lang};
use nlwkn::WaterRight;

use crate::flat_table::FlatTable;

mod args;
mod flat_table;

fn main() {
    let Args {
        reports_json,
        lang,
        format,
        out
    } = Args::parse();

    let report_json_content =
        fs::read_to_string(&reports_json).expect("could not read reports json");

    let out = match out {
        Some(out) => out,
        None => match (reports_json.parent(), reports_json.file_stem()) {
            (Some(parent), Some(file_stem)) => {
                let mut path_buf = PathBuf::from(parent);
                let mut file_name = file_stem.to_owned();
                file_name.push(".");
                file_name.push(format.to_string());
                path_buf.push(file_name);
                path_buf
            }
            (None, Some(file_stem)) => {
                let mut file_name = file_stem.to_owned();
                file_name.push(".");
                file_name.push(format.to_string());
                PathBuf::from(file_name)
            }
            (_, None) => panic!("`report_json` is no file path")
        }
    };

    let mut water_rights: Vec<WaterRight> =
        serde_json::from_str(&report_json_content).expect("could not parse reports json");
    println!("parsed json");

    water_rights = water_rights.into_iter().filter(|_| true).collect();
    println!("filtered water rights");

    let mut out_file = File::create(out).expect("could not create output file");
    let mut out_string = String::new();
    match (format, lang) {
        (Format::Csv, Lang::En) => {
            let flat_table: FlatTable<flat_table::marker::En> =
                flat_table::FlatTable::from_water_rights(water_rights.as_slice());
            flat_table.fmt_csv(&mut out_string).expect("could not format csv");
        }
        (Format::Csv, Lang::De) => {
            let flat_table: FlatTable<flat_table::marker::De> =
                flat_table::FlatTable::from_water_rights(water_rights.as_slice());
            flat_table.fmt_csv(&mut out_string).expect("could not format csv");
        }
    }
    out_file.write_all(out_string.as_bytes()).expect("could not write to out file");
}
