use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::marker::PhantomData;
use std::mem;

use itertools::Itertools;
use crate::flat_table::value::FlatTableValue;

pub enum FlatTableKey<M> {
    Multiple {
        phantom: PhantomData<M>,
        en: Cow<'static, str>,
        de: Cow<'static, str>
    },
    Single(Cow<'static, str>)
}

impl FlatTableKey<marker::Unselect> {
    pub const ACTIVE: FlatTableKey<marker::Unselect> = Self::from_str("active", "aktiv/inaktiv");
    pub const ADDRESS: FlatTableKey<marker::Unselect> = Self::from_str("address", "Adresse");
    pub const ANNOTATION: FlatTableKey<marker::Unselect> =
        Self::from_str("annotation", "Bemerkung");
    pub const BASIN_CODE: FlatTableKey<marker::Unselect> =
        Self::from_str("basin code", "Einzugsgebietskennzahl");
    pub const COUNTY: FlatTableKey<marker::Unselect> = Self::from_str("county", "Landkreis");
    pub const DAM_TARGETS_DEFAULT: FlatTableKey<marker::Unselect> =
        Self::from_str("dam target level default", "Stauziel");
    pub const DAM_TARGETS_MAX: FlatTableKey<marker::Unselect> =
        Self::from_str("dam target level max", "Höchststau");
    pub const DAM_TARGETS_STEADY: FlatTableKey<marker::Unselect> =
        Self::from_str("dam target level steady", "Dauerstau");
    pub const DAM_TARGET_LEVELS: FlatTableKey<marker::Unselect> =
        Self::from_str("dam target levels", "Stauziele");
    pub const DATE_OF_CHANGE: FlatTableKey<marker::Unselect> =
        Self::from_str("date of change", "Änderungsdatum");
    pub const EU_SURVEY_AREA: FlatTableKey<marker::Unselect> =
        Self::from_str("eu survey area", "EU-Bearbeitungsgebiet");
    pub const EXTERNAL_IDENTIFIER: FlatTableKey<marker::Unselect> =
        Self::from_str("external identifier", "Externe Kennung");
    pub const FILE_REFERENCE: FlatTableKey<marker::Unselect> =
        Self::from_str("file reference", "Aktenzeichen");
    pub const FIRST_GRANT: FlatTableKey<marker::Unselect> =
        Self::from_str("first grant", "erstmalig erstellt am");
    pub const FLOOD_AREA: FlatTableKey<marker::Unselect> =
        Self::from_str("flood area", "Überschwemmungsgebiet");
    pub const FLUID_DISCHARGE: FlatTableKey<marker::Unselect> =
        Self::from_str("fluid discharge", "Ableitungsmenge");
    pub const GRANTING_AUTHORITY: FlatTableKey<marker::Unselect> =
        Self::from_str("granting authority", "Erteilende Behörde");
    pub const GROUNDWATER_BODY: FlatTableKey<marker::Unselect> =
        Self::from_str("groundwater body", "Grundwasserkörper");
    pub const INJECTION_LIMIT: FlatTableKey<marker::Unselect> =
        Self::from_str("injection limit", "Erlaubniswert");
    pub const INJECTION_RATE: FlatTableKey<marker::Unselect> =
        Self::from_str("injection rate", "Einleitungsmenge");
    pub const IRRIGATION_AREA: FlatTableKey<marker::Unselect> =
        Self::from_str("irrigation area", "Beregnungsfläche");
    pub const LAND_RECORD: FlatTableKey<marker::Unselect> =
        Self::from_str("land record", "Gemarkung, Flur");
    pub const LEGAL_DEPARTMENT_ABBREVIATION: FlatTableKey<marker::Unselect> =
        Self::from_str("legal department abbreviation", "Abteilungskürzel");
    pub const LEGAL_DEPARTMENT_DESCRIPTION: FlatTableKey<marker::Unselect> =
        Self::from_str("legal department description", "Abteilungsbezeichnung");
    pub const LEGAL_PURPOSE: FlatTableKey<marker::Unselect> =
        Self::from_str("legal purpose", "Rechtszweck");
    pub const LEGAL_TITLE: FlatTableKey<marker::Unselect> =
        Self::from_str("legal title", "Rechtstitel");
    pub const MAINTENANCE_ASSOCIATION: FlatTableKey<marker::Unselect> =
        Self::from_str("maintenance association", "Unterhaltungsverband");
    pub const MUNICIPAL_AREA: FlatTableKey<marker::Unselect> =
        Self::from_str("municipal area", "Gemeindegebiet");
    pub const NO: FlatTableKey<marker::Unselect> =
        Self::from_str("water right no.", "Wasserrecht Nr.");
    pub const PH_VALUES: FlatTableKey<marker::Unselect> = Self::from_str("ph values", "pH-Werte");
    pub const PH_VALUES_MAX: FlatTableKey<marker::Unselect> =
        Self::from_str("ph values max", "pH-Werte max");
    pub const PH_VALUES_MIN: FlatTableKey<marker::Unselect> =
        Self::from_str("ph values min", "pH-Werte min");
    pub const PLOT: FlatTableKey<marker::Unselect> = Self::from_str("plot", "Flurstück");
    pub const PUMPING_RATE: FlatTableKey<marker::Unselect> =
        Self::from_str("pumping rate", "Förderleistung");
    pub const RAIN_SUPPLEMENT: FlatTableKey<marker::Unselect> =
        Self::from_str("rain supplement", "Zusatzregen");
    pub const REAL: FlatTableKey<marker::Unselect> = Self::from_str("real", "real/virtuell");
    pub const REGISTERING_AUTHORITY: FlatTableKey<marker::Unselect> =
        Self::from_str("registering authority", "eingetragen durch");
    pub const REGULATION_CITATION: FlatTableKey<marker::Unselect> =
        Self::from_str("regulation citation", "Verordnungszitat");
    pub const RIGHTS_HOLDER: FlatTableKey<marker::Unselect> =
        Self::from_str("rights holder", "Rechtsinhaber");
    pub const RIVER_BASIN: FlatTableKey<marker::Unselect> =
        Self::from_str("river basin", "Flussgebiet");
    const SORT_ORDER: [Self; 41] = [
        Self::NO,
        Self::RIGHTS_HOLDER,
        Self::VALID_FROM,
        Self::VALID_UNTIL,
        Self::STATUS,
        Self::LEGAL_TITLE,
        Self::WATER_AUTHORITY,
        Self::REGISTERING_AUTHORITY,
        Self::GRANTING_AUTHORITY,
        Self::FIRST_GRANT,
        Self::DATE_OF_CHANGE,
        Self::FILE_REFERENCE,
        Self::EXTERNAL_IDENTIFIER,
        Self::SUBJECT,
        Self::ADDRESS,
        Self::LEGAL_DEPARTMENT_ABBREVIATION,
        Self::LEGAL_DEPARTMENT_DESCRIPTION,
        Self::USAGE_LOCATION_NO,
        Self::USAGE_LOCATION_NAME,
        Self::USAGE_LOCATION_SERIAL_NO,
        Self::ACTIVE,
        Self::REAL,
        Self::LEGAL_PURPOSE,
        Self::TOP_MAP_1_25000,
        Self::MUNICIPAL_AREA,
        Self::COUNTY,
        Self::LAND_RECORD,
        Self::PLOT,
        Self::MAINTENANCE_ASSOCIATION,
        Self::EU_SURVEY_AREA,
        Self::BASIN_CODE,
        Self::REGULATION_CITATION,
        Self::RIVER_BASIN,
        Self::GROUNDWATER_BODY,
        Self::WATER_BODY,
        Self::FLOOD_AREA,
        Self::WATER_PROTECTION_AREA,
        Self::IRRIGATION_AREA,
        Self::UTM_EASTING,
        Self::UTM_NORTHING,
        Self::ANNOTATION
    ];
    pub const STATUS: FlatTableKey<marker::Unselect> = Self::from_str("status", "Zustand");
    pub const SUBJECT: FlatTableKey<marker::Unselect> = Self::from_str("subject", "Betreff");
    pub const TOP_MAP_1_25000: FlatTableKey<marker::Unselect> =
        Self::from_str("top. map 1:25000", "Top. Karte 1:25.000");
    pub const USAGE_LOCATION_NAME: FlatTableKey<marker::Unselect> =
        Self::from_str("usage location name", "Nutzungsort/Bezeichnung");
    pub const USAGE_LOCATION_NO: FlatTableKey<marker::Unselect> =
        Self::from_str("usage location no.", "Nutzungsort Nr.");
    pub const USAGE_LOCATION_SERIAL_NO: FlatTableKey<marker::Unselect> =
        Self::from_str("usage location serial no.", "Nutzungsort Lfd. Nr.");
    pub const UTM_EASTING: FlatTableKey<marker::Unselect> =
        Self::from_str("utm easting", "UTM-Rechtswert");
    pub const UTM_NORTHING: FlatTableKey<marker::Unselect> =
        Self::from_str("utm northing", "UTM-Hochwert");
    pub const VALID_FROM: FlatTableKey<marker::Unselect> =
        Self::from_str("valid from", "Gültig Ab/erteilt am");
    pub const VALID_UNTIL: FlatTableKey<marker::Unselect> =
        Self::from_str("valid until", "Gültig Bis");
    pub const WASTER_WATER_FLOW_VOLUME: FlatTableKey<marker::Unselect> =
        Self::from_str("waste water flow volume", "Abwasservolumentstrom");
    pub const WATER_AUTHORITY: FlatTableKey<marker::Unselect> =
        Self::from_str("water authority", "Wasserbehörde");
    pub const WATER_BODY: FlatTableKey<marker::Unselect> = Self::from_str("water body", "Gewässer");
    pub const WATER_PROTECTION_AREA: FlatTableKey<marker::Unselect> =
        Self::from_str("water protection area", "Wasserschutzgebiet");
    pub const WITHDRAWAL_RATE: FlatTableKey<marker::Unselect> =
        Self::from_str("withdrawal rate", "Entnahmemenge");
}

impl<M> Clone for FlatTableKey<M> {
    fn clone(&self) -> Self {
        match self {
            FlatTableKey::Multiple { de, en, .. } => FlatTableKey::Multiple {
                de: de.clone(),
                en: en.clone(),
                phantom: PhantomData
            },
            FlatTableKey::Single(s) => FlatTableKey::Single(s.clone())
        }
    }
}

impl<M> FlatTableKey<M> {
    const fn from_str(en: &'static str, de: &'static str) -> Self {
        Self::Multiple {
            phantom: PhantomData,
            en: Cow::Borrowed(en),
            de: Cow::Borrowed(de)
        }
    }

