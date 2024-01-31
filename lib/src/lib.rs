use std::collections::{BTreeSet, HashMap};
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
        #[serde(alias = "rightsHolder")]
        holder?: String,

        /// "Gültig Bis"
        valid_until?: String,

        /// "Zustand"
        status?: String,

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
        #[serde(alias = "firstGrant")]
        initially_granted?: String,

        /// "Änderungsdatum"
        #[serde(alias = "dateOfChange")]
        last_change?: String,

        /// "Aktenzeichen"
        file_reference?: String,

        /// "Externe Kennung"
        external_identifier?: String,

        /// "Betreff"
        subject?: String,

        /// "Adresse"
        address?: String,

        /// The usage locations of a water right are split into multiple legal
        /// departments.
        /// This map holds all legal departments available in a water right and
        /// their corresponding usage locations.
        legal_departments: HashMap<LegalDepartmentAbbreviation, LegalDepartment>,

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
        #[serde(alias = "serialNo")]
        serial?: String,

        /// "aktiv/inaktiv"
        active?: bool,

        /// "real/virtuell"
        real?: bool,

        /// "Nutzungsort/Bezeichnung"
        name?: String,

        /// "Rechtszweck"
        legal_purpose?: (String, String),

        /// "Top. Karte 1:25.000"
        #[serde(alias = "topMap1:25000")]
        map_excerpt?: SingleOrPair<u64, String>,

        /// "Gemeindegebiet"
        municipal_area?: (u64, String),

        /// "Landkreis"
        county?: String,

        /// "Gemarkung, Flur"
        land_record?: OrFallback<LandRecord>,

        /// "Flurstück"
        plot?: String,

        /// "Unterhaltungsverband"
        maintenance_association?: (u64, String),

        /// "EU-Bearbeitungsgebiet"
        eu_survey_area?: (u64, String),

        /// "Einzugsgebietskennzahl"
        #[serde(alias = "basinCode")]
        catchment_area_code?: SingleOrPair<u64, String>,

        /// "Verordnungszitat"
        regulation_citation?: String,

        /// "Entnahmemenge"
        #[serde(
            skip_serializing_if = "RateRecord::is_empty",
            default,
            alias = "withdrawalRate"
        )]
        withdrawal_rates: RateRecord,

        /// "Förderleistung"
        #[serde(
            skip_serializing_if = "RateRecord::is_empty",
            default,
            alias = "pumpingRate"
        )]
        pumping_rates: RateRecord,

        /// "Einleitungsmenge"
        #[serde(
            skip_serializing_if = "RateRecord::is_empty",
            default,
            alias = "injectionRate"
        )]
        injection_rates: RateRecord,

        /// "Abwasservolumenstrom"
        #[serde(skip_serializing_if = "RateRecord::is_empty", default)]
        waste_water_flow_volume: RateRecord,

        /// "Flussgebiet"
        river_basin?: String,

        /// "Grundwasserkörper"
        groundwater_body?: String,

        /// "Gewässer"
        water_body?: String,

        /// "Überschwemmungsgebiet"
        flood_area?: String,

        /// "Wasserschutzgebiet"
        water_protection_area?: String,

        /// "Stauziele"
        #[serde(skip_serializing_if = "DamTargets::is_empty", default)]
        dam_target_levels: DamTargets,

        /// "Ableitungsmenge"
        #[serde(skip_serializing_if = "RateRecord::is_empty", default)]
        fluid_discharge: RateRecord,

        /// "Zusatzregen"
        #[serde(skip_serializing_if = "RateRecord::is_empty", default)]
        rain_supplement: RateRecord,

        /// "Beregnungsfläche"
        irrigation_area?: Quantity,

        /// "pH-Werte"
        #[serde(rename = "pHValues")]
        ph_values?: PHValues,

        /// "Erlaubniswert" for legal department B
        #[serde(
            skip_serializing_if = "Vec::is_empty",
            default,
            alias = "injectionLimit"
        )]
        injection_limits: Vec<(String, Quantity)>,

        /// "UTM-Rechtswert"
        utm_easting?: u64,

        /// "UTM-Hochwert"
        utm_northing?: u64,
    }

    #[serde(rename_all = "camelCase")]
    struct LandRecord {
        #[serde(alias = "registerDistrict")]
        district: String,

        #[serde(alias = "fieldNumber")]
        field: u32,
    }

    /// pH values of the water.
    #[skip_serializing_none]
    struct PHValues {
        min?: u64,
        max?: u64,
    }

    /// Targets the dam should be at.
    #[skip_serializing_none]
    #[non_exhaustive]
    #[derive(Default)]
    struct DamTargets {
        default?: Quantity,

        /// "Dauertstau"
        steady?: Quantity,

        /// "Höchststau"
        max?: Quantity,
    }
}

impl WaterRight {
    pub fn new(water_right_no: WaterRightNo) -> Self {
        WaterRight {
            no: water_right_no,
            holder: None,
            valid_until: None,
            status: None,
            valid_from: None,
            legal_title: None,
            water_authority: None,
            registering_authority: None,
            granting_authority: None,
            initially_granted: None,
            last_change: None,
            file_reference: None,
            external_identifier: None,
            subject: None,
            address: None,
            legal_departments: Default::default(),
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
            serial: None,
            active: None,
            real: None,
            name: None,
            legal_purpose: None,
            map_excerpt: None,
            municipal_area: None,
            county: None,
            land_record: None,
            plot: None,
            maintenance_association: None,
            eu_survey_area: None,
            catchment_area_code: None,
            regulation_citation: None,
            withdrawal_rates: Default::default(),
            pumping_rates: Default::default(),
            injection_rates: Default::default(),
            waste_water_flow_volume: Default::default(),
            river_basin: None,
            groundwater_body: None,
            water_body: None,
            flood_area: None,
            water_protection_area: None,
            dam_target_levels: DamTargets::default(),
            fluid_discharge: Default::default(),
            rain_supplement: Default::default(),
            irrigation_area: None,
            ph_values: None,
            injection_limits: Default::default(),
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

impl Display for LegalDepartmentAbbreviation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let char = match self {
            LegalDepartmentAbbreviation::A => 'A',
            LegalDepartmentAbbreviation::B => 'B',
            LegalDepartmentAbbreviation::C => 'C',
            LegalDepartmentAbbreviation::D => 'D',
            LegalDepartmentAbbreviation::E => 'E',
            LegalDepartmentAbbreviation::F => 'F',
            LegalDepartmentAbbreviation::K => 'K',
            LegalDepartmentAbbreviation::L => 'L'
        };

        write!(f, "{char}")
    }
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

pub type RateRecord = BTreeSet<OrFallback<Rate<f64>>>;

impl DamTargets {
    pub fn is_empty(&self) -> bool {
        self.steady.is_none() && self.max.is_none() && self.default.is_none()
    }
}
