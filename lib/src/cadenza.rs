use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::path::PathBuf;

use calamine::{DataType, RangeDeserializerBuilder, Reader, Xlsx};
use indexmap::IndexSet;
use serde::{Deserialize, Deserializer, Serialize};

use crate::util::StringOption;
use crate::WaterRightNo;

#[derive(Debug, Serialize)]
pub struct CadenzaTable {
    path: PathBuf,
    rows: Vec<CadenzaTableRow>
}

/// Inner representation of a row in a [`CadenzaTable`].
///
/// This struct should be used primarily by the library itself or for complex
/// custom manipulations where direct access to row values is necessary.
/// It manages the low-level representation and operations on data within a row.
/// For faster but less precise equality checks or hashing, use
/// [`CadenzaTableRow`], which dereferences to this type for basic operations.
///
/// Note: The [`CadenzaTable::diff`] method utilizes the full equality checks
/// provided by this type to ensure accurate comparisons between rows.
#[cfg_attr(test, derive(Default))]
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct CadenzaTableRowInner {
    #[serde(rename = "Wasserrecht Nr.")]
    pub no: WaterRightNo,

    #[serde(rename = "Rechtsinhaber")]
    pub rights_holder: Option<String>,

    #[serde(rename = "Gültig Bis", deserialize_with = "deserialize_date", default)]
    pub valid_until: Option<String>,

    #[serde(rename = "Zustand")]
    pub status: Option<String>,

    #[serde(rename = "Gültig Ab", deserialize_with = "deserialize_date", default)]
    pub valid_from: Option<String>,

    #[deprecated]
    #[serde(rename = "Rechtsabteilungen")]
    pub legal_departments: Option<String>,

    #[serde(rename = "Rechtstitel")]
    pub legal_title: Option<String>,

    #[serde(rename = "Wasserbehoerde")]
    pub water_authority: Option<String>,

    #[serde(rename = "Erteilende Behoerde")]
    pub granting_authority: Option<String>,

    #[serde(
        rename = "Aenderungsdatum",
        deserialize_with = "deserialize_date",
        default
    )]
    pub date_of_change: Option<String>,

    #[serde(rename = "Aktenzeichen")]
    pub file_reference: Option<String>,

    #[serde(rename = "Externe Kennung")]
    pub external_identifier: Option<String>,

    #[serde(rename = "Betreff")]
    pub subject: Option<String>,

    #[serde(rename = "Adresse")]
    pub address: Option<String>,

    #[serde(rename = "Nutzungsort Nr.")]
    pub usage_location_no: u64,

    #[serde(rename = "Nutzungsort")]
    pub usage_location: Option<String>,

    #[serde(rename = "Rechtsabteilung")]
    pub legal_department: String,

    #[serde(rename = "Rechtszweck")]
    pub legal_purpose: Option<String>,

    #[serde(rename = "Landkreis")]
    pub county: Option<String>,

    #[serde(rename = "Flussgebiet")]
    pub river_basin: Option<String>,

    #[serde(rename = "Grundwasserkörper")]
    pub groundwater_body: Option<String>,

    #[serde(rename = "Überschwemmungsgebiet")]
    pub flood_area: Option<String>,

    #[serde(rename = "Wasserschutzgebiet")]
    pub water_protection_area: Option<String>,

    #[serde(rename = "UTM-Rechtswert", deserialize_with = "zero_as_none")]
    pub utm_easting: Option<u64>,

    #[serde(rename = "UTM-Hochwert", deserialize_with = "zero_as_none")]
    pub utm_northing: Option<u64>
}

/// Represents a row in a [`CadenzaTable`].
///
/// This is the primary type used for interacting with rows in the table
/// throughout the codebase.
/// It wraps a [`CadenzaTableRowInner`] which holds the actual data values,
/// while providing an easier and more intuitive interface for most operations.
///
/// Implements [`Deref`] targeting [`CadenzaTableRowInner`] to facilitate direct
/// access to inner values.
/// It's designed to be transparent during serialization and testing, mirroring
/// the behavior and attributes of its inner type.
#[derive(Debug, Deserialize, Serialize, Eq)]
#[cfg_attr(test, derive(Default))]
#[repr(transparent)]
#[serde(transparent)]
pub struct CadenzaTableRow(CadenzaTableRowInner);

impl CadenzaTable {
    pub fn from_path(path: impl Into<PathBuf>) -> anyhow::Result<CadenzaTable> {
        let path = path.into();
        let mut workbook: Xlsx<_> = calamine::open_workbook(&path)?;
        let worksheets = workbook.worksheets();
        let (_, range) = worksheets.first().ok_or(anyhow::Error::msg("workbook empty"))?;
        let iter = RangeDeserializerBuilder::new().has_headers(true).from_range(range)?;
        let rows: Result<Vec<CadenzaTableRow>, _> = iter.collect();
        Ok(CadenzaTable { path, rows: rows? })
    }

