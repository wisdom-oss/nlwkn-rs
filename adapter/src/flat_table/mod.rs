use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter, Write};

use itertools::Itertools;
pub use key::*;
use nlwkn::{WaterRight, WaterRightNo};
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

impl<M> FlatTable<M>
where
    FlatTableKey<M>: AsRef<str>
{
    pub fn from_water_rights(water_rights: &[WaterRight]) -> Self {
        Self::from_water_rights_with_notifier(water_rights, |_| {})
    }

    pub fn from_water_rights_with_notifier(water_rights: &[WaterRight], notifier: impl Fn(WaterRightNo)) -> Self {
        let mut rows = FlatTableRows::new();
        for water_right in water_rights {
            let mut other = util::flatten_water_right(water_right);
            println!("{}", water_right.no);
            rows.append(&mut other);
        }

        let mut keys: BTreeSet<FlatTableKey<M>> = BTreeSet::new();
        for row in rows.iter() {
            for key in row.keys() {
                keys.insert(key.clone());
            }
        }

        FlatTable { values: rows, keys }
    }

    pub fn fmt_csv<W>(&self, w: &mut W) -> std::fmt::Result
    where
        W: Write
    {
        // TODO: replace this when `std` stabilized `intersperse`
        for key in Itertools::intersperse(self.keys.iter().map(AsRef::as_ref), ";") {
            w.write_str(key)?;
        }
        writeln!(w)?;

        for row in self.values.iter() {
            let mut keys = self.keys.iter();
            let Some(first_key) = keys.next()
            else {
                continue;
            };
            match row.get(first_key) {
                Some(v) => write!(w, "{v}")?,
                None => write!(w, "")?
            }

            for key in keys {
                write!(w, ";")?;
                match row.get(key) {
                    Some(v) => write!(w, "{v}")?,
                    None => write!(w, "")?
                }
            }

            writeln!(w)?;
        }

        Ok(())
    }
}
