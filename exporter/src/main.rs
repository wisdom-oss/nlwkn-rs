use std::path::PathBuf;
use std::str::FromStr;
use std::{env, fs};

use clap::Parser;
use nlwkn::WaterRight;
use postgres::{Client as PostgresClient, NoTls};
use static_toml::static_toml;

mod postgres_copy;

const INIT_QUERY: &str = include_str!("../../target/resources/init.sql");

static_toml! {
    static CONFIG = include_toml!("config.toml");
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

    let mut pg_client = setup_pg_client(pg_args)?;
    pg_client.batch_execute(INIT_QUERY)?;

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
