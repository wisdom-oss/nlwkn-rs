use std::fs;
use std::path::{Path, PathBuf};
use std::io::Write;

use clap::Parser;
use itertools::{any, Itertools};
use lazy_static::lazy_static;
use lopdf::Document;
use nlwkn_rs::cadenza::CadenzaTable;
use nlwkn_rs::{WaterRight, WaterRightNo};
use regex::Regex;

use crate::parse::parse_document;
use crate::util::OptionUpdate;

mod intermediate;
mod parse;
mod util;

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

    let (reports, broken_reports) = load_reports(report_dir).unwrap();
    if !broken_reports.is_empty() {
        println!("found {} broken reports", broken_reports.len());
    }

    let cadenza_table = CadenzaTable::from_path(&xlsx_path).unwrap();
    let mut water_rights = Vec::with_capacity(cadenza_table.rows().capacity());

    let mut out = std::io::stdout().lock();
    for (water_right_no, document) in reports {
        write!(out, "{water_right_no}").unwrap();
        let mut water_right = WaterRight::new(water_right_no);
        parse_document(&mut water_right, document).unwrap();

        for row in cadenza_table.rows().iter().filter(|row| row.no == water_right_no) {
            let wr = &mut water_right;
            wr.bailee.update_if_none_clone(row.bailee.as_ref());
            wr.valid_to.update_if_none_clone(row.valid_to.as_ref());
            wr.state.update_if_none_clone(row.state.as_ref());
            wr.valid_from.update_if_none_clone(row.valid_from.as_ref());
            wr.legal_title.update_if_none_clone(row.legal_title.as_ref());
            wr.water_authority.update_if_none_clone(row.water_authority.as_ref());
            wr.granting_authority.update_if_none_clone(row.granting_authority.as_ref());
            wr.date_of_change.update_if_none_clone(row.date_of_change.as_ref());
            wr.file_reference.update_if_none_clone(row.file_reference.as_ref());
            wr.external_identifier.update_if_none_clone(row.external_identifier.as_ref());
            wr.address.update_if_none_clone(row.address.as_ref());
        }

        for usage_location in water_right
            .legal_departments
            .iter_mut()
            .map(|(_, department)| department.usage_locations.iter_mut())
            .flatten()
        {
            let Some(row) = cadenza_table.rows().iter().find(|row| {
                row.no == water_right_no &&
                    usage_location.name.is_some() &&
                    row.usage_location == usage_location.name
            })
            else {
                continue;
            };

            let ul = usage_location;
            ul.no.update_if_none(Some(row.usage_location_no));
            ul.legal_scope.update_if_none_with(|| {
                row.legal_scope.as_ref().and_then(|ls| {
                    ls.splitn(2, " ").map(ToString::to_string).collect_tuple::<(String, String)>()
                })
            });
            ul.county.update_if_none_clone(row.county.as_ref());
            ul.rivershed.update_if_none_clone(row.rivershed.as_ref());
            ul.groundwater_volume.update_if_none_clone(row.groundwater_volume.as_ref());
            ul.flood_area.update_if_none_clone(row.flood_area.as_ref());
            ul.water_protection_area.update_if_none_clone(row.water_protection_area.as_ref());
            ul.utm_easting.update_if_none_clone(row.utm_easting.as_ref());
            ul.utm_northing.update_if_none_clone(row.utm_northing.as_ref());
        }

        // TODO: sanitize utm values
        // TODO: normalize dates

        water_rights.push(water_right);
        break;
    }

    dbg!(water_rights);
}

type Reports = Vec<(WaterRightNo, Document)>;
type BrokenReports = Vec<(WaterRightNo, lopdf::Error)>;
fn load_reports(report_dir: impl AsRef<Path>) -> anyhow::Result<(Reports, BrokenReports)> {
    let read_dir = fs::read_dir(report_dir)?.into_iter();

    let size_hint = read_dir.size_hint();
    let size_hint = size_hint.1.unwrap_or(size_hint.0);

    let mut reports = Vec::with_capacity(size_hint);
    let mut broken_reports = Vec::with_capacity(size_hint);

    for dir_entry in read_dir {
        let dir_entry = dir_entry?;

        let file_name = dir_entry.file_name();
        let file_name = file_name.to_string_lossy();
        let Some(captured) = REPORT_FILE_RE.captures(file_name.as_ref()) else {
            // file is not a fetched pdf file
            continue;
        };
        let water_right_no: WaterRightNo = captured["no"].parse()?;

        match Document::load(dir_entry.path()) {
            Ok(document) => reports.push((water_right_no, document)),
            Err(err) => broken_reports.push((water_right_no, err))
        }
    }

    Ok((reports, broken_reports))
}
