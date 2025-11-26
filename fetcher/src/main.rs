use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{fs, io};

use clap::Parser;
use console::{Alignment, Color};
use indicatif::ProgressBar;
use nlwkn::cadenza::{CadenzaTable, CadenzaTableRow};
use nlwkn::cli::{progress_message, ProgressBarGuard, PRINT_PADDING};
use nlwkn::WaterRightNo;
use reqwest::redirect::Policy;
use thiserror::Error;

use crate::req::{FetchReportUrlError, JSessionId};
use crate::tor::start_socks_proxy;

// mod browse;
mod req;
mod tor;

static_toml::static_toml! {
    static CONFIG = include_toml!("config.toml");
}

/// NLWKN Water Right Webcrawler
#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// Path to cadenza-provided xlsx file
    #[clap(required_unless_present = "water_right_no")]
    xlsx_path: Option<PathBuf>,

    /// Water right number to fetch
    #[clap(long = "no")]
    water_right_no: Option<WaterRightNo>,

    /// Ignore already downloaded files
    #[clap(long)]
    force: bool
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let _proxy_handle = tokio::spawn(start_socks_proxy());

    let to_fetch = match (args.water_right_no, args.xlsx_path) {
        (Some(no), _) => vec![no],
        (None, Some(xlsx_path)) => collect_no_from_cadenza_table(&xlsx_path),
        (None, None) => unreachable!("handled by clap")
    };

    let client = reqwest::ClientBuilder::new()
        .proxy(
            reqwest::Proxy::http(format!("socks5://localhost:{}", *tor::SOCKS_PORT).as_str())
                .expect("proxy schema invalid")
        )
        .redirect(Policy::none())
        .timeout(Duration::from_mins(5))
        .build()
        .expect("cannot build GET client");

    let mut j_session_id = None;

    {
        let _pb = ProgressBarGuard::new_wait_spinner("Waiting for TOR proxy...");
        while client.get(CONFIG.cadenza.url).send().await.is_err() {
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }

    fs::create_dir_all(CONFIG.data.reports).expect("could not create necessary directories");

    let mut fetched_reports = match args.force {
        true => BTreeSet::new(),
        false => {
            let _pb = ProgressBarGuard::new_wait_spinner("Fetching already downloaded reports...");
            BTreeSet::from_iter(
                find_fetched_reports()
                    .expect("could not find already fetched reports")
                    .iter()
                    .copied()
            )
        }
    };

    let mut unfetched_reports = Vec::new();

    let progress = ProgressBar::new(to_fetch.len() as u64)
        .with_style(nlwkn::cli::PROGRESS_STYLE.clone())
        .with_message("Fetching Reports");
    progress.enable_steady_tick(Duration::from_secs(1));

    'wr_loop: for water_right_no in to_fetch {
        if fetched_reports.contains(&water_right_no) {
            progress_message(
                &progress,
                "Skipped",
                Color::Green,
                format!("{water_right_no}, already fetched")
            );
            progress.inc(1);
            continue;
        }

        progress.set_prefix(water_right_no.to_string());
        progress.tick();

        for retry in 1..=(CONFIG.cadenza.retries as u32) {
            let fetched = fetch(water_right_no, &client, j_session_id.as_ref()).await;
            match fetched {
                Ok(new_j_session_id) => {
                    progress_message(&progress, "Fetched", Color::Green, water_right_no);
                    progress.inc(1);
                    fetched_reports.insert(water_right_no);
                    j_session_id = Some(new_j_session_id);
                    continue 'wr_loop;
                }

                Err(FetchError::ReportUrl(FetchReportUrlError::NoResults)) => {
                    progress_message(
                        &progress,
                        "Warning",
                        Color::Yellow,
                        format!("no results found for {water_right_no}")
                    );
                    progress.inc(1);
                    continue 'wr_loop;
                }

                Err(err) => {
                    progress_message(
                        &progress,
                        "Error",
                        Color::Red,
                        format!("failed to fetch, {err}")
                    );

                    // start with a new session
                    j_session_id = None;

                    // use quadratic backoff for wait until retry
                    let wait = 2u64.pow(retry);
                    progress.println(format!(
                        "{}  will try again in {wait} seconds...",
                        console::pad_str("", PRINT_PADDING, Alignment::Right, None)
                    ));
                    tokio::time::sleep(Duration::from_secs(wait)).await;
                }
            }
        }

        unfetched_reports.push(water_right_no);
        progress_message(
            &progress,
            "Warning",
            Color::Yellow,
            format!("exceeded amount of retries, will skip {water_right_no}")
        );
        progress.inc(1);
    }

    progress.finish_and_clear();
    match unfetched_reports.is_empty() {
        false => println!(
            "{}, could not fetch: {}",
            console::style("Fetching done").magenta(),
            unfetched_reports.iter().map(|no| no.to_string()).collect::<Vec<String>>().join(", ")
        ),
        true => println!("{}", console::style("Fetched all reports").magenta())
    }
}

