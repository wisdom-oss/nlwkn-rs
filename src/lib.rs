use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
        mainenance_association?: (i64, String),

        /// "EU-Bearbeitungsgebiet"
        eu_survey_area?: (i64, String),

        /// "Einzugsgebietskennzahl"
        basin_no?: (i64, String),

        /// "Entnahmemenge"
        withdrawal_rate?: RateRecord,

        /// "Einleitungsmenge"
        injection_rate?: RateRecord,

        /// "Abwasservolumenstrom"
        waste_water_flow_volume?: RateRecord,

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
        fluid_discharge?: RateRecord,

        /// "Zusatzregen"
        rain_supplement?: RateRecord,

        /// "Beregnungsfläche"
        irrigation_area?: DimensionedNumber,

        /// "pH-Werte"
        #[serde(rename = "pHValues")]
        ph_values?: PHValues,

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

/// The abbreviations of the legal departments.
#[derive(Debug, Serialize, Deserialize)]
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

type RateRecord = HashMap<TimeDimension, DimensionedNumber>;