    #[inline]
    pub fn rows(&self) -> &[CadenzaTableRow] {
        &self.rows
    }

    /// Iterator of all water right numbers.
    ///
    /// The items of this iterator are unique and ordered by the internal rows,
    /// use [`sort_by`] to sort the table if necessary.
    pub fn water_right_no_iter(&self) -> impl ExactSizeIterator<Item = WaterRightNo> {
        self.rows().iter().map(|row| row.no).collect::<IndexSet<WaterRightNo>>().into_iter()
    }

    pub fn sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&CadenzaTableRow, &CadenzaTableRow) -> Ordering
    {
        let slice = self.rows.as_mut_slice();
        slice.sort_by(compare);
    }

    pub fn dedup_by<F>(&mut self, same_bucket: F)
    where
        F: FnMut(&mut CadenzaTableRow, &mut CadenzaTableRow) -> bool
    {
        self.rows.dedup_by(same_bucket);
    }

    pub fn sanitize(&mut self) {
        #[allow(deprecated)]
        for row in self.rows.iter_mut().map(|r| &mut r.0) {
            row.rights_holder = row.rights_holder.take().sanitize();
            row.valid_until = row.valid_until.take().sanitize();
            row.status = row.status.take().sanitize();
            row.valid_from = row.valid_from.take().sanitize();
            row.legal_departments = row.legal_departments.take().sanitize();
            row.legal_title = row.legal_title.take().sanitize();
            row.water_authority = row.water_authority.take().sanitize();
            row.granting_authority = row.granting_authority.take().sanitize();
            row.date_of_change = row.date_of_change.take().sanitize();
            row.file_reference = row.file_reference.take().sanitize();
            row.external_identifier = row.external_identifier.take().sanitize();
            row.subject = row.subject.take().sanitize();
            row.address = row.address.take().sanitize();
            row.usage_location = row.usage_location.take().sanitize();
            row.legal_purpose = row.legal_purpose.take().sanitize();
            row.county = row.county.take().sanitize();
            row.river_basin = row.river_basin.take().sanitize();
            row.groundwater_body = row.groundwater_body.take().sanitize();
            row.flood_area = row.flood_area.take().sanitize();
            row.water_protection_area = row.water_protection_area.take().sanitize();
        }
    }

    /// Convert the default cadenza filename into a iso 8061 timestamp.
    ///
    /// For example `table04042024125645598.xlsx` will result into
    /// `2024-04-04T12:56:45.598`.
    pub fn iso_date(&self) -> Option<String> {
        let file_stem = self.path.file_stem()?.to_string_lossy();
        let digits = file_stem.strip_prefix("table")?;
        if !digits.is_ascii() || digits.len() != 17 {
            // digits are only ascii characters
            return None;
        }

        let mut date = String::with_capacity("YYYY-MM-DDTHH:MM:SS.sss".len());
        date.push_str(&digits[4..8]);
        date.push('-');
        date.push_str(&digits[2..4]);
        date.push('-');
        date.push_str(&digits[0..2]);
        date.push('T');
        date.push_str(&digits[8..10]);
        date.push(':');
        date.push_str(&digits[10..12]);
        date.push(':');
        date.push_str(&digits[12..14]);
        date.push('.');
        date.push_str(&digits[14..17]);
        Some(date)
    }

    pub fn diff<'s, 'o, 'b>(&'s self, other: &'o Self) -> CadenzaTableDiff<'b>
    where
        's: 'b,
        'o: 'b
    {
        let self_map: HashMap<_, _> =
            HashMap::from_iter(self.rows().iter().map(|row| (row.key(), row)));
        let other_map: HashMap<_, _> =
            HashMap::from_iter(other.rows().iter().map(|row| (row.key(), row)));

        let keys: HashSet<(u64, u64)> =
            HashSet::from_iter(self_map.keys().cloned().chain(other_map.keys().cloned()));

        let mut diff = CadenzaTableDiff {
            compared: (self.iso_date(), other.iso_date()),
            added: vec![],
            removed: vec![],
            modified: vec![]
        };

        for ref key in keys {
            match (self_map.get(key), other_map.get(key)) {
                (None, None) => unreachable!("key must be from at least one map"),
                (Some(self_row), None) => diff.removed.push(self_row),
                (None, Some(other_row)) => diff.added.push(other_row),
                (Some(self_row), Some(other_row)) => {
                    // use inner representation to ensure a full check
                    if self_row.0 != other_row.0 {
                        diff.modified.push((self_row, other_row))
                    }
                }
            }
        }

        diff
    }
}

