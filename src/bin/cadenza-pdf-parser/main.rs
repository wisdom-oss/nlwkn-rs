use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use clap::Parser;
use console::Color;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use indicatif::ProgressBar;
use itertools::Itertools;
use lazy_static::lazy_static;
use lopdf::Document;
use nlwkn_rs::cadenza::CadenzaTable;
use nlwkn_rs::cli::{progress_message, PROGRESS_STYLE, PROGRESS_UPDATE_INTERVAL, SPINNER_STYLE};
use nlwkn_rs::util::{OptionUpdate, zero_is_none};
use nlwkn_rs::{WaterRight, WaterRightNo};
use regex::Regex;
use tokio::task::JoinHandle;

use crate::parse::parse_document;

mod intermediate;
mod parse;

lazy_static! {
    static ref REPORT_FILE_RE: Regex = Regex::new(r"^rep(?<no>\d+).pdf$").expect("valid regex");
    static ref PROGRESS: ProgressBar = ProgressBar::new_spinner();
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

#[tokio::main]
async fn main() -> ExitCode {
    let Args {
        xlsx_path,
        data_path
    } = Args::parse();

    let report_dir = {
        let mut path_buf = data_path.clone();
        path_buf.push("reports");
        path_buf
    };

    PROGRESS.set_style(SPINNER_STYLE.clone());
    PROGRESS.enable_steady_tick(PROGRESS_UPDATE_INTERVAL);

    let (reports, _broken_reports) = match load_reports(report_dir) {
        Ok(reports) => reports,
        Err(e) => {
            progress_message(
                &PROGRESS,
                "Error",
                Color::Red,
                format!("could not load reports, {e}")
            );
            PROGRESS.finish_and_clear();
            return ExitCode::FAILURE;
        }
    };

    PROGRESS.set_style(SPINNER_STYLE.clone());
    PROGRESS.set_message("Parsing table...");
    let mut cadenza_table = match CadenzaTable::from_path(&xlsx_path) {
        Ok(table) => table,
        Err(err) => {
            progress_message(
                &PROGRESS,
                "Error",
                Color::Red,
                format!("could not parse table, {err}")
            );
            PROGRESS.finish_and_clear();
            return ExitCode::FAILURE;
        }
    };
    cadenza_table.sanitize();
    let cadenza_table = Arc::new(cadenza_table);

    PROGRESS.set_style(PROGRESS_STYLE.clone());
    PROGRESS.set_message("Parsing Reports");
    PROGRESS.set_length(reports.len() as u64);
    PROGRESS.set_position(0);

    let mut tasks = FuturesUnordered::new();
    for (water_right_no, document) in reports {
        let cadenza_table = cadenza_table.clone();
        // TODO: move this tasks into own function
        let task: JoinHandle<Result<WaterRight, (WaterRightNo, anyhow::Error)>> =
            tokio::spawn(async move {
                let mut water_right = WaterRight::new(water_right_no);
                if let Err(e) = parse_document(&mut water_right, document) {
                    return Err((water_right_no, e));
                }

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
                    .flat_map(|(_, department)| department.usage_locations.iter_mut())
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
                            ls.splitn(2, ' ')
                                .map(ToString::to_string)
                                .collect_tuple::<(String, String)>()
                        })
                    });
                    ul.county.update_if_none_clone(row.county.as_ref());
                    ul.rivershed.update_if_none_clone(row.rivershed.as_ref());
                    ul.groundwater_volume.update_if_none_clone(row.groundwater_volume.as_ref());
                    ul.flood_area.update_if_none_clone(row.flood_area.as_ref());
                    ul.water_protection_area
                        .update_if_none_clone(row.water_protection_area.as_ref());
                    ul.utm_easting.update_if_none_clone(row.utm_easting.as_ref());
                    ul.utm_northing.update_if_none_clone(row.utm_northing.as_ref());

                    // sanitize coordinates
                    ul.utm_easting = ul.utm_easting.and_then(zero_is_none);
                    ul.utm_northing = ul.utm_northing.and_then(zero_is_none);
                }

                // remove "Bemerkung: " from annotations if they begin with that
                if let Some(annotation) = water_right.annotation.as_ref() {
                    if annotation.starts_with("Bemerkung: ") {
                        water_right.annotation = annotation
                            .splitn(2, "Bemerkung: ")
                            .nth(1)
                            .expect("separator already checked")
                            .to_owned()
                            .into();
                    }
                }

                // fill granting authority if registering authority is set but not granting, the
                // registering authority then also granted
                if let (Some(register), None) = (
                    water_right.registering_authority.as_ref(),
                    water_right.granting_authority.as_ref()
                ) {
                    water_right.granting_authority = Some(register.to_string());
                }

                // TODO: normalize dates

                Ok(water_right)
            });

        tasks.push(task);
    }

    let mut water_rights = Vec::with_capacity(cadenza_table.rows().capacity());
    let mut parse_errors = BTreeMap::new();
    while let Some(task_res) = tasks.next().await {
        let parse_res = match task_res {
            Ok(parse_res) => parse_res,
            Err(err) => {
                progress_message(
                    &PROGRESS,
                    "Error",
                    Color::Red,
                    format!("could not join task, {err}")
                );
                PROGRESS.inc(1);
                continue;
            }
        };

        let water_right_no = match parse_res {
            Ok(water_right) => {
                let no = water_right.no;
                water_rights.push(water_right);
                no
            }

            Err((water_right_no, err)) => {
                progress_message(
                    &PROGRESS,
                    "Warning",
                    Color::Yellow,
                    format!("could not parse report for {water_right_no}, {err}, will be skipped")
                );
                parse_errors.insert(water_right_no, err.to_string());
                water_right_no
            }
        };

        PROGRESS.set_prefix(water_right_no.to_string());
        PROGRESS.inc(1);
    }

    // TODO: put following code into clear functions

    // save parsed reports

    PROGRESS.set_style(SPINNER_STYLE.clone());
    PROGRESS.set_message("Saving results...");

    let reports_json_path = {
        let mut path = data_path.clone();
        path.push("reports.json");
        path
    };

    #[cfg(debug_assertions)]
    let reports_json = serde_json::to_string_pretty(&water_rights);
    #[cfg(not(debug_assertions))]
    let reports_json = serde_json::to_string(&water_rights);
    let reports_json = match reports_json {
        Ok(json) => json,
        Err(e) => {
            progress_message(
                &PROGRESS,
                "Error",
                Color::Red,
                format!("could not serialize water rights to json, {e}")
            );
            PROGRESS.finish_and_clear();
            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = fs::write(&reports_json_path, reports_json) {
        progress_message(
            &PROGRESS,
            "Error",
            Color::Red,
            format!("could not write reports json, {e}")
        );
        PROGRESS.finish_and_clear();
        return ExitCode::FAILURE;
    }

    // save broken reports

    let broken_reports_json = match serde_json::to_string_pretty(
        &_broken_reports.into_iter().map(|(no, _)| no).collect::<Vec<WaterRightNo>>()
    ) {
        Ok(json) => json,
        Err(e) => {
            progress_message(
                &PROGRESS,
                "Error",
                Color::Red,
                format!("could not serialize broken reports to json, {e}")
            );
            PROGRESS.finish_and_clear();
            return ExitCode::FAILURE;
        }
    };

    let broken_reports_path = {
        let mut path = data_path.clone();
        path.push("broken-reports.json");
        path
    };

    if let Err(e) = fs::write(&broken_reports_path, broken_reports_json) {
        progress_message(
            &PROGRESS,
            "Error",
            Color::Red,
            format!("could not write broken reports json, {e}")
        );
        PROGRESS.finish_and_clear();
        return ExitCode::FAILURE;
    }

    // save parse errors

    let parse_errors_json = match serde_json::to_string_pretty(&parse_errors) {
        Ok(json) => json,
        Err(e) => {
            progress_message(
                &PROGRESS,
                "Error",
                Color::Red,
                format!("could not serialize parse errors to json, {e}")
            );
            PROGRESS.finish_and_clear();
            return ExitCode::FAILURE;
        }
    };

    let parse_errors_path = {
        let mut path = data_path.clone();
        path.push("parse-errors.json");
        path
    };

    if let Err(e) = fs::write(&parse_errors_path, parse_errors_json) {
        progress_message(
            &PROGRESS,
            "Error",
            Color::Red,
            format!("could not write parse errors json, {e}")
        );
        PROGRESS.finish_and_clear();
        return ExitCode::FAILURE;
    }

    PROGRESS.finish_and_clear();
    println!(
        "{}{}{} {}",
        console::style("Successfully parsed reports (").magenta(),
        console::style(water_rights.len()).cyan(),
        console::style(") written to").magenta(),
        console::style(reports_json_path.display()).cyan()
    );
    ExitCode::SUCCESS
}

type Reports = Vec<(WaterRightNo, Document)>;
type BrokenReports = Vec<(WaterRightNo, lopdf::Error)>;
fn load_reports(report_dir: impl AsRef<Path>) -> anyhow::Result<(Reports, BrokenReports)> {
    PROGRESS.set_message("Counting reports...");
    let entry_count = fs::read_dir(&report_dir)?.count();
    let read_dir = fs::read_dir(report_dir)?;

    PROGRESS.set_message("Loading Reports");
    PROGRESS.set_length(entry_count as u64);
    PROGRESS.set_position(0);
    PROGRESS.set_style(PROGRESS_STYLE.clone());

    let mut reports = Vec::with_capacity(entry_count);
    let mut broken_reports = Vec::with_capacity(entry_count);

    for dir_entry in read_dir {
        let dir_entry = dir_entry?;

        let file_name = dir_entry.file_name();
        let file_name = file_name.to_string_lossy();
        let Some(captured) = REPORT_FILE_RE.captures(file_name.as_ref())
        else {
            progress_message(
                &PROGRESS,
                "Warning",
                Color::Yellow,
                format!("could not extract water right number from {file_name:?}, will be ignored")
            );
            continue;
        };
        let water_right_no: WaterRightNo = captured["no"].parse()?;

        PROGRESS.set_prefix(water_right_no.to_string());

        match Document::load(dir_entry.path()) {
            Ok(document) => reports.push((water_right_no, document)),
            Err(err) => broken_reports.push((water_right_no, err))
        }

        PROGRESS.inc(1);
    }

    progress_message(
        &PROGRESS,
        "Loaded",
        Color::Green,
        format!("{} reports correctly", reports.len())
    );
    if !broken_reports.is_empty() {
        progress_message(
            &PROGRESS,
            "Warning",
            Color::Yellow,
            format!("could not load {} reports", broken_reports.len())
        );
    }

    Ok((reports, broken_reports))
}
