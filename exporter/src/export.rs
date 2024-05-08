//! # Export
//! 1. open transaction via [`PostgresClient::transaction`]
//! 2. use [`Transaction::copy_in`] for [batch execution via STDIN](https://www.postgresql.org/docs/current/sql-copy.html)
//! 3. use [`CopyInWriter`] to write rows

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::io::Write as _;

use anyhow::Context;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use itertools::Itertools;
use nlwkn::cadenza::{CadenzaTable, CadenzaTableDiff};
use nlwkn::cli::{PROGRESS_STYLE, SPINNER_STYLE};
use nlwkn::helper_types::Quantity;
use nlwkn::{LegalDepartmentAbbreviation, UsageLocation, WaterRight, WaterRightNo};
use postgres::types::ToSql;
use postgres::{Client as PostgresClient, Transaction};

use crate::postgres_copy::{IterPostgresCopy, Null, PostgresCopy, PostgresCopyContext};
use crate::PROGRESS;

pub struct InjectionLimit<'il> {
    pub substance: &'il String,
    pub quantity: &'il Quantity
}

pub struct UtmPoint {
    pub easting: u64,
    pub northing: u64
}

pub struct IsoDate<'s>(pub &'s str);

pub enum Diff<'d> {
    None,
    AllNew,
    Update(CadenzaTableDiff<'d>)
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct WaterRightStatus {
    no: WaterRightNo,
    id: usize,
    deleted: Option<DateTime<Tz>>
}

impl WaterRightStatus {
    fn from_diff(
        diff: CadenzaTableDiff,
        db_ids: &HashMap<WaterRightNo, usize>
    ) -> anyhow::Result<HashSet<WaterRightStatus>> {
        let CadenzaTableDiff {
            compared,
            added_rights: added,
            removed_rights: removed,
            modified_rights: modified,
            ..
        } = diff;
        let mut statuses = HashSet::new();

        for no in added {
            let id = *db_ids.get(&no).with_context(|| format!("could not find {no} in db ids"))?;
            let deleted = None;
            statuses.insert(WaterRightStatus { no, id, deleted });
        }

        for no in modified {
            let id = *db_ids.get(&no).with_context(|| format!("could not find {no} in db ids"))?;
            let deleted = None;
            statuses.insert(WaterRightStatus { no, id, deleted });
        }

        let deleted = compared.1.context("could not get deleted timestamp from diff")?;
        for no in removed {
            let id = *db_ids.get(&no).with_context(|| format!("could not find {no} in db ids"))?;
            let deleted = Some(deleted.clone());
            statuses.insert(WaterRightStatus { no, id, deleted });
        }

        Ok(statuses)
    }
}

pub fn water_rights_to_pg<'d>(
    pg_client: &mut PostgresClient,
    water_rights: &[WaterRight],
    diff: Diff
) -> anyhow::Result<()> {
    let mut transaction = pg_client.transaction()?;
    copy_water_rights(&mut transaction, water_rights)?;
    let usage_locations = water_rights
        .iter()
        .flat_map(|wr| {
            wr.legal_departments
                .values()
                .flat_map(|ld| ld.usage_locations.iter().map(|ul| (wr.no, ld.abbreviation, ul)))
        })
        .collect();
    let db_ids = fetch_water_right_db_ids(&mut transaction)?;
    copy_usage_locations(&mut transaction, usage_locations, &db_ids)?;
    match diff {
        Diff::None => (),
        Diff::AllNew => {
            let statuses = db_ids.into_iter().map(|(no, id)| WaterRightStatus {
                no,
                id,
                deleted: None
            });
            copy_current_rights(&mut transaction, statuses)?;
        }
        Diff::Update(diff) => {
            let statuses = WaterRightStatus::from_diff(diff, &db_ids)?;
            update_current_rights(&mut transaction, statuses)?;
        }
    }
    PROGRESS.set_style(SPINNER_STYLE.clone());
    PROGRESS.set_message("Committing transaction to database...");
    transaction.commit()?;
    Ok(())
}

macro_rules! interleave_tabs {
    // Base case: when there's only one expression left, execute it without adding a tab after
    ($writer:expr; $expr:expr) => {
        $expr // Execute the last expression
    };

    // Match any expression followed by a comma, and then recursively call for the rest
    ($writer:expr; $expr:expr; $($rest:expr);+ $(;)?) => {
        $expr; // Execute the first expression
        $writer.write_all(b"\t")?; // Write a tab.
        interleave_tabs!($writer; $($rest);*); // Recursively process the remaining expressions
    };
}

