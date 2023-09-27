use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use args::{Args, Format, Lang};
use clap::{Parser};
use indicatif::ProgressBar;
use lazy_static::lazy_static;
use nlwkn::cli::{PROGRESS_STYLE, PROGRESS_UPDATE_INTERVAL, SPINNER_STYLE};
use nlwkn::WaterRight;

use crate::flat_table::{FlatTable, Progress};

mod args;
mod flat_table;

lazy_static! {
    static ref PROGRESS: ProgressBar = ProgressBar::new_spinner();
}

fn main() {
    let Args {
        reports_json,
        lang,
        format,
        out
    } = Args::parse();

    PROGRESS.enable_steady_tick(PROGRESS_UPDATE_INTERVAL);

    PROGRESS.set_style(SPINNER_STYLE.clone());
    PROGRESS.set_message("Reading reports file...");
    let report_json_content =
        fs::read_to_string(&reports_json).expect("could not read reports json");

    let out = match out {
        Some(out) => out,
        None => construct_out_path(reports_json.as_path(), format)
    };

    PROGRESS.set_message("Parsing reports...");
    let water_rights: Vec<WaterRight> =
        serde_json::from_str(&report_json_content).expect("could not parse reports json");

    let mut out_file = File::create(&out).expect("could not create output file");
    let mut out_string = String::new();

    let atomic_counter = AtomicUsize::default();
    match (format, lang) {
        (Format::Csv, Lang::En) => {
            let flat_table: FlatTable<flat_table::marker::En> =
                flat_table::FlatTable::from_water_rights_with_notifier(
                    water_rights.as_slice(),
                    flatten_notifier(&atomic_counter, water_rights.len())
                );
            flat_table
                .fmt_csv(&mut out_string, csv_notifier(&atomic_counter))
                .expect("could not format csv");
        }
        (Format::Csv, Lang::De) => {
            let flat_table: FlatTable<flat_table::marker::De> =
                flat_table::FlatTable::from_water_rights_with_notifier(
                    water_rights.as_slice(),
                    flatten_notifier(&atomic_counter, water_rights.len())
                );
            flat_table
                .fmt_csv(&mut out_string, csv_notifier(&atomic_counter))
                .expect("could not format csv");
        }
    }

    PROGRESS.set_style(SPINNER_STYLE.clone());
    PROGRESS.set_message("Saving results...");
    out_file.write_all(out_string.as_bytes()).expect("could not write to out file");

    PROGRESS.finish_and_clear();
    println!(
        "{} {}",
        console::style("Written results to").magenta(),
        console::style(out.display()).green()
    );
}

fn construct_out_path(reports_json_path: &Path, format: Format) -> PathBuf {
    match (reports_json_path.parent(), reports_json_path.file_stem()) {
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
}

fn flatten_notifier<'ac>(
    atomic_counter: &'ac AtomicUsize,
    water_rights_len: usize
) -> impl Fn(Progress) + 'ac {
    PROGRESS.set_style(PROGRESS_STYLE.clone());
    PROGRESS.set_length(water_rights_len as u64);
    PROGRESS.set_message("Flattening Reports");
    PROGRESS.set_prefix("🪚");
    PROGRESS.set_position(0);
    atomic_counter.swap(0, Ordering::Relaxed);

    |progress: flat_table::Progress| match progress {
        Progress::Flattened(_) => {
            PROGRESS.set_position(atomic_counter.fetch_add(1, Ordering::Relaxed) as u64);
        }
        Progress::Rows(row_count) => {
            PROGRESS.set_message("Updating Keys");
            PROGRESS.set_prefix("🧶");
            PROGRESS.set_length(row_count as u64);
            PROGRESS.set_position(0);
            atomic_counter.swap(0, Ordering::Relaxed);
        }
        Progress::KeyUpdate => {
            PROGRESS.set_position(atomic_counter.fetch_add(1, Ordering::Relaxed) as u64);
        }
    }
}

fn csv_notifier<'ac>(atomic_counter: &'ac AtomicUsize) -> impl Fn() + 'ac {
    PROGRESS.set_style(PROGRESS_STYLE.clone());
    // the length is the same as before
    PROGRESS.set_message("Formatting CSV");
    PROGRESS.set_prefix("📝");
    PROGRESS.set_position(0);
    atomic_counter.swap(0, Ordering::Relaxed);

    || PROGRESS.set_position(atomic_counter.fetch_add(1, Ordering::Relaxed) as u64)
}
