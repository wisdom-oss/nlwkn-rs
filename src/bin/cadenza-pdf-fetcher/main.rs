use crate::tor::start_socks_proxy;
use crate::xlsx::{CadenzaTable, CadenzaTableRow};
use clap::Parser;
use console::Alignment;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use nlwkn_rs::cli::ProgressBarGuard;
use nlwkn_rs::WaterRightNo;
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fmt::Write;

use std::path::PathBuf;

use std::fs;
use std::time::Duration;

mod browse;
mod tor;
mod xlsx;

static_toml::static_toml! {
    static CONFIG = include_toml!("config.toml");
}

const PRINT_PADDING: usize = 9;

/// NLWKN Water Right Webcrawler
#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// Path to cadenza-provided xlsx file
    xlsx_path: PathBuf,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let _proxy_handle = tokio::spawn(start_socks_proxy());

    let mut cadenza_table = {
        let _pb = ProgressBarGuard::new_wait_spinner("Parsing table...");
        CadenzaTable::from_path(&args.xlsx_path).expect("could not parse table")
    };

    {
        let _pb = ProgressBarGuard::new_wait_spinner("Sorting table...");
        cadenza_table.sort_by(sort_cadenza_table);
    }

    {
        let _pb = ProgressBarGuard::new_wait_spinner("Deduplicating table...");
        cadenza_table.dedup_by(dedup_cadenza_table);
    }

    let client = reqwest::ClientBuilder::new()
        .proxy(
            reqwest::Proxy::http(format!("socks5://localhost:{}", *tor::SOCKS_PORT).as_str())
                .expect("proxy schema invalid"),
        )
        .build()
        .expect("cannot build GET client");

    {
        let _pb = ProgressBarGuard::new_wait_spinner("Waiting for TOR proxy...");
        while client.get(browse::CADENZA_URL).send().await.is_err() {
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }

    fs::create_dir_all(CONFIG.data.reports).expect("could not create necessary directories");

    let mut fetched_reports = {
        let _pb = ProgressBarGuard::new_wait_spinner("Fetching already downloaded reports...");
        BTreeSet::from_iter(
            find_fetched_reports()
                .expect("could not find already fetched reports")
                .iter()
                .copied(),
        )
    };

    let mut unfetched_reports = Vec::new();

    let progress = ProgressBar::new(cadenza_table.rows().len() as u64)
        .with_style(
            ProgressStyle::with_template(
                "{msg:.cyan}  {wide_bar:.magenta/.234}  {human_pos:.magenta}{slash:.magenta}{human_len:.magenta} {prefix:.cyan}"
            )
                .expect("is valid schema")
                .with_key("slash", |_: &ProgressState, w: &mut dyn Write| write!(w, "/").expect("write should work here"))
                .progress_chars("━ ━")
        )
        .with_message("Fetching Reports");
    progress.enable_steady_tick(Duration::from_secs(1));

    'wr_loop: for water_right_no in cadenza_table.rows().iter().map(|row| row.no) {
        if fetched_reports.contains(&water_right_no) {
            progress.println(format!(
                "{} {}, already fetched",
                console::pad_str(
                    console::style("Skipped").green().to_string().as_str(),
                    PRINT_PADDING,
                    Alignment::Right,
                    None,
                ),
                water_right_no
            ));
            progress.inc(1);
            continue;
        }

        progress.set_prefix(water_right_no.to_string());
        progress.tick();

        for retry in 1..=(CONFIG.cadenza.retries as u32) {
            let fetched = fetch(water_right_no, &client).await;
            match fetched {
                Ok(_) => {
                    progress.println(format!(
                        "{} {}",
                        console::pad_str(
                            console::style("Fetched").green().to_string().as_str(),
                            PRINT_PADDING,
                            Alignment::Right,
                            None,
                        ),
                        water_right_no
                    ));
                    progress.inc(1);
                    fetched_reports.insert(water_right_no);
                    continue 'wr_loop;
                }

                Err(err) => {
                    progress.println(format!(
                        "{} failed to fetch, {}",
                        console::pad_str(
                            console::style("Error").red().to_string().as_str(),
                            PRINT_PADDING,
                            Alignment::Right,
                            None,
                        ),
                        err
                    ));

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
        progress.println(format!(
            "{} exceeded amount of retries, will skip {water_right_no}",
            console::pad_str(
                console::style("Warning").yellow().to_string().as_str(),
                PRINT_PADDING,
                Alignment::Right,
                None,
            )
        ));
        progress.inc(1);
    }

    progress.finish_and_clear();
    match unfetched_reports.is_empty() {
        false => println!(
            "{}, could not fetch: {}",
            console::style("Fetching done").magenta(),
            unfetched_reports
                .iter()
                .map(|no| no.to_string())
                .collect::<Vec<String>>()
                .join(", ")
        ),
        true => println!("{}", console::style("Fetched all reports").magenta()),
    }
}

async fn fetch(water_right_no: WaterRightNo, client: &reqwest::Client) -> anyhow::Result<()> {
    let report_link = browse::fetch_water_right_report(water_right_no)?;

    let full_report_link = format!(
        "{}{}",
        browse::CADENZA_URL,
        report_link
            .split("/cadenza/")
            .nth(1)
            .ok_or(anyhow::Error::msg("report link has no '/cadenza/' in path"))?
    );
    let pdf_bytes = client.get(&full_report_link).send().await?.bytes().await?;
    fs::write(
        format!("{}/rep{}.pdf", CONFIG.data.reports, water_right_no),
        pdf_bytes,
    )?;

    Ok(())
}

fn sort_cadenza_table(a: &CadenzaTableRow, b: &CadenzaTableRow) -> Ordering {
    // we want the `E` legal departments first

    // the legal department abbreviations are unreliable, therefore this
    let a_has_e = a.legal_department.starts_with("Entnahme");
    let b_has_e = b.legal_department.starts_with("Entnahme");

    // also prioritize some counties
    let prioritized_counties = vec!["Aurich", "Wittmund", "Friesland", "Leer"];
    let a_in_county = match a.county.as_deref() {
        Some(county) => prioritized_counties.contains(&county),
        None => false,
    };
    let b_in_county = match b.county.as_deref() {
        Some(county) => prioritized_counties.contains(&county),
        None => false,
    };

    // prioritize `E` legal departments, otherwise sort by water right no
    match (a_has_e, b_has_e, a_in_county, b_in_county) {
        (true, false, _, _) => Ordering::Less,
        (false, true, _, _) => Ordering::Greater,
        (true, true, true, false) => Ordering::Less,
        (true, true, false, true) => Ordering::Greater,
        _ => a.no.cmp(&b.no),
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