impl CadenzaTableRow {
    /// Construct a key for maps.
    pub fn key(&self) -> (u64, u64) {
        (self.no, self.usage_location_no)
    }
}

impl Deref for CadenzaTableRow {
    type Target = CadenzaTableRowInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq for CadenzaTableRow {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}

impl Hash for CadenzaTableRow {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key().hash(state)
    }
}

/// Differences between two [`CadenzaTable`]s.
#[derive(Debug, Serialize)]
pub struct CadenzaTableDiff<'b> {
    /// Timestamps of both tables, (`self`, `other`)
    pub compared: (Option<String>, Option<String>),
    pub added: Vec<&'b CadenzaTableRow>,
    pub removed: Vec<&'b CadenzaTableRow>,
    pub modified: Vec<(&'b CadenzaTableRow, &'b CadenzaTableRow)>
}

fn deserialize_date<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>
{
    let float: calamine::Data = calamine::Data::deserialize(deserializer)?;
    Ok(Some(
        float.as_date().ok_or(serde::de::Error::custom("cannot convert to date"))?.to_string()
    ))
}

fn zero_as_none<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>
{
    let option: Option<u64> = Option::deserialize(deserializer)?;
    match option {
        Some(0) => Ok(None),
        Some(x) => Ok(Some(x)),
        None => Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    const XLSX_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/test/cadenza.xlsx");

    #[allow(deprecated)]
    #[test]
    fn parsing_works() {
        let xlsx_path = Path::new(XLSX_PATH);
        let table = CadenzaTable::from_path(xlsx_path).unwrap();
        let rows = table.rows();

        let first_row = CadenzaTableRow(CadenzaTableRowInner {
            no: 1101,
            rights_holder: "Körtke".to_string().into(),
            valid_until: "2009-12-31".to_string().into(),
            status: "aktiv".to_string().into(),
            valid_from: "1989-01-23".to_string().into(),
            legal_departments: "A B ".to_string().into(),
            legal_title: "Erlaubnis".to_string().into(),
            water_authority: "Landkreis Gifhorn".to_string().into(),
            granting_authority: None,
            date_of_change: None,
            file_reference: "6630-01-1610".to_string().into(),
            external_identifier: "1/1".to_string().into(),
            subject: None,
            address: "1/34556".to_string().into(),
            usage_location_no: 101,
            usage_location: "OW-entn.f.Fischt.b.NiedrigwasKörtkeBokel".to_string().into(),
            legal_department: "Entnahme von Wasser oder Entnahmen fester Stoffe aus oberirdischen \
                               Gewässern"
                .to_string(),
            legal_purpose: "A70 Speisung von Teichen".to_string().into(),
            county: "Gifhorn".to_string().into(),
            river_basin: "Elbe/Labe".to_string().into(),
            groundwater_body: "Ilmenau Lockergestein links".to_string().into(),
            flood_area: None,
            water_protection_area: None,
            utm_easting: Some(32603873),
            utm_northing: Some(5852015)
        });

        assert_eq!(rows[0], first_row);
    }

    #[test]
    fn sort_works() {
        let a = CadenzaTableRow(CadenzaTableRowInner {
            no: 3,
            ..Default::default()
        });

        let b = CadenzaTableRow(CadenzaTableRowInner {
            no: 2,
            ..Default::default()
        });

        let c = CadenzaTableRow(CadenzaTableRowInner {
            no: 1,
            ..Default::default()
        });

        let mut table = CadenzaTable {
            path: PathBuf::new(),
            rows: vec![a, b, c]
        };
        for (i, r) in [3, 2, 1].iter().zip(table.rows().iter()) {
            assert_eq!(*i, r.no);
        }

        table.sort_by(|a, b| a.no.cmp(&b.no));
        for (i, r) in [1, 2, 3].iter().zip(table.rows().iter()) {
            assert_eq!(*i, r.no);
        }
    }

    #[test]
    fn path_to_iso_date_works() {
        let table = |path| CadenzaTable {
            path: PathBuf::from(path),
            rows: vec![]
        };

        assert_eq!(
            table("table04042024125645598.xlsx").iso_date(),
            Some(String::from("2024-04-04T12:56:45.598"))
        );

        assert_eq!(
            table("some_dir/table04042024125645598.xlsx").iso_date(),
            Some(String::from("2024-04-04T12:56:45.598"))
        );

        assert_eq!(table("table0404202412564559.xlsx").iso_date(), None);
        assert_eq!(table("table040420241256455981.xlsx").iso_date(), None);
        assert_eq!(table("0404202412564559.xlsx").iso_date(), None);
    }
}
