use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use itertools::Itertools;
pub use key::*;
use nlwkn::{WaterRight, WaterRightNo};
use rayon::prelude::*;
pub use value::*;

use crate::flat_table::key::FlatTableKey;
use crate::flat_table::value::FlatTableValue;

mod key;
mod util;
mod value;

pub struct FlatTable<M> {
    values: FlatTableRows<M>,
    keys: BTreeSet<FlatTableKey<M>>
}

pub type FlatTableRows<M> = Vec<FlatTableRow<M>>;
pub type FlatTableRow<M> = BTreeMap<FlatTableKey<M>, FlatTableValue>;

#[derive(Debug)]
pub enum Progress {
    Flattened(WaterRightNo),
    Rows(usize),
    KeyUpdate
}

impl<M> FlatTable<M>
where
    FlatTableKey<M>: AsRef<str>,
    M: Send + Sync
{
    pub fn from_water_rights_with_notifier(
        water_rights: &[WaterRight],
        notifier: impl Fn(Progress) + Send + Sync
    ) -> Self {
        let rows: FlatTableRows<M> = water_rights
            .par_iter()
            .flat_map(|water_right| {
                let other = util::flatten_water_right(water_right);
                notifier(Progress::Flattened(water_right.no));
                other
            })
            .collect();

        notifier(Progress::Rows(rows.len()));
        let mut keys: BTreeSet<FlatTableKey<M>> = BTreeSet::new();
        for row in rows.iter() {
            for key in row.keys() {
                keys.insert(key.clone());
            }

            // first value is the water right number, no matter how it is named now
            notifier(Progress::KeyUpdate)
        }

        FlatTable { values: rows, keys }
    }

    pub fn fmt_csv<W>(&self, w: &mut W, notifier: impl Fn() + Send + Sync) -> std::fmt::Result
    where
        W: Write
    {
        // TODO: replace this when `std` stabilized `intersperse`
        for key in Itertools::intersperse(self.keys.iter().map(AsRef::as_ref), ";") {
            w.write_str(key)?;
        }
        writeln!(w)?;

        let rows: Vec<_> = self
            .values
            .par_iter()
            .flat_map(|row| {
                let mut keys = self.keys.iter();
                let Some(first_key) = keys.next()
                else {
                    return None;
                };
                let mut row_string = String::new();
                if let Some(v) = row.get(first_key) {
                    write!(row_string, "{v}").expect("never fails on string")
                }

                for key in keys {
                    row_string.push(';');
                    if let Some(v) = row.get(key) {
                        write!(row_string, "{v}").expect("never fails on string");
                    }
                }

                writeln!(row_string).expect("never fails on string");
                notifier();
                Some(row_string)
            })
            .collect();

        for row in rows {
            w.write_str(&row)?;
        }

        Ok(())
    }
}
