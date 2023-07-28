use lazy_static::lazy_static;
use nlwkn_rs::helper_types::OptionalPair;
use nlwkn_rs::{LegalDepartment, LegalDepartmentAbbreviation, UsageLocation, WaterRight};
use regex::Regex;

use crate::intermediate::key_value::KeyValuePair;

pub fn parse_departments(
    items: Vec<(String, Vec<Vec<KeyValuePair>>)>,
    water_right: &mut WaterRight
) -> anyhow::Result<()> {
    for (department_text, usage_locations) in items {
        let mut department_text_split = department_text.splitn(3, " ");
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
        parse_usage_locations(usage_locations, &mut legal_department)?;
        water_right
            .legal_departments
            .insert(abbreviation, legal_department);
    }

    Ok(())
}

fn parse_usage_locations(
    usage_locations: Vec<Vec<KeyValuePair>>,
    legal_department: &mut LegalDepartment
) -> anyhow::Result<()> {
    for usage_location_items in usage_locations {
        let mut usage_location = UsageLocation::new();
        parse_usage_location(usage_location_items, &mut usage_location)?;
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
    usage_location: &mut UsageLocation
) -> anyhow::Result<()> {
    for (key, values) in items {
        let mut values = values.into_iter();

        let dash_as_none = |s: String| match s.as_str() {
            "-" => None,
            _ => Some(s)
        };

        let mut first = values.next().and_then(dash_as_none);
        let mut second = values.next().and_then(dash_as_none);

        match (key.as_str(), first.take(), second.take()) {
            ("Nutzungsort Lfd. Nr.:", Some(v), _) => {
                let captured = USAGE_LOCATION_RE
                    .captures(&v)
                    .ok_or(anyhow::Error::msg("'Nutzungsort' has invalid format"))?;
                usage_location.serial_no = Some(captured["ser_no"].to_string());
                usage_location.active = Some(&captured["active"] == "aktiv");
                usage_location.real = Some(&captured["real"] == "real");
            }
            ("Bezeichnung:", v, _) => usage_location.name = v,
            ("Rechtszweck:", v, _) => {
                usage_location.legal_scope = v.map(|v| OptionalPair::Single(v))
            }
            ("East und North:", Some(v), _) => usage_location.utm_northing = Some(v.parse()?),
            ("Top. Karte 1:25.000:", Some(num), Some(s)) => {
                usage_location.top_map_1_25000 = Some((num.parse()?, s))
            }
            ("(ETRS89/UTM 32N)", Some(v), _) => usage_location.utm_easting = Some(v.parse()?),
            ("Gemeindegebiet:", Some(num), Some(s)) => {
                usage_location.municipal_area = Some((num.parse()?, s))
            }
            ("Gemarkung, Flur:", None, None) => (),
            ("Gemarkung, Flur:", Some(v), _) => {
                let captured = STRING_NUM_RE
                    .captures(&v)
                    .ok_or(anyhow::Error::msg("'Gemarkung, Flur' has invalid format"))?;
                usage_location.local_sub_district = Some(captured["string"].to_string());
                usage_location.field = Some(captured["num"].parse()?);
            }
            ("Unterhaltungsverband:", Some(num), Some(s)) => {
                usage_location.maintenance_association = Some((num.parse()?, s))
            }
            ("Flurstück:", None, None) => (),
            ("Flurstück:", Some(v), _) => usage_location.plot = Some(v.parse()?),
            ("EU-Bearbeitungsgebiet:", Some(num), Some(s)) => {
                usage_location.eu_survey_area = Some((num.parse()?, s))
            }
            ("Gewässer:", v, _) => usage_location.water_body = v,
            ("Einzugsgebietskennzahl:", Some(num), Some(s)) => {
                usage_location.basin_no = Some((num.parse()?, s))
            }
            ("Verordnungszitat:", v, _) => usage_location.regulation_citation = v,
            ("Erlaubniswert:", Some(v), _) => {
                let mut split = v.rsplitn(3, " ");
                let unit = split
                    .next()
                    .ok_or(anyhow::Error::msg("'Erlaubniswert' has no unit"))?;
                let value = split
                    .next()
                    .ok_or(anyhow::Error::msg("'Erlaubniswert' has no value"))?;
                let kind = split
                    .next()
                    .ok_or(anyhow::Error::msg("'Erlaubniswert' has no specifier"))?;
                match kind {
                    "Entnahmemenge" => {
                        let (time_dim, dim_num) = nlwkn_rs::rate_entry_from_str(value, unit)?;
                        usage_location.withdrawal_rate.insert(time_dim, dim_num);
                    }
                    _ => todo!()
                }
            }

            (key, first, second) => {
                panic!(
                    "invalid entry for the root:\nkey: {key:?}\nfirst: {first:?}\nsecond: \
                     {second:?}"
                );
            }
        }
    }

    Ok(())
}