    /// Converts a `&FlatTableKey<marker::Unselect>` to `&FlatTableKey<M>`,
    /// where `M` is any marker type.
    ///
    /// # Safety
    ///
    /// This function uses `std::mem::transmute` to perform a zero-cost
    /// conversion of the reference. The safety of this operation is ensured
    /// because:
    /// - The memory layout of `FlatTableKey<marker::Unselect>` and
    ///   `FlatTableKey<M>` is identical.
    /// - The marker types, irrespective of their differences, are encapsulated
    ///   within `PhantomData` which does not affect the memory layout.
    ///
    /// As such, there's no risk of undefined behavior arising from this
    /// conversion, provided the structure of `FlatTableKey` remains
    /// consistent.
    pub fn from_unselect_ref(value: &FlatTableKey<marker::Unselect>) -> &Self {
        unsafe { mem::transmute(value) }
    }

    pub fn from_unselect(value: FlatTableKey<marker::Unselect>) -> Self {
        unsafe { mem::transmute(value) }
    }

    pub fn ref_de(&self) -> &str {
        match self {
            FlatTableKey::Multiple { de, .. } => de.as_ref(),
            FlatTableKey::Single(s) => s.as_ref()
        }
    }

    pub fn ref_en(&self) -> &str {
        match self {
            FlatTableKey::Multiple { en, .. } => en.as_ref(),
            FlatTableKey::Single(s) => s.as_ref()
        }
    }
}

impl<M> FlatTableKey<M>
where
    FlatTableKey<M>: AsRef<str>
{
    pub fn sort_index(&self) -> Option<usize> {
        FlatTableKey::<marker::Unselect>::SORT_ORDER
            .iter()
            .map(|i| Self::from_unselect_ref(i))
            .find_position(|&i| self == i)
            .map(|(i, _)| i)
    }
}

impl<M> From<String> for FlatTableKey<M> {
    fn from(value: String) -> Self {
        Self::Single(Cow::Owned(value))
    }
}

impl<M> From<(String, String)> for FlatTableKey<M> {
    fn from((en, de): (String, String)) -> Self {
        Self::Multiple {
            phantom: PhantomData,
            en: Cow::Owned(en),
            de: Cow::Owned(de)
        }
    }
}

impl AsRef<str> for FlatTableKey<marker::En> {
    fn as_ref(&self) -> &str {
        self.ref_en()
    }
}

impl AsRef<str> for FlatTableKey<marker::De> {
    fn as_ref(&self) -> &str {
        self.ref_de()
    }
}

impl<M> Eq for FlatTableKey<M> where FlatTableKey<M>: AsRef<str> {}

impl<M> PartialEq for FlatTableKey<M>
where
    FlatTableKey<M>: AsRef<str>
{
    fn eq(&self, other: &Self) -> bool {
        self.as_ref() == other.as_ref()
    }
}

impl<M> Ord for FlatTableKey<M>
where
    FlatTableKey<M>: AsRef<str>
{
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.sort_index(), other.sort_index()) {
            (Some(this), Some(that)) => this.cmp(&that),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => self.as_ref().cmp(other.as_ref())
        }
    }
}

impl<M> PartialOrd for FlatTableKey<M>
where
    FlatTableKey<M>: AsRef<str>
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub mod marker {
    pub struct Unselect;
    pub struct En;
    pub struct De;
}
