use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use lazy_static::lazy_static;
use regex::Regex;

use crate::util::data_structs;
use helper_types::*;

pub mod cli;
pub mod helper_types;
mod util;

pub type WaterRightNo = u64;

data_structs! {
    /// Data type describing a single water right.
    /// Projected from the cadenza table.
    #[serde(rename_all = "camelCase")]
    struct WaterRight {
        /// "Wasserrecht Nr."
        no: WaterRightNo,

        /// "Rechtsinhaber"
        bailee?: String,

        /// "Gültig Bis"
        valid_to?: String,

        /// "Zustand"
        state?: String,

        /// "Gültig Ab/erteilt am"
        valid_from?: String,

        /// "Rechtstitel"
        legal_title?: String,

        /// "Wasserbehörde"
        water_authority?: String,

        /// "eingetragen durch"
        registering_authority?: String,

        /// "Erteilende Behörde/erteilt durch"
        granting_authority?: String,

        /// "erstmalig erstellt am"
        first_grant?: String,

        /// "Änderungsdatum"
        date_of_change?: String,

        /// "Aktenzeichen"
        file_reference?: String,

        /// "Externe Kennung"
        external_identifier?: String,

        /// "Betreff"
        subject?: String,

        /// "Adresse"
        address?: String,

        legal_departments: HashMap<LegalDepartmentAbbreviation, LegalDepartment>,

        //report_file?: Buffer,

        /// Date when the report was crawled.
        date_of_file_crawl?: String,

        /// "Bemerkung"
        annotation?: String,
    }

    /// The water rights are split into different departments.
    #[serde(rename_all = "camelCase")]
    struct LegalDepartment {
        /// "Abteilungsbezeichnung"
        description: String,

        /// "Abteilungskürzel"
        abbreviation: LegalDepartmentAbbreviation,

        /// "Nutzungsorte"
        usage_locations: Vec<UsageLocation>,
    }

    /// A single water right may have multiple usage locations.
    #[serde(rename_all = "camelCase")]
    struct UsageLocation {
        /// "Nutzungsort Nr."
        no?: i64,

        /// "Nutzungsort Lfd. Nr."
        serial_no?: String,

        /// "aktiv/inaktiv"
        active?: bool,

        /// "real/virtuell"
        real?: bool,

        /// "Nutzungsort/Bezeichnung"
        name?: String,

        /// "Rechtszweck"
        legal_scope?: OptionalPair<String>,

        /// "Top. Karte 1:25.000"
        #[serde(rename = "topMap1:25000")]
        top_map_1_25000?: (i64, String),

        /// "Gemeindegebiet"
        municipal_area?: (i64, String),

        /// "Landkreis"
        county?: String,

        /// "Gemarkung"
        local_sub_district?: String,

        /// "Flur"
        field?: i64,

        /// "Flurstück"
        plot?: String,

        /// "Unterhaltungsverband"
        maintenance_association?: (i64, String),

        /// "EU-Bearbeitungsgebiet"
        eu_survey_area?: (i64, String),

        /// "Einzugsgebietskennzahl"
        basin_no?: (i64, String),

        /// "Verordnungszitat"
        regulation_citation?: String,

        /// "Entnahmemenge"
        withdrawal_rate: RateRecord,

        /// "Einleitungsmenge"
        injection_rate: RateRecord,

        /// "Abwasservolumenstrom"
        waste_water_flow_volume: RateRecord,

        /// "Flussgebiet"
        rivershed?: String,

        /// "Grundwasserkörper"
        groundwater_volume?: String,

        /// "Gewässer"
        water_body?: String,

        /// "Überschwemmungsgebiet"
        flood_area?: String,

        /// "Wasserschutzgebiet"
        water_protection_area?: String,

        /// "Stauziele"
        dam_target_levels?: DamTargets,

        /// "Ableitungsmenge"
        fluid_discharge: RateRecord,

        /// "Zusatzregen"
        rain_supplement: RateRecord,

        /// "Beregnungsfläche"
        irrigation_area?: DimensionedNumber,

        // TODO: check if this is still necessary or if HashMap would be better
        /// "pH-Werte"
        #[serde(rename = "pHValues")]
        ph_values?: PHValues,

        // TODO: check if this is still necessary or if HashMap would be better
        /// "Feststoffe"
        solid?: Solids,

        /// "UTM-Rechtswert"
        utm_easting?: i64,

        /// "UTM-Hochwert"
        utm_northing?: i64,
    }

    /// pH values of the water.
    struct PHValues {
        min?: u64,
        max?: u64,
    }

    /// Targets the dam should be at.
    struct DamTargets {
        default?: DimensionedNumber,

        /// "Dauertstau"
        steady?: DimensionedNumber,

        /// "Höchststau"
        max?: DimensionedNumber,
    }

    #[serde(rename_all = "camelCase")]
    struct Solids {
        /// "Abfiltrierbare Stoffe"
        filterable?: DescriptiveNumber,

        /// "Absetzbare Stoffe"
        settleable?: DescriptiveNumber,

        /// "Chlor, freies ( maßanalytisch )"
        chlorine_free_analytical?: DescriptiveNumber,

        /// "CSB"
        cod?: DescriptiveNumber,

        /// "BSB5"
        bod?: DescriptiveNumber,

        /// "Gesamtphosphat-Phosphor"
        total_phosphate_phosphorus?: DescriptiveNumber,

        /// "Ammoniumstickstoff"
        ammonium_nitrogen?: DescriptiveNumber,

        /// "Stickstoff, anorganisch"
        nitrogen_inorganic?: DescriptiveNumber,

        nitrate_nitrogen?: DescriptiveNumber,
    }
}