#[derive(Debug, Error)]
enum FetchError {
    #[error(transparent)]
    ReportUrl(#[from] FetchReportUrlError),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    Write(#[from] io::Error)
}

async fn fetch(
    water_right_no: WaterRightNo,
    client: &reqwest::Client,
    j_session_id: Option<&JSessionId>
) -> Result<JSessionId, FetchError> {
    let (report_link, j_session_id) =
        req::fetch_report_url(water_right_no, client, j_session_id).await?;
    let pdf_bytes = client.get(&report_link).send().await?.bytes().await?;
    fs::write(
        format!("{}/rep{}.pdf", CONFIG.data.reports, water_right_no),
        pdf_bytes
    )?;

    Ok(j_session_id)
}

fn collect_no_from_cadenza_table(xlsx_path: &Path) -> Vec<WaterRightNo> {
    let mut cadenza_table = {
        let _pb = ProgressBarGuard::new_wait_spinner("Parsing table...");
        CadenzaTable::from_path(xlsx_path).expect("could not parse table")
    };

    {
        let _pb = ProgressBarGuard::new_wait_spinner("Sorting table...");
        cadenza_table.sort_by(sort_cadenza_table);
    }

    {
        let _pb = ProgressBarGuard::new_wait_spinner("Deduplicating table...");
        cadenza_table.dedup_by(dedup_cadenza_table);
    }

    cadenza_table.rows().iter().map(|row| row.no).collect()
}

fn sort_cadenza_table(a: &CadenzaTableRow, b: &CadenzaTableRow) -> Ordering {
    // we want the `E` legal departments first

    // the legal department abbreviations are unreliable, therefore this
    let a_has_e = a.legal_department.starts_with("Entnahme");
    let b_has_e = b.legal_department.starts_with("Entnahme");

    // also prioritize some counties
    let prioritized_counties = ["Aurich", "Wittmund", "Friesland", "Leer"];
    let a_in_county = match a.county.as_deref() {
        Some(county) => prioritized_counties.contains(&county),
        None => false
    };
    let b_in_county = match b.county.as_deref() {
        Some(county) => prioritized_counties.contains(&county),
        None => false
    };

    // prioritize `E` legal departments, otherwise sort by water right no
    match (a_has_e, b_has_e, a_in_county, b_in_county) {
        (true, false, _, _) => Ordering::Less,
        (false, true, _, _) => Ordering::Greater,
        (true, true, true, false) => Ordering::Less,
        (true, true, false, true) => Ordering::Greater,
        _ => a.no.cmp(&b.no)
    }
}

fn dedup_cadenza_table(a: &mut CadenzaTableRow, b: &mut CadenzaTableRow) -> bool {
    a.no == b.no
}

fn find_fetched_reports() -> anyhow::Result<Vec<WaterRightNo>> {
    let mut fetched_reports: Vec<WaterRightNo> = Vec::new();

    let report_dir_iter = fs::read_dir(CONFIG.data.reports)?;
    for item in report_dir_iter {
        let item = item?;
        let file_name = item.file_name();
        let file_name = file_name.to_string_lossy();
        if !file_name.ends_with(".pdf") || !file_name.starts_with("rep") {
            continue;
        }

        let water_right_no = file_name
            .split("rep")
            .nth(1)
            .expect("file must start with 'rep'")
            .split(".pdf")
            .next()
            .expect("first element of split always exists")
            .parse()?;
        fetched_reports.push(water_right_no);
    }

    Ok(fetched_reports)
}
