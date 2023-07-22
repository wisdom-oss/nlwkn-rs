use anyhow::Result;
use calamine::{RangeDeserializerBuilder, Reader, Xlsx};
use nlwkn_rs::WaterRightNo;
use serde::{Deserialize, Deserializer};
use std::cmp::Ordering;
use std::path::Path;

#[derive(Debug)]
pub struct CadenzaTable(Vec<CadenzaTableRow>);

#[derive(Debug, Deserialize, PartialEq)]
#[cfg_attr(test, derive(Default))]
#[serde(deny_unknown_fields)]
pub struct CadenzaTableRow {
    #[serde(rename = "Wasserrecht Nr.")]
    pub no: WaterRightNo,

    #[serde(rename = "Rechtsinhaber")]
    pub bailee: Option<String>,

    #[serde(rename = "Gültig Bis", deserialize_with = "deserialize_date", default)]
    pub valid_to: Option<String>,

    #[serde(rename = "Zustand")]
    pub state: Option<String>,

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

    #[serde(rename = "Aenderungsdatum")]
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
    pub legal_scope: Option<String>,

    #[serde(rename = "Landkreis")]
    pub county: Option<String>,

    #[serde(rename = "Flussgebiet")]
    pub rivershed: Option<String>,

    #[serde(rename = "Grundwasserkörper")]
    pub groundwater_volume: Option<String>,

    #[serde(rename = "Überschwemmungsgebiet")]
    pub flood_area: Option<String>,

    #[serde(rename = "Wasserschutzgebiet")]
    pub water_protection_area: Option<String>,

    #[serde(rename = "UTM-Rechtswert", deserialize_with = "zero_as_none")]
    pub utm_easting: Option<i64>,

    #[serde(rename = "UTM-Hochwert", deserialize_with = "zero_as_none")]
    pub utm_northing: Option<i64>,
}

impl CadenzaTable {
    pub fn from_path(path: &Path) -> Result<CadenzaTable> {
        let mut workbook: Xlsx<_> = calamine::open_workbook(path)?;
        let worksheets = workbook.worksheets();
        let (_, range) = worksheets
            .get(0)
            .ok_or(anyhow::Error::msg("workbook empty"))?;
        let iter = RangeDeserializerBuilder::new()
            .has_headers(true)
            .from_range(range)?;
        let rows: Result<Vec<CadenzaTableRow>, _> = iter.collect();
        Ok(CadenzaTable(rows?))
    }

    pub fn rows(&self) -> &Vec<CadenzaTableRow> {
        &self.0
    }

    pub fn sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&CadenzaTableRow, &CadenzaTableRow) -> Ordering,
    {
        let slice = self.0.as_mut_slice();
        slice.sort_by(compare);
    }

    pub fn dedup_by<F>(&mut self, same_bucket: F)
    where
        F: FnMut(&mut CadenzaTableRow, &mut CadenzaTableRow) -> bool,
    {
        self.0.dedup_by(same_bucket);
    }
}

fn deserialize_date<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let float: calamine::DataType = calamine::DataType::deserialize(deserializer)?;
    Ok(Some(
        float
            .as_date()
            .ok_or(serde::de::Error::custom("cannot convert to date"))?
            .to_string(),
    ))
}

fn zero_as_none<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    let option: Option<i64> = Option::deserialize(deserializer)?;
    match option {
        Some(0) => Ok(None),
        Some(x) => Ok(Some(x)),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use crate::xlsx::{CadenzaTable, CadenzaTableRow};
    use std::path::Path;

    const XLSX_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/test/cadenza.xlsx");

    #[allow(deprecated)]
    #[test]
    fn parsing_works() {
        let xlsx_path = Path::new(XLSX_PATH);
        let table = CadenzaTable::from_path(xlsx_path).unwrap();
        let rows = table.rows();

        let first_row = CadenzaTableRow {
            no: 1101,
            bailee: "Körtke".to_string().into(),
            valid_to: "2009-12-31".to_string().into(),
            state: "aktiv".to_string().into(),
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
            usage_location: "OW-entn.f.Fischt.b.NiedrigwasKörtkeBokel"
                .to_string()
                .into(),
            legal_department:
                "Entnahme von Wasser oder Entnahmen fester Stoffe aus oberirdischen Gewässern"
                    .to_string(),
            legal_scope: "A70 Speisung von Teichen".to_string().into(),
            county: "Gifhorn".to_string().into(),
            rivershed: "Elbe/Labe".to_string().into(),
            groundwater_volume: "Ilmenau Lockergestein links".to_string().into(),
            flood_area: None,
            water_protection_area: None,
            utm_easting: Some(32603873),
            utm_northing: Some(5852015),
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