fn copy_water_rights(
    transaction: &mut Transaction,
    water_rights: &[WaterRight]
) -> anyhow::Result<()> {
    PROGRESS.set_style(PROGRESS_STYLE.clone());
    PROGRESS.set_length(water_rights.len() as u64);
    PROGRESS.set_message("Copying water rights...");
    PROGRESS.set_prefix("🐘");
    PROGRESS.set_position(0);

    #[cfg_attr(feature = "file-log", allow(unused_mut))]
    let mut writer = transaction.copy_in(
        "
            COPY water_rights.rights
            FROM STDIN
            WITH (
                FORMAT text,
                DEFAULT '@DEFAULT',
                ENCODING 'utf8'
            )
        "
    )?;
    #[cfg(feature = "file-log")]
    let mut writer = log_through::LogThrough::new(writer, "rights.export").prepare_rights()?;

    macro_rules! iso_date {
        ($iso_date_opt:expr) => {
            $iso_date_opt
                .as_ref()
                .map(|s| IsoDate(s))
                .copy_to(&mut writer, PostgresCopyContext::default())
        };
    }

    // PostgresCopyContext implements Copy,
    // so this will be a new context for each call
    let ctx = PostgresCopyContext::default();
    for water_right in water_rights.iter() {
        interleave_tabs! {
            writer;
            writer.write_all(b"@DEFAULT")?; // for id
            water_right.no.copy_to(&mut writer, ctx)?;
            water_right.external_identifier.copy_to(&mut writer, ctx)?;
            water_right.file_reference.copy_to(&mut writer, ctx)?;
            water_right.legal_departments.keys().copy_to(&mut writer, ctx)?;
            water_right.holder.copy_to(&mut writer, ctx)?;
            water_right.address.copy_to(&mut writer, ctx)?;
            water_right.subject.copy_to(&mut writer, ctx)?;
            water_right.legal_title.copy_to(&mut writer, ctx)?;
            water_right.status.copy_to(&mut writer, ctx)?;
            iso_date!(water_right.valid_from)?;
            iso_date!(water_right.valid_until)?;
            iso_date!(water_right.initially_granted)?;
            iso_date!(water_right.last_change)?;
            water_right.water_authority.copy_to(&mut writer, ctx)?;
            water_right.registering_authority.copy_to(&mut writer, ctx)?;
            water_right.granting_authority.copy_to(&mut writer, ctx)?;
            water_right.annotation.copy_to(&mut writer, ctx)?;
        }
        writeln!(writer)?;
        PROGRESS.inc(1);
    }

    #[cfg(feature = "file-log")]
    let writer = writer.into_writer()?;
    writer.finish()?;
    Ok(())
}

fn fetch_water_right_db_ids(
    transaction: &mut Transaction
) -> anyhow::Result<HashMap<WaterRightNo, usize>> {
    let rows = transaction.query("SELECT id, water_right_number FROM water_rights.rights", &[
    ])?;
    let mut db_ids = HashMap::with_capacity(rows.len());
    for row in rows {
        let (id, no): (i64, i64) = (row.get("id"), row.get("water_right_number"));
        // this conversion should be safe as we only store unsigned integers in
        // our db for water right numbers and ids
        let (id, no) = (id as usize, no as WaterRightNo);

        match db_ids.get(&no) {
            None => db_ids.insert(no, id),
            // we use serial type, therefore if id(a) < id(b) => t(a) < t(b)
            Some(other_id) if other_id < &id => db_ids.insert(no, id),
            Some(_) => None
        };
    }

    Ok(db_ids)
}

