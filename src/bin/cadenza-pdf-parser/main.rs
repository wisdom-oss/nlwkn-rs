use std::fs;
use std::path::PathBuf;

use clap::Parser;
use lazy_static::lazy_static;
use lopdf::Document;
use nlwkn_rs::{WaterRight, WaterRightNo};
use regex::Regex;
use nlwkn_rs::cadenza::CadenzaTable;
use crate::parse::parse_document;

mod intermediate;
mod parse;

lazy_static! {
    static ref REPORT_FILE_RE: Regex = Regex::new(r"^rep(?<no>\d+).pdf$").expect("valid regex");
}

/// NLWKN Water Right Webcrawler
#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// Path to cadenza-provided xlsx file
    xlsx_path: PathBuf,

    /// Path to data directory
    #[arg(default_value = "data")]
    data_path: PathBuf
}

fn main() {
    let Args {
        xlsx_path,
        data_path
    } = Args::parse();

    let report_dir = {
        let mut path_buf = data_path.clone();
        path_buf.push("reports");
        path_buf
    };

    let reports = fs::read_dir(report_dir)
        .unwrap()
        .into_iter()
        .map(|read_dir| {
            read_dir.map(|dir_entry| {
                let file_name = dir_entry.file_name();
                let file_name = file_name.to_string_lossy();
                let captured = REPORT_FILE_RE.captures(file_name.as_ref()).unwrap();
                let water_right_no: WaterRightNo = captured["no"].parse().unwrap();
                let document = Document::load(dir_entry.path()).unwrap();
                (water_right_no, document)
            })
        })
        .collect::<Result<Vec<(WaterRightNo, Document)>, _>>()
        .unwrap();

    let cadenza_table = CadenzaTable::from_path(&xlsx_path).unwrap();

    for (water_right_no, document) in reports {
        let mut water_right = WaterRight::new(water_right_no);
        parse_document(&mut water_right, document).unwrap();

        if let Some(row) = cadenza_table.rows().iter().find(|row| row.no == water_right_no) {
            water_right.bailee = water_right.bailee.or_else(|| row.bailee.clone())
            todo!() // TODO: more
        }

        for (_, department) in water_right.legal_departments.iter_mut() {
            for usage_location in department.usage_locations.iter_mut() {
                if let Some(row) = cadenza_table.rows().iter().find(|row| row.no == water_right_no && usage_location.name.is_some() && row.usage_location == usage_location.name) {
                    usage_location.no = Some(row.usage_location_no);
                }
            }
        }

        break;
    }
}

// fn main() -> anyhow::Result<()> {
//     let document = lopdf::Document::load(
//         env::args()
//             .nth(1)
//             .ok_or(anyhow::Error::msg("no argument passed"))?
//     )?;
//     let text_block_repr = TextBlockRepr::try_from(document.clone())?;
//     let key_value_repr = KeyValueRepr::from(text_block_repr);
//
//     for (key, values) in key_value_repr.0.iter() {
//         print!("{}: ", console::style(key).magenta());
//         for value in values {
//             print!("{}, ", console::style(value).cyan());
//         }
//         println!()
//     }
//
//
//
//     let water_right = parse::parse_document(287209, document)?;
//     dbg!(water_right);
//
//     Ok(())
// }
