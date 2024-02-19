//! # Export
//! 1. open transaction via [`PostgresClient::transaction`]
//! 2. use [`Transaction::copy_in`] for [batch execution via STDIN](https://www.postgresql.org/docs/current/sql-copy.html)
//! 3. use [`CopyInWriter`] to write rows

use std::fs::File;
use std::io;
use std::io::{stdout, Stdout, Write};

use nlwkn::helper_types::Quantity;
use nlwkn::{LegalDepartmentAbbreviation, UsageLocation, WaterRight, WaterRightNo};
use postgres::{Client as PostgresClient, Transaction};

use crate::postgres_copy::{iter_copy_to, utm_point_copy_to, IsoDate, PostgresCopy};

pub struct InjectionLimit<'il> {
    pub substance: &'il String,
    pub quantity: &'il Quantity
}

struct LogThrough<T> {
    writer: T,
    file: File
}

impl<T> LogThrough<T> {
    fn new(writer: T) -> Self {
        Self {
            writer,
            file: File::create("data/pg-export.log.tsv").unwrap()
        }
    }

    fn into_writer(self) -> T {
        self.writer
    }
}

impl<T> io::Write for LogThrough<T>
where
    T: io::Write
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf)?;
        self.writer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()?;
        self.writer.flush()
    }
}

pub fn water_rights_to_pg(
    pg_client: &mut PostgresClient,
    water_rights: &[WaterRight]
) -> anyhow::Result<()> {
    let mut transaction = pg_client.transaction()?;
    copy_water_rights(&mut transaction, water_rights)?;
    copy_usage_locations(
        &mut transaction,
        water_rights
            .iter()
            .map(|wr| {
                wr.legal_departments
                    .values()
                    .map(|ld| ld.usage_locations.iter().map(|ul| (wr.no, ld.abbreviation, ul)))
                    .flatten()
            })
            .flatten()
    )?;
    transaction.commit()?;
    Ok(())
}

fn copy_water_rights(
    transaction: &mut Transaction,
    water_rights: &[WaterRight]
) -> anyhow::Result<()> {
    let mut writer = transaction.copy_in(
        "
            COPY water_rights.rights
            FROM STDIN
            WITH (
                FORMAT text,
                ENCODING 'utf8'
            )
        "
    )?;

    macro_rules! iso_date {
        ($iso_date_opt:expr) => {
            $iso_date_opt.as_ref().map(|s| IsoDate(s)).copy_to(&mut writer)
        };
    }

    for water_right in water_rights.iter() {
        water_right.no.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        water_right.external_identifier.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        water_right.file_reference.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        iter_copy_to(water_right.legal_departments.keys(), &mut writer)?;
        writer.write(b"\t")?;
        water_right.holder.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        water_right.address.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        water_right.subject.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        water_right.legal_title.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        water_right.status.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        iso_date!(water_right.valid_from)?;
        writer.write(b"\t")?;
        iso_date!(water_right.valid_until)?;
        writer.write(b"\t")?;
        iso_date!(water_right.initially_granted)?;
        writer.write(b"\t")?;
        iso_date!(water_right.last_change)?;
        writer.write(b"\t")?;
        water_right.water_authority.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        water_right.registering_authority.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        water_right.granting_authority.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        water_right.annotation.copy_to(&mut writer)?;
        writer.write(b"\n")?;
    }

    writer.finish()?;
    Ok(())
}

fn copy_usage_locations<'l>(
    transaction: &mut Transaction,
    usage_location: impl Iterator<Item = (WaterRightNo, LegalDepartmentAbbreviation, &'l UsageLocation)>
) -> anyhow::Result<()> {
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
    let mut writer = LogThrough::new(writer);

    for (no, lda, location) in usage_location.take(500) {
        writer.write(b"@DEFAULT")?;
        writer.write(b"\t")?;
        location.no.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        location.serial.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        no.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        lda.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        location.active.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        location.real.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        location.name.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        location.legal_purpose.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        location.map_excerpt.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        location.municipal_area.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        location.county.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        location.land_record.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        location.plot.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        location.maintenance_association.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        location.eu_survey_area.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        location.catchment_area_code.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        location.regulation_citation.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        iter_copy_to((&location.withdrawal_rates).into_iter(), &mut writer)?;
        writer.write(b"\t")?;
        iter_copy_to((&location.pumping_rates).into_iter(), &mut writer)?;
        writer.write(b"\t")?;
        iter_copy_to((&location.injection_rates).into_iter(), &mut writer)?;
        writer.write(b"\t")?;
        iter_copy_to((&location.waste_water_flow_volume).into_iter(), &mut writer)?;
        writer.write(b"\t")?;
        location.river_basin.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        location.groundwater_body.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        location.water_body.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        location.flood_area.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        location.water_protection_area.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        location.dam_target_levels.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        iter_copy_to((&location.fluid_discharge).into_iter(), &mut writer)?;
        writer.write(b"\t")?;
        iter_copy_to((&location.rain_supplement).into_iter(), &mut writer)?;
        writer.write(b"\t")?;
        location.irrigation_area.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        location.ph_values.copy_to(&mut writer)?;
        writer.write(b"\t")?;
        iter_copy_to(
            location.injection_limits.iter().map(|(substance, quantity)| InjectionLimit {
                substance,
                quantity
            }),
            &mut writer
        )?;
        writer.write(b"\t")?;
        match (location.utm_easting, location.utm_northing) {
            (Some(easting), Some(northing)) => utm_point_copy_to(easting, northing, &mut writer)?,
            _ => writer.write(br"\N").map(|_| ())?
        };
        writer.write(b"\n")?;
    }

    writer.flush()?;
    let writer = writer.into_writer();
    writer.finish()?;
    Ok(())
}