impl WaterRight {
    pub fn new(water_right_no: WaterRightNo) -> Self {
        WaterRight {
            no: water_right_no,
            bailee: None,
            valid_to: None,
            state: None,
            valid_from: None,
            legal_title: None,
            water_authority: None,
            registering_authority: None,
            granting_authority: None,
            first_grant: None,
            date_of_change: None,
            file_reference: None,
            external_identifier: None,
            subject: None,
            address: None,
            legal_departments: Default::default(),
            date_of_file_crawl: None,
            annotation: None,
        }
    }
}

impl LegalDepartment {
    pub fn new(abbreviation: LegalDepartmentAbbreviation, description: String) -> Self {
        LegalDepartment {
            description,
            abbreviation,
            usage_locations: vec![],
        }
    }
}

impl UsageLocation {
    pub fn new() -> Self {
        UsageLocation {
            no: None,
            serial_no: None,
            active: None,
            real: None,
            name: None,
            legal_scope: None,
            top_map_1_25000: None,
            municipal_area: None,
            county: None,
            local_sub_district: None,
            field: None,
            plot: None,
            maintenance_association: None,
            eu_survey_area: None,
            basin_no: None,
            regulation_citation: None,
            withdrawal_rate: Default::default(),
            injection_rate: Default::default(),
            waste_water_flow_volume: Default::default(),
            rivershed: None,
            groundwater_volume: None,
            water_body: None,
            flood_area: None,
            water_protection_area: None,
            dam_target_levels: None,
            fluid_discharge: Default::default(),
            rain_supplement: Default::default(),
            irrigation_area: None,
            ph_values: None,
            solid: None,
            utm_easting: None,
            utm_northing: None,
        }
    }
}

/// The abbreviations of the legal departments.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum LegalDepartmentAbbreviation {
    /// "Entnahme von Wasser oder Entnahmen fester Stoffe aus oberirdischen Gewässern"
    A,

    /// "Einbringen und Einleiten von Stoffen in oberirdische und Küstengewässer
    B,

    /// "Aufstauen und Absenken oberirdischer Gewässer"
    C,

    /// "Andere Einwirkung auf oberirdische Gewässer"
    D,

    /// "Entnahme, Zutageförderung, Zutageleiten und Ableiten von Grundwasser"
    E,

    /// "Andere Nutzungen und Einwirkungen auf das Grundwasser"
    F,

    /// "Zwangsrechte"
    K,

    /// "Fischereirechte"
    L,
}

#[derive(Debug)]
pub struct ParseLegalDepartmentError(String);

impl Display for ParseLegalDepartmentError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown legal department abbreviation {}", self.0)
    }
}

impl Error for ParseLegalDepartmentError {}

impl FromStr for LegalDepartmentAbbreviation {
    type Err = ParseLegalDepartmentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "A" => Ok(Self::A),
            "B" => Ok(Self::B),
            "C" => Ok(Self::C),
            "D" => Ok(Self::D),
            "E" => Ok(Self::E),
            "F" => Ok(Self::F),
            "K" => Ok(Self::K),
            "L" => Ok(Self::L),
            s => Err(ParseLegalDepartmentError(s.to_string())),
        }
    }
}

type RateRecord = HashMap<TimeDimension, DimensionedNumber>;

lazy_static! {
    static ref UNIT_RE: Regex = Regex::new(r"^(?<measurement>[^/]+)/(?<factor>\d*)(?<time>\w+)$").expect("valid regex");
}

pub fn rate_entry_from_str(value: &str, unit: &str) -> anyhow::Result<(TimeDimension, DimensionedNumber)> {
    let value: f64 = value.parse()?;

    let unit_capture = UNIT_RE.captures(unit).ok_or(anyhow::Error::msg(format!("unit {unit:?} has invalid format")))?;
    let unit = unit_capture["measurement"].to_string();
    let factor: u64 = unit_capture["factor"].parse().unwrap_or(1);
    let time = match &unit_capture["time"] {
        "a" => TimeDimension::Years(factor),
        _ => todo!()
    };

    Ok((time, DimensionedNumber { value, unit }))
}
