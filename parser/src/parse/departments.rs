use std::str::FromStr;

use itertools::Itertools;
use lazy_static::lazy_static;
use nlwkn::helper_types::{OrFallback, Quantity, Rate, SingleOrPair};
use nlwkn::util::StringOption;
use nlwkn::{LandRecord, LegalDepartment, LegalDepartmentAbbreviation, UsageLocation, WaterRight};
use regex::Regex;

use crate::intermediate::key_value::KeyValuePair;

pub fn parse_departments(
    items: Vec<(String, Vec<Vec<KeyValuePair>>)>,
    water_right: &mut WaterRight
) -> anyhow::Result<()> {
    for (department_text, usage_locations) in items {
        let mut department_text_split = department_text.splitn(3, ' ');
        let abbreviation: LegalDepartmentAbbreviation = department_text_split
            .next()
            .ok_or(anyhow::Error::msg("department is missing abbreviation"))?
            .parse()?;
        department_text_split.next();
        let description = department_text_split
            .next()
            .ok_or(anyhow::Error::msg("department is missing description"))?
            .to_string();

        let mut legal_department = LegalDepartment::new(abbreviation, description);
        parse_usage_locations(usage_locations, &mut legal_department, abbreviation)?;
        water_right.legal_departments.insert(abbreviation, legal_department);
    }

    Ok(())
}

fn parse_usage_locations(
    usage_locations: Vec<Vec<KeyValuePair>>,
    legal_department: &mut LegalDepartment,
    department: LegalDepartmentAbbreviation
) -> anyhow::Result<()> {
    for usage_location_items in usage_locations {
        let mut usage_location = UsageLocation::new();
        parse_usage_location(usage_location_items, &mut usage_location, department)?;
        legal_department.usage_locations.push(usage_location);
    }

    Ok(())
}

lazy_static! {
    static ref USAGE_LOCATION_RE: Regex =
        Regex::new(r"^(?<ser_no>.*) \((?<active>\w+), (?<real>\w+)\)$").expect("valid regex");
    static ref STRING_NUM_RE: Regex =
        Regex::new(r"^(?<string>\D+)\s*(?<num>\d+)$").expect("valid regex");
}

fn parse_usage_location(
    items: Vec<KeyValuePair>,
    usage_location: &mut UsageLocation,
    department: LegalDepartmentAbbreviation
) -> anyhow::Result<()> {
    for (key, values) in items {
        let mut values = values.into_iter();
        let mut first = values.next().sanitize();
        let mut second = values.next().sanitize();

        match (key.as_str(), first.take(), second.take()) {
            ("Nutzungsort Lfd. Nr.:", Some(v), _) => {
                let captured = USAGE_LOCATION_RE.captures(&v).ok_or(anyhow::Error::msg(
                    format!("'Nutzungsort' has invalid format: {v}")
                ))?;
                usage_location.serial_no = Some(captured["ser_no"].to_string());
                usage_location.active = Some(&captured["active"] == "aktiv");
                usage_location.real = Some(&captured["real"] == "real");
            }
            ("Bezeichnung:", v, _) => usage_location.name = v.map(|s| s.replace('\n', " ")),
            ("Rechtszweck:", Some(v), _) => {
                usage_location.legal_purpose =
                    v.splitn(2, ' ').map(ToString::to_string).collect_tuple()
            }
            ("East und North:", Some(v), _) => usage_location.utm_easting = Some(v.parse()?),
            ("Top. Karte 1:25.000:", None, None) => (),
            ("Top. Karte 1:25.000:", Some(num), None) => {
                usage_location.top_map_1_25000 =
                    Some(SingleOrPair::Single(num.replace(' ', "").parse()?))
            }
            ("Top. Karte 1:25.000:", Some(num), Some(s)) => {
                usage_location.top_map_1_25000 =
                    Some(SingleOrPair::Pair(num.replace(' ', "").parse()?, s))
            }
            ("(ETRS89/UTM 32N)", Some(v), _) => usage_location.utm_northing = Some(v.parse()?),
            ("Gemeindegebiet:", None, None) => (),
            ("Gemeindegebiet:", Some(num), Some(s)) => {
                usage_location.municipal_area = Some((num.parse()?, s))
            }
            ("Gemarkung, Flur:", None, None) => (),
            ("Gemarkung, Flur:", Some(v), _) => {
                let v = v.replace(' ', "");
                match STRING_NUM_RE.captures(&v).ok_or(anyhow::Error::msg(format!(
                    "'Gemarkung, Flur' has invalid format: {v}"
                ))) {
                    Ok(captured) => usage_location.land_record.replace(
                        LandRecord {
                            register_district: captured["string"].to_string(),
                            field_number: captured["num"].parse()?
                        }
                        .into()
                    ),
                    Err(_) => usage_location.land_record.replace(OrFallback::Fallback(v))
                };
            }
            ("Unterhaltungsverband:", None, None) => (),
            ("Unterhaltungsverband:", Some(num), Some(s)) => {
                usage_location.maintenance_association = Some((num.parse()?, s))
            }
            ("Flurstück:", None, None) => (),
            ("Flurstück:", Some(v), _) => usage_location.plot = Some(v.parse()?),
            ("EU-Bearbeitungsgebiet:", None, None) => (),
            ("EU-Bearbeitungsgebiet:", Some(num), Some(s)) => {
                usage_location.eu_survey_area = Some((num.parse()?, s))
            }
            ("Gewässer:", v, _) => usage_location.water_body = v,
            ("Einzugsgebietskennzahl:", None, None) => (),
            ("Einzugsgebietskennzahl:", Some(num), None) => {
                usage_location.basin_code =
                    Some(SingleOrPair::Single(num.replace(' ', "").parse()?))
            }
            ("Einzugsgebietskennzahl:", Some(num), Some(s)) => {
                usage_location.basin_code =
                    Some(SingleOrPair::Pair(num.replace(' ', "").parse()?, s))
            }
            ("Verordnungszitat:", v, _) => usage_location.regulation_citation = v,
            ("Erlaubniswert:", Some(v), _) => parse_allowance_value(v, usage_location, department)?,

            (key, first, second) => {
                return Err(anyhow::Error::msg(format!(
                    "invalid entry for the usage location, key: {key:?}, first: {first:?}, \
                     second: {second:?}"
                )));
            }
        }
    }

    Ok(())
}

