//! # Export
//! 1. open transaction via [`PostgresClient::transaction`]
//! 2. use [`Transaction::copy_in`] for [batch execution via STDIN](https://www.postgresql.org/docs/current/sql-copy.html)
//! 3. use [`CopyInWriter`] to write rows

use std::io;
use std::io::{Stdout, stdout, Write};

use nlwkn::WaterRight;
use postgres::{Client as PostgresClient, Transaction};

use crate::postgres_copy::{IsoDate, iter_copy_to, PostgresCopy};

struct LogThrough<T> {
    writer: T,
    stdout: Stdout
}

impl<T> LogThrough<T> {
    fn new(writer: T) -> Self {
        Self {
            writer,
            stdout: stdout()
        }
    }

    fn into_writer(self) -> T {
        self.writer
    }
}

impl<T> io::Write for LogThrough<T> where T: io::Write {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stdout.write(buf)?;
        self.writer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stdout.flush()?;
        self.writer.flush()
    }
}

pub fn water_rights_to_pg(
    pg_client: &mut PostgresClient,
    water_rights: &[WaterRight]
) -> anyhow::Result<()> {
    let mut transaction = pg_client.transaction()?;
    copy_water_rights(&mut transaction, water_rights)?;
    // TODO: do the usage locations
    transaction.commit()?;
    Ok(())
}

fn copy_water_rights(
    transaction: &mut Transaction,
    water_rights: &[WaterRight]
) -> anyhow::Result<()> {
    let mut writer =
        transaction.copy_in("COPY water_rights.rights FROM STDIN WITH ENCODING 'utf8'")?;
    let mut writer = LogThrough::new(writer);

    macro_rules! iso_date {
        ($iso_date_opt:expr) => {
            $iso_date_opt.as_ref().map(|s| IsoDate(s)).copy_to(&mut writer)
        };
    }

    // FIXME: only handles first one
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

    writer.flush()?;
    let writer = writer.into_writer();
    writer.finish()?;
    Ok(())
}
