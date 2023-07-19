use crate::tor::start_socks_proxy;
use crate::xlsx::{CadenzaTable, CadenzaTableRow};
use clap::Parser;
use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tor_rtcompat::PreferredRuntime;

mod browse;
mod tor;
mod xlsx;

static_toml::static_toml! {
    static CONFIG = include_toml!("config.toml");
}

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

    println!("parsing table");
    let mut cadenza_table = CadenzaTable::from_path(&args.xlsx_path).unwrap();
    println!("sorting table");
    cadenza_table.sort_by(sort_cadenza_table);
    println!("deduplicating table");
    cadenza_table.dedup_by(dedup_cadenza_table);

    let client = reqwest::ClientBuilder::new().proxy(reqwest::Proxy::http(format!("socks5://localhost:{}", *tor::SOCKS_PORT).as_str()).unwrap()).build().unwrap();
    fs::create_dir_all("data/reports").unwrap();

    for (i, water_right_no) in cadenza_table.rows().iter().map(|row| row.no).enumerate() {
        println!("fetching water right {}", water_right_no);
        let report_link = browse::fetch_water_right_report(water_right_no).unwrap();
        dbg!(&report_link);

        let full_report_link = format!("{}{}", browse::CADENZA_URL, report_link.split("/cadenza/").skip(1).next().unwrap());
        let pdf_bytes = client.get(&full_report_link).send().await.unwrap().bytes().await.unwrap();
        fs::write(format!("data/reports/rep{}.pdf", water_right_no), pdf_bytes).unwrap();

        if i > 5 {
            break;
        }
    }
}

fn sort_cadenza_table(a: &CadenzaTableRow, b: &CadenzaTableRow) -> Ordering {
    // we want the `E` legal departments first

    // the legal department abbreviations are unreliable, therefore this
    let prio_a = a.legal_department.starts_with("Entnahme");
    let prio_b = b.legal_department.starts_with("Entnahme");

    // prioritize `E` legal departments, otherwise sort by water right no
    match (prio_a, prio_b) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (_, _) => a.no.cmp(&b.no),
    }
}

fn dedup_cadenza_table(a: &mut CadenzaTableRow, b: &mut CadenzaTableRow) -> bool {
    a.no == b.no
}