fn parse_allowance_value(
    value: String,
    usage_location: &mut UsageLocation,
    department: LegalDepartmentAbbreviation
) -> anyhow::Result<()> {
    use LegalDepartmentAbbreviation::*;

    let mut split = value.rsplitn(3, ' ');
    let unit = split.next().ok_or(anyhow::Error::msg("'Erlaubniswert' has no unit"))?;
    let value = split.next().ok_or(anyhow::Error::msg("'Erlaubniswert' has no value"))?;
    let kind = split.next().ok_or(anyhow::Error::msg("'Erlaubniswert' has no specifier"))?;
    let rate = format!("{value} {unit}");
    let rate = match Rate::from_str(&rate) {
        Ok(rate) => OrFallback::Expected(rate),
        Err(_) => OrFallback::Fallback(rate)
    };

    match kind {
        "Entnahmemenge" => {
            usage_location.withdrawal_rate.insert(rate);
        }
        "Förderleistung" => {
            usage_location.pumping_rate.insert(rate);
        }
        "Einleitungsmenge" => {
            usage_location.injection_rate.insert(rate);
        }
        "Stauziel, bezogen auf NN" => {
            usage_location
                .dam_target_levels
                .default
                .replace((value.parse()?, unit.to_string()).into());
        }
        "Stauziel (Höchststau), bezogen auf NN" => {
            usage_location.dam_target_levels.max.replace((value.parse()?, unit.to_string()).into());
        }
        "Stauziel (Dauerstau), bezogen auf NN" => {
            usage_location
                .dam_target_levels
                .steady
                .replace((value.parse()?, unit.to_string()).into());
        }
        "Abwasservolumenstrom, Sekunde" |
        "Abwasservolumenstrom, RW, Sekunde" |
        "Abwasservolumenstrom, Std." |
        "Abwasservolumenstrom, Tag" |
        "Abwasservolumenstrom, Jahr" |
        "Abwasservolumenstrom, RW, Jahr" => {
            usage_location.waste_water_flow_volume.insert(rate);
        }
        "Beregnungsfläche" => {
            usage_location.irrigation_area.replace((value.parse()?, unit.to_string()).into());
        }
        "Zusatzregen" => {
            usage_location.rain_supplement.insert(rate);
        }
        "Ableitungsmenge" => {
            usage_location.fluid_discharge.insert(rate);
        }
        a if matches!(department, B | C | F) => {
            usage_location.injection_limit.push((a.to_string(), Quantity {
                value: value.parse()?,
                unit: unit.to_string()
            }));
        }
        a => return Err(anyhow::Error::msg(format!("unknown allow value: {a:?}")))
    }

    Ok(())
}
