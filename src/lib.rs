use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use helper_types::*;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::util::data_structs;

pub mod cadenza;
pub mod cli;
pub mod helper_types;
pub mod util;

pub type WaterRightNo = u64;

data_structs! {
    /// Data type describing a single water right.
    /// Projected from the cadenza table.
    #[serde(rename_all = "camelCase")]
    #[skip_serializing_none]
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
    #[skip_serializing_none]
    #[derive(Default)]
    struct UsageLocation {
        /// "Nutzungsort Nr."
        no?: u64,

        /// "Nutzungsort Lfd. Nr."
        serial_no?: String,

        /// "aktiv/inaktiv"
        active?: bool,

        /// "real/virtuell"
        real?: bool,

        /// "Nutzungsort/Bezeichnung"
        name?: String,

        /// "Rechtszweck"
        legal_scope?: (String, String),

        /// "Top. Karte 1:25.000"
        #[serde(rename = "topMap1:25000")]
        top_map_1_25000?: (u64, String),

        /// "Gemeindegebiet"
        municipal_area?: (u64, String),

        /// "Landkreis"
        county?: String,

        /// "Gemarkung"
        local_sub_district?: String,

        /// "Flur"
        field?: u64,

        /// "Flurstück"
        plot?: String,

        /// "Unterhaltungsverband"
        maintenance_association?: (u64, String),

        /// "EU-Bearbeitungsgebiet"
        eu_survey_area?: (u64, String),

        /// "Einzugsgebietskennzahl"
        basin_no?: (u64, String),

        /// "Verordnungszitat"
        regulation_citation?: String,

        /// "Entnahmemenge"
        #[serde(skip_serializing_if = "RateRecord::is_empty")]
        withdrawal_rate: RateRecord,

        /// "Einleitungsmenge"
        #[serde(skip_serializing_if = "RateRecord::is_empty")]
        injection_rate: RateRecord,

        /// "Abwasservolumenstrom"
        #[serde(skip_serializing_if = "RateRecord::is_empty")]
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
        #[serde(skip_serializing_if = "RateRecord::is_empty")]
        fluid_discharge: RateRecord,

        /// "Zusatzregen"
        #[serde(skip_serializing_if = "RateRecord::is_empty")]
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
        utm_easting?: u64,

        /// "UTM-Hochwert"
        utm_northing?: u64,
    }

    /// pH values of the water.
    #[skip_serializing_none]
    struct PHValues {
        min?: u64,
        max?: u64,
    }

    /// Targets the dam should be at.
    #[skip_serializing_none]
    struct DamTargets {
        default?: DimensionedNumber,

        /// "Dauertstau"
        steady?: DimensionedNumber,

        /// "Höchststau"
        max?: DimensionedNumber,
    }

    #[serde(rename_all = "camelCase")]
    #[skip_serializing_none]
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
            annotation: None
        }
    }
}

impl LegalDepartment {
    pub fn new(abbreviation: LegalDepartmentAbbreviation, description: String) -> Self {
        LegalDepartment {
            description,
            abbreviation,
            usage_locations: vec![]
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
            utm_northing: None
        }
    }
}

/// The abbreviations of the legal departments.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum LegalDepartmentAbbreviation {
    /// "Entnahme von Wasser oder Entnahmen fester Stoffe aus oberirdischen
    /// Gewässern"
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
    L
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
            s => Err(ParseLegalDepartmentError(s.to_string()))
        }
    }
}

type RateRecord = BTreeSet<Rate<f64>>;
