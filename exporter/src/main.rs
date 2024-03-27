use std::path::PathBuf;
use std::str::FromStr;
use std::{env, fs};

use clap::Parser;
use indicatif::ProgressBar;
use lazy_static::lazy_static;
use nlwkn::cli::{PROGRESS_UPDATE_INTERVAL, SPINNER_STYLE};
use nlwkn::WaterRight;
use postgres::{Client as PostgresClient, NoTls};
use static_toml::static_toml;

mod export;
mod postgres_copy;

const INIT_QUERY: &str = include_str!("../../target/resources/init.sql");

static_toml! {
    static CONFIG = include_toml!("config.toml");
}

lazy_static! {
    static ref PROGRESS: ProgressBar = ProgressBar::new_spinner();
}

/// NLWKN Water Right DB Exporter
#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// Path to reports JSON file
    pub reports_json: PathBuf,

    #[clap(flatten)]
    pub pg_args: PostgresArgs
}

#[derive(Debug, Parser)]
struct PostgresArgs {
    /// Postgres username
    #[arg(long)]
    pub user: Option<String>,

    /// Postgres password
    #[arg(long)]
    pub password: Option<String>,

    /// Postgres host
    #[arg(long)]
    pub host: Option<String>,

    /// Postgres port
    #[arg(long)]
    pub port: Option<u16>
}

fn main() -> anyhow::Result<()> {
    let Args {
        reports_json,
        pg_args
    } = Args::parse();

    PROGRESS.enable_steady_tick(PROGRESS_UPDATE_INTERVAL);

    PROGRESS.set_style(SPINNER_STYLE.clone());
    PROGRESS.set_message("Setting up postgres client...");
    let mut pg_client = setup_pg_client(pg_args)?;
    PROGRESS.set_message("Initializing database...");
    pg_client.batch_execute(INIT_QUERY)?;

    PROGRESS.set_message("Reading reports file...");
    let water_rights = fs::read_to_string(reports_json)?;
    PROGRESS.set_message("Parsing reports...");
    let water_rights: Vec<WaterRight> = serde_json::from_str(&water_rights)?;
    export::water_rights_to_pg(&mut pg_client, &water_rights)?;

    PROGRESS.finish_and_clear();
    println!(
        "{}",
        console::style("Successfully exported water rights to database").green()
    );
    Ok(())
}

fn setup_pg_client(
    PostgresArgs {
        user,
        password,
        host,
        port
    }: PostgresArgs
) -> anyhow::Result<PostgresClient> {
    let mut pg_config = PostgresClient::configure();
    pg_config.application_name(concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_BIN_NAME")));
    pg_config.dbname(CONFIG.postgres.database);
    env::var("PG_USER").ok().or(user).map(|v| pg_config.user(&v));
    env::var("PG_PASS").ok().or(password).map(|v| pg_config.password(&v));
    env::var("PG_HOST").ok().or(host).map(|v| pg_config.host(&v));
    env::var("PG_PORT")
        .ok()
        .and_then(|v| u16::from_str(&v).ok())
        .or(port)
        .map(|v| pg_config.port(v));
    Ok(pg_config.connect(NoTls)?)
}