fn copy_usage_locations(
    transaction: &mut Transaction,
    usage_locations: Vec<(WaterRightNo, LegalDepartmentAbbreviation, &UsageLocation)>,
    db_ids: &HashMap<WaterRightNo, usize>
) -> anyhow::Result<()> {
    PROGRESS.set_style(PROGRESS_STYLE.clone());
    PROGRESS.set_length(usage_locations.len() as u64);
    PROGRESS.set_message("Copying usage locations...");
    PROGRESS.set_prefix("🐘");
    PROGRESS.set_position(0);

    #[cfg_attr(feature = "file-log", allow(unused_mut))]
    let mut writer = transaction.copy_in(
        "
            COPY water_rights.usage_locations
            FROM STDIN
            WITH (
                FORMAT text,
                DEFAULT '@DEFAULT',
                ENCODING 'utf8'
            )
        "
    )?;
    #[cfg(feature = "file-log")]
    let mut writer =
        log_through::LogThrough::new(writer, "usage_locations.export").prepare_usage_locations()?;

    let ctx = PostgresCopyContext::default();
    for (no, lda, location) in usage_locations {
        let water_right_no =
            db_ids.get(&no).with_context(|| format!("could not find {no} in db ids"))?;
        interleave_tabs! {
            writer;
            writer.write_all(b"@DEFAULT")?;
            location.no.copy_to(&mut writer, ctx)?;
            location.serial.copy_to(&mut writer, ctx)?;
            water_right_no.copy_to(&mut writer, ctx)?;
            lda.copy_to(&mut writer, ctx)?;
            location.active.copy_to(&mut writer, ctx)?;
            location.real.copy_to(&mut writer, ctx)?;
            location.name.copy_to(&mut writer, ctx)?;
            location.legal_purpose.copy_to(&mut writer, ctx)?;
            location.map_excerpt.copy_to(&mut writer, ctx)?;
            location.municipal_area.copy_to(&mut writer, ctx)?;
            location.county.copy_to(&mut writer, ctx)?;
            location.land_record.copy_to(&mut writer, ctx)?;
            location.plot.copy_to(&mut writer, ctx)?;
            location.maintenance_association.copy_to(&mut writer, ctx)?;
            location.eu_survey_area.copy_to(&mut writer, ctx)?;
            location.catchment_area_code.copy_to(&mut writer, ctx)?;
            location.regulation_citation.copy_to(&mut writer, ctx)?;
            location.withdrawal_rates.copy_to(&mut writer, ctx)?;
            location.pumping_rates.copy_to(&mut writer, ctx)?;
            location.injection_rates.copy_to(&mut writer, ctx)?;
            location.waste_water_flow_volume.copy_to(&mut writer, ctx)?;
            location.river_basin.copy_to(&mut writer, ctx)?;
            location.groundwater_body.copy_to(&mut writer, ctx)?;
            location.water_body.copy_to(&mut writer, ctx)?;
            location.flood_area.copy_to(&mut writer, ctx)?;
            location.water_protection_area.copy_to(&mut writer, ctx)?;
            location.dam_target_levels.copy_to(&mut writer, ctx)?;
            location.fluid_discharge.copy_to(&mut writer, ctx)?;
            location.rain_supplement.copy_to(&mut writer, ctx)?;
            location.irrigation_area.copy_to(&mut writer, ctx)?;
            location.ph_values.copy_to(&mut writer, ctx)?;
            location
                .injection_limits
                .iter()
                .map(|(substance, quantity)| InjectionLimit {
                    substance,
                    quantity
                })
                .copy_to(&mut writer, ctx)?;
            match (location.utm_easting, location.utm_northing) {
                (Some(easting), Some(northing)) => Some(UtmPoint { easting, northing }),
                _ => None
            }
            .copy_to(&mut writer, ctx)?;
        }
        writeln!(writer)?;
        PROGRESS.inc(1);
    }

    #[cfg(feature = "file-log")]
    let writer = writer.into_writer()?;
    writer.finish()?;
    Ok(())
}

fn copy_current_rights(
    transaction: &mut Transaction,
    water_right_statuses: impl ExactSizeIterator<Item = WaterRightStatus>
) -> anyhow::Result<()> {
    PROGRESS.set_style(PROGRESS_STYLE.clone());
    PROGRESS.set_length(water_right_statuses.len() as u64);
    PROGRESS.set_message("Copying current rights...");
    PROGRESS.set_prefix("🐘");
    PROGRESS.set_position(0);

    let mut writer = transaction.copy_in(
        "
            COPY water_rights.current_rights
            FROM STDIN
            WITH (
                FORMAT text,
                DEFAULT '@DEFAULT',
                ENCODING 'utf8'
            )
        "
    )?;

    let ctx = PostgresCopyContext::default();
    for status in water_right_statuses {
        interleave_tabs! {
            writer;
            status.no.copy_to(&mut writer, ctx)?;
            status.id.copy_to(&mut writer, ctx)?;
            status.deleted.copy_to(&mut writer, ctx)?;
        }
        writeln!(&mut writer)?;
        PROGRESS.inc(1);
    }

    writer.finish()?;
    Ok(())
}

