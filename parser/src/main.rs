use std::collections::{BTreeMap, HashMap};
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use clap::Parser;
use console::{Color, Style};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use indicatif::ProgressBar;
use itertools::Itertools;
use lazy_static::lazy_static;
use lopdf::Document;
use nlwkn::cadenza::CadenzaTable;
use nlwkn::cli::{progress_message, PROGRESS_STYLE, PROGRESS_UPDATE_INTERVAL, SPINNER_STYLE};
use nlwkn::util::{zero_is_none, OptionUpdate};
use nlwkn::{WaterRight, WaterRightNo};
use regex::Regex;
use tokio::task::JoinHandle;

use crate::parse::parse_document;

mod intermediate;
mod parse;

lazy_static! {
    static ref REPORT_FILE_RE: Regex = Regex::new(r"^rep(?<no>\d+).pdf$").expect("valid regex");
    static ref PROGRESS: ProgressBar = ProgressBar::new_spinner();
}

/// NLWKN Water Right Parser
#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// Path to cadenza-provided xlsx file
    xlsx_path: PathBuf,

    /// Path to data directory
    #[arg(default_value = "data")]
    data_path: PathBuf,

    /// Parse specific water right number report
    #[arg(long = "no")]
    water_right_no: Option<WaterRightNo>
}

// TODO: add edge case handling input

#[tokio::main]
async fn main() -> ExitCode {
    let Args {
        xlsx_path,
        data_path,
        water_right_no: arg_no
    } = Args::parse();

    let report_dir = {
        let mut path_buf = data_path.clone();
        path_buf.push("reports");
        path_buf
    };

    PROGRESS.set_style(SPINNER_STYLE.clone());
    PROGRESS.enable_steady_tick(PROGRESS_UPDATE_INTERVAL);

    let (reports, broken_reports) = match load_reports(report_dir, arg_no) {
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
    PROGRESS.set_prefix("🚀");

    let mut tasks = FuturesUnordered::new();
    let reports = reports.into_iter().filter(|(rep_no, _)| match arg_no {
        Some(arg_no) => *rep_no == arg_no,
        None => true
    });
    for (water_right_no, document) in reports {
        let cadenza_table = cadenza_table.clone();
        tasks.push(parsing_task(water_right_no, document, cadenza_table));
    }

    let mut water_rights = Vec::with_capacity(cadenza_table.rows().capacity());
    let mut pdf_only_water_rights = Vec::with_capacity(cadenza_table.rows().capacity());
    let mut parsing_issues = BTreeMap::new();
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

        let _water_right_no = match parse_res {
            Ok((water_right, enriched)) => {
                let no = water_right.no;
                match enriched {
                    true => water_rights.push(water_right),
                    false => pdf_only_water_rights.push(water_right)
                }
                no
            }

            Err((water_right_no, err)) => {
                progress_message(
                    &PROGRESS,
                    "Warning",
                    Color::Yellow,
                    format!("could not parse report for {water_right_no}, {err}, will be skipped")
                );
                parsing_issues.insert(water_right_no, err.to_string());
                water_right_no
            }
        };

        PROGRESS.inc(1);
    }

    PROGRESS.set_style(SPINNER_STYLE.clone());
    PROGRESS.set_message("Saving results...");
    let ResultPaths {
        broken_reports_path,
        parsing_issues_path,
        pdf_only_reports_path,
        reports_path
    } = match save_results(
        &data_path,
        &water_rights,
        &pdf_only_water_rights,
        &broken_reports,
        &parsing_issues
    ) {
        Ok(paths) => paths,
        Err(e) => {
            progress_message(&PROGRESS, "Error", Color::Red, e);
            PROGRESS.finish_and_clear();
            return ExitCode::FAILURE;
        }
    };

    PROGRESS.finish_and_clear();
    eprintln!();
    print!("{}", Report {
        broken: (broken_reports.len(), broken_reports_path.display()),
        parsing_issues: (parsing_issues.len(), parsing_issues_path.display()),
        pdf_only: (pdf_only_water_rights.len(), pdf_only_reports_path.display()),
        successful: (water_rights.len(), reports_path.display())
    });
    ExitCode::SUCCESS
}

