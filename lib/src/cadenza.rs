use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::path::Path;

use calamine::{DataType, RangeDeserializerBuilder, Reader, Xlsx};
use serde::{Deserialize, Deserializer};

use crate::util::StringOption;
use crate::WaterRightNo;

#[derive(Debug)]
pub struct CadenzaTable(Vec<CadenzaTableRow>);

#[derive(Debug, Deserialize, Eq)]
#[cfg_attr(test, derive(Default))]
#[serde(deny_unknown_fields)]
pub struct CadenzaTableRow {
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

impl CadenzaTable {
    pub fn from_path(path: &Path) -> anyhow::Result<CadenzaTable> {
        let mut workbook: Xlsx<_> = calamine::open_workbook(path)?;
        let worksheets = workbook.worksheets();
        let (_, range) = worksheets.first().ok_or(anyhow::Error::msg("workbook empty"))?;
        let iter = RangeDeserializerBuilder::new().has_headers(true).from_range(range)?;
        let rows: Result<Vec<CadenzaTableRow>, _> = iter.collect();
        Ok(CadenzaTable(rows?))
    }

    pub fn rows(&self) -> &Vec<CadenzaTableRow> {
        &self.0
    }

    pub fn sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&CadenzaTableRow, &CadenzaTableRow) -> Ordering
    {
        let slice = self.0.as_mut_slice();
        slice.sort_by(compare);
    }

    pub fn dedup_by<F>(&mut self, same_bucket: F)
    where
        F: FnMut(&mut CadenzaTableRow, &mut CadenzaTableRow) -> bool
    {
        self.0.dedup_by(same_bucket);
    }

    pub fn sanitize(&mut self) {
        #[allow(deprecated)]
        for row in self.0.iter_mut() {
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
}

impl PartialEq for CadenzaTableRow {
    fn eq(&self, other: &Self) -> bool {
        self.no == other.no && self.usage_location_no == other.usage_location_no
    }
}

impl Hash for CadenzaTableRow {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.no, self.usage_location_no).hash(state)
    }
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

        let first_row = CadenzaTableRow {
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
        };

        assert_eq!(rows[0], first_row);
    }

    #[test]
    fn sort_works() {
        let a = CadenzaTableRow {
            no: 3,
            ..Default::default()
        };

        let b = CadenzaTableRow {
            no: 2,
            ..Default::default()
        };

        let c = CadenzaTableRow {
            no: 1,
            ..Default::default()
        };

        let mut table = CadenzaTable(vec![a, b, c]);
        for (i, r) in [3, 2, 1].iter().zip(table.rows().iter()) {
            assert_eq!(*i, r.no);
        }

        table.sort_by(|a, b| a.no.cmp(&b.no));
        for (i, r) in [1, 2, 3].iter().zip(table.rows().iter()) {
            assert_eq!(*i, r.no);
        }
    }
}