fn update_current_rights(
    transaction: &mut Transaction,
    water_right_statuses: HashSet<WaterRightStatus>
) -> anyhow::Result<()> {
    PROGRESS.set_style(PROGRESS_STYLE.clone());
    PROGRESS.set_length(water_right_statuses.len() as u64);
    PROGRESS.set_message("Updating current rights...");
    PROGRESS.set_prefix("🐘");
    PROGRESS.set_position(0);

    enum Element {
        Int(i64),
        DateTimeOpt(Option<DateTime<Utc>>)
    }

    let batch_size = 10_000;
    for chunk in water_right_statuses.into_iter().chunks(batch_size).into_iter() {
        let mut query = String::from("INSERT INTO water_rights.current_rights VALUES\n");
        let mut params = Vec::with_capacity(chunk.try_len().unwrap_or_default());

        let mut handle = |query: &mut String, i: usize, status: WaterRightStatus| {
            let idx = i * 3;
            write!(query, "(${}, ${}, ${})", idx + 1, idx + 2, idx + 3)
                .expect("infallible on string");
            params.push(Element::Int(status.no as i64));
            params.push(Element::Int(status.id as i64));
            params.push(Element::DateTimeOpt(status.deleted.map(|dt| dt.to_utc())));
            PROGRESS.inc(1);
        };

        // handle first element
        let mut chunk_iter = chunk.enumerate();
        if let Some((i, status)) = chunk_iter.next() {
            handle(&mut query, i, status);
        }

        // handle the rest, postgres cannot handle trailing commas in sql
        for (i, status) in chunk_iter {
            writeln!(&mut query, ",").expect("infallible on string");
            handle(&mut query, i, status);
        }

        let params: Vec<_> = params
            .iter()
            .map(|el| match el {
                Element::Int(i) => i as &(dyn ToSql + Sync),
                Element::DateTimeOpt(s) => s as &(dyn ToSql + Sync)
            })
            .collect();

        writeln!(
            &mut query,
            "{}\n{}",
            "ON CONFLICT (water_right_number) DO UPDATE",
            "SET internal_id = EXCLUDED.internal_id, deleted = EXCLUDED.deleted"
        )
        .expect("infallible on string");

        transaction.execute(&query, &params)?;
    }

    Ok(())
}

#[cfg(feature = "file-log")]
mod log_through {
    use std::fs::File;
    use std::io;
    use std::io::Write;

    pub struct LogThrough<T> {
        writer: T,
        file: File
    }

    impl<T> LogThrough<T>
    where
        T: io::Write
    {
        pub fn new(writer: T, filename: &str) -> Self {
            Self {
                writer,
                file: File::create(format!("data/{filename}.log.tsv")).unwrap()
            }
        }

        pub fn into_writer(mut self) -> io::Result<T> {
            self.flush()?;
            Ok(self.writer)
        }

        pub fn log(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.file.write(buf)
        }

        pub fn prepare_rights(mut self) -> io::Result<Self> {
            self.log(
                concat!(
                    "id\t",
                    "external_identifier\t",
                    "file_reference\t",
                    "legal_departments\t",
                    "holder\t",
                    "address\t",
                    "subject\t",
                    "legal_title\t",
                    "status\t",
                    "valid_from\t",
                    "valid_until\t",
                    "initially_granted\t",
                    "last_change\t",
                    "water_authority\t",
                    "granting_authority\t",
                    "annotation\n"
                )
                .as_bytes()
            )?;
            Ok(self)
        }

        pub fn prepare_usage_locations(mut self) -> io::Result<Self> {
            self.log(
                concat!(
                    "id\t",
                    "no\t",
                    "serial\t",
                    "water_right\t",
                    "legal_department\t",
                    "active\t",
                    "real\t",
                    "name\t",
                    "legal_purpose\t",
                    "map_excerpt\t",
                    "municipal_area\t",
                    "county\t",
                    "land_record\t",
                    "plot\t",
                    "maintenance_association\t",
                    "eu_survey_area\t",
                    "catchment_area_code\t",
                    "regulation_citation\t",
                    "withdrawal_rates\t",
                    "pumping_rates\t",
                    "injection_rates\t",
                    "waste_water_flow_volume\t",
                    "river_basin\t",
                    "groundwater_body\t",
                    "water_body\t",
                    "flood_area\t",
                    "water_protection_area\t",
                    "dam_target_levels\t",
                    "fluid_discharge\t",
                    "rain_supplement\t",
                    "irrigation_area\t",
                    "ph_values\t",
                    "injection_limits\t",
                    "location\n"
                )
                .as_bytes()
            )?;
            Ok(self)
        }
    }

    impl<T> io::Write for LogThrough<T>
    where
        T: io::Write
    {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.file.write_all(buf)?;
            self.writer.write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.file.flush()?;
            self.writer.flush()
        }
    }
}