type Reports = Vec<(WaterRightNo, Document)>;
type BrokenReports = Vec<(WaterRightNo, lopdf::Error)>;
#[inline]
fn load_reports(
    report_dir: impl AsRef<Path>,
    selected: Option<WaterRightNo>
) -> anyhow::Result<(Reports, BrokenReports)> {
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

        match selected {
            Some(selected) if selected != water_right_no => (),
            _ => match Document::load(dir_entry.path()) {
                Ok(document) => reports.push((water_right_no, document)),
                Err(err) => broken_reports.push((water_right_no, err))
            }
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

// TODO: this uses tokio for parallelization, tokio is here not the best choice
// since these       operations are cpu-intensive, rayon would be a better
// choice
#[inline]
fn parsing_task(
    water_right_no: WaterRightNo,
    report_doc: Document,
    cadenza_table: Arc<CadenzaTable>
) -> JoinHandle<Result<(WaterRight, bool), (WaterRightNo, anyhow::Error)>> {
    tokio::spawn(async move {
        let mut water_right = WaterRight::new(water_right_no);
        if let Err(e) = parse_document(&mut water_right, report_doc) {
            return Err((water_right_no, e));
        }

        let mut enriched = false;
        for row in cadenza_table.rows().iter().filter(|row| row.no == water_right_no) {
            enriched = true;
            let wr = &mut water_right;
            wr.rights_holder.update_if_none_clone(row.rights_holder.as_ref());
            wr.valid_until.update_if_none_clone(row.valid_until.as_ref());
            wr.status.update_if_none_clone(row.status.as_ref());
            wr.valid_from.update_if_none_clone(row.valid_from.as_ref());
            wr.legal_title.update_if_none_clone(row.legal_title.as_ref());
            wr.water_authority.update_if_none_clone(row.water_authority.as_ref());
            wr.granting_authority.update_if_none_clone(row.granting_authority.as_ref());
            wr.date_of_change.update_if_none_clone(row.date_of_change.as_ref());
            wr.file_reference.update_if_none_clone(row.file_reference.as_ref());
            wr.external_identifier.update_if_none_clone(row.external_identifier.as_ref());
            wr.address.update_if_none_clone(row.address.as_ref());
        }

        let mut relevant_cadenza_rows: HashMap<_, _> = cadenza_table
            .rows()
            .iter()
            .filter(|row| row.no == water_right_no)
            .map(|row| (row.usage_location_no, row))
            .collect();

        for usage_location in water_right
            .legal_departments
            .iter_mut()
            .flat_map(|(_, department)| department.usage_locations.iter_mut())
        {
            let usage_location_by_name = relevant_cadenza_rows.values().find(|row| {
                usage_location.name.is_some() && row.usage_location == usage_location.name
            });
            let usage_location_by_coords = relevant_cadenza_rows.values().find(|row| {
                usage_location.utm_easting.is_some() &&
                    row.utm_easting == usage_location.utm_easting &&
                    usage_location.utm_northing.is_some() &&
                    row.utm_northing == usage_location.utm_northing
            });

            let usage_location_no = match (usage_location_by_name, usage_location_by_coords) {
                (Some(usage_location), _) | (None, Some(usage_location)) => {
                    usage_location.usage_location_no
                }
                (None, None) => {
                    progress_message(
                        &PROGRESS,
                        "Warning",
                        Color::Yellow,
                        format!(
                            "could not find usage location no for report {water_right_no}, \
                             enrichment may be missing values"
                        )
                    );
                    continue;
                }
            };

            let row = relevant_cadenza_rows
                .remove(&usage_location_no)
                .expect("we got the no from the that map");

            let ul = usage_location;
            ul.no.update_if_none(Some(row.usage_location_no));
            ul.legal_purpose.update_if_none_with(|| {
                row.legal_purpose.as_ref().and_then(|ls| {
                    ls.splitn(2, ' ').map(ToString::to_string).collect_tuple::<(String, String)>()
                })
            });
            ul.county.update_if_none_clone(row.county.as_ref());
            ul.river_basin.update_if_none_clone(row.river_basin.as_ref());
            ul.groundwater_body.update_if_none_clone(row.groundwater_body.as_ref());
            ul.flood_area.update_if_none_clone(row.flood_area.as_ref());
            ul.water_protection_area.update_if_none_clone(row.water_protection_area.as_ref());
            ul.utm_easting.update_if_none_clone(row.utm_easting.as_ref());
            ul.utm_northing.update_if_none_clone(row.utm_northing.as_ref());

            // sanitize coordinates
            ul.utm_easting = ul.utm_easting.and_then(zero_is_none);
            ul.utm_northing = ul.utm_northing.and_then(zero_is_none);
        }

        if !relevant_cadenza_rows.is_empty() {
            let missing_locations = relevant_cadenza_rows.keys().collect::<Vec<_>>();
            progress_message(
                &PROGRESS,
                "Warning",
                Color::Yellow,
                format!(
                    "in the report {} the usage locations {:?} are missing",
                    water_right_no, missing_locations
                )
            );
        }

        // remove "Bemerkung: " from annotations if they begin with that
        match water_right.annotation.as_ref() {
            Some(annotation) if annotation == "Bemerkung:" => water_right.annotation = None,
            Some(annotation) if annotation.starts_with("Bemerkung: ") => {
                water_right.annotation = annotation
                    .split_once("Bemerkung: ")
                    .map(|x| x.1)
                    .expect("separator already checked")
                    .to_owned()
                    .into();
            }
            _ => ()
        }

        // fill granting authority if registering authority is set but not granting, the
        // registering authority then also granted
        if let (Some(register), None) = (
            water_right.registering_authority.as_ref(),
            water_right.granting_authority.as_ref()
        ) {
            water_right.granting_authority = Some(register.to_string());
        }

        // normalize dates into ISO form
        for date_opt in [
            &mut water_right.valid_until,
            &mut water_right.valid_from,
            &mut water_right.first_grant,
            &mut water_right.date_of_change
        ] {
            let Some(date) = date_opt.as_ref()
            else {
                continue;
            };

            let mut split = date.split('.');
            let day = split.next();
            let month = split.next();
            let year = split.next();
            if split.next().is_some() {
                progress_message(
                    &PROGRESS,
                    "Warning",
                    Color::Yellow,
                    format!("a date in {water_right_no} has an invalid format")
                );
                continue;
            }

            if let (Some(day), Some(month), Some(year)) = (day, month, year) {
                let _ = date_opt.insert(format!("{year}-{month}-{day}"));
            }
        }

        Ok((water_right, enriched))
    })
}

struct ResultPaths {
    pub broken_reports_path: PathBuf,
    pub parsing_issues_path: PathBuf,
    pub pdf_only_reports_path: PathBuf,
    pub reports_path: PathBuf
}
#[inline]
fn save_results(
    data_path: &Path,
    water_rights: &[WaterRight],
    pdf_only_water_rights: &[WaterRight],
    broken_reports: &BrokenReports,
    parsing_issues: &BTreeMap<WaterRightNo, String>
) -> Result<ResultPaths, String> {
    // TODO: use multiple smaller functions for clarity

    // save parsed reports

    let reports_json_path = {
        let mut path: PathBuf = data_path.clone().into();
        path.push("reports.json");
        path
    };

    #[cfg(debug_assertions)]
    let reports_json = serde_json::to_string_pretty(water_rights);
    #[cfg(not(debug_assertions))]
    let reports_json = serde_json::to_string(&water_rights);
    let reports_json = match reports_json {
        Ok(json) => json,
        Err(e) => return Err(format!("could not serialize water rights to json, {e}"))
    };

    if let Err(e) = fs::write(&reports_json_path, reports_json) {
        return Err(format!("could not write reports json, {e}"));
    }

    // save pdf only reports

    let pdf_only_reports_json_path = {
        let mut path: PathBuf = data_path.clone().into();
        path.push("pdf-only-reports.json");
        path
    };

    #[cfg(debug_assertions)]
    let pdf_only_reports_json = serde_json::to_string_pretty(pdf_only_water_rights);
    #[cfg(not(debug_assertions))]
    let pdf_only_reports_json = serde_json::to_string(&pdf_only_water_rights);
    let pdf_only_reports_json = match pdf_only_reports_json {
        Ok(json) => json,
        Err(e) => {
            return Err(format!(
                "could not serialize pdf only water rights to json, {e}"
            ))
        }
    };

    if let Err(e) = fs::write(&pdf_only_reports_json_path, pdf_only_reports_json) {
        return Err(format!("could not write pdf only reports json, {e}"));
    }

    // save broken reports

    let broken_reports_json = match serde_json::to_string_pretty(
        &broken_reports.iter().map(|(no, _)| no).copied().collect::<Vec<WaterRightNo>>()
    ) {
        Ok(json) => json,
        Err(e) => return Err(format!("could not serialize broken reports to json, {e}"))
    };

    let broken_reports_path = {
        let mut path: PathBuf = data_path.clone().into();
        path.push("broken-reports.json");
        path
    };

    if let Err(e) = fs::write(&broken_reports_path, broken_reports_json) {
        return Err(format!("could not write broken reports json, {e}"));
    }

    // save parsing issues

    let parsing_issues_json = match serde_json::to_string_pretty(&parsing_issues) {
        Ok(json) => json,
        Err(e) => return Err(format!("could not serialize parsing issues to json, {e}"))
    };

    let parsing_issues_path = {
        let mut path: PathBuf = data_path.clone().into();
        path.push("parsing-issues.json");
        path
    };

    if let Err(e) = fs::write(&parsing_issues_path, parsing_issues_json) {
        return Err(format!("could not write parsing issues json, {e}"));
    }

    Ok(ResultPaths {
        broken_reports_path,
        parsing_issues_path,
        pdf_only_reports_path: pdf_only_reports_json_path,
        reports_path: reports_json_path
    })
}

struct Report<T0, T1, T2, T3> {
    broken: (usize, T0),
    parsing_issues: (usize, T1),
    pdf_only: (usize, T2),
    successful: (usize, T3)
}

impl<T0, T1, T2, T3> Display for Report<T0, T1, T2, T3>
where
    T0: Display,
    T1: Display,
    T2: Display,
    T3: Display
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let description_style = Style::new().fg(Color::Yellow);
        let category_style = Style::new().fg(Color::Magenta);
        let key_style = Style::new().fg(Color::Cyan);
        let equal_style = Style::new().fg(Color::White);
        let num_value_style = Style::new().fg(Color::Magenta).bright();
        let str_value_style = Style::new().fg(Color::Blue).bright();

        let description_indicator = description_style.apply_to("#");
        let identifier_open = category_style.apply_to("[");
        let identifier_close = category_style.apply_to("]");
        let count_key = key_style.apply_to("count");
        let output_file_key = key_style.apply_to("output_file");
        let equal_sign = equal_style.apply_to("=");
        let string_indicator = str_value_style.apply_to("'");

        let entries: &[(Vec<&str>, &str, usize, &dyn Display)] = &[
            (
                vec![
                    "Broken PDF files which cannot be loaded.",
                    "Could be due to corrupted or incompatible files.",
                ],
                "broken",
                self.broken.0,
                &self.broken.1
            ),
            (
                vec![
                    "Reports with parsing issues.",
                    "First issue with it's respective water right number.",
                ],
                "parsing_issues",
                self.parsing_issues.0,
                &self.parsing_issues.1
            ),
            (
                vec![
                    "Reports where data could only be extracted from the PDF file.",
                    "XLSX data might be missing.",
                ],
                "pdf_only",
                self.pdf_only.0,
                &self.pdf_only.1
            ),
            (
                vec!["Reports parsed and enriched with both PDF and XLSX data."],
                "reports",
                self.successful.0,
                &self.successful.1
            )
        ];

        for (description, identifier, count, output_file) in entries {
            for description in description {
                writeln!(
                    f,
                    "{} {}",
                    description_indicator,
                    description_style.apply_to(description)
                )?;
            }
            writeln!(
                f,
                "{}{}{}",
                identifier_open,
                category_style.apply_to(identifier),
                identifier_close
            )?;
            writeln!(
                f,
                "{} {} {}",
                count_key,
                equal_sign,
                num_value_style.apply_to(count)
            )?;
            writeln!(
                f,
                "{} {} {}{}{}",
                output_file_key,
                equal_sign,
                string_indicator,
                str_value_style.apply_to(output_file),
                string_indicator
            )?;
            writeln!(f)?;
        }

        Ok(())
    }
}
