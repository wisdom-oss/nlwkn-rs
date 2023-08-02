use nlwkn_rs::WaterRight;

use crate::intermediate::key_value::KeyValuePair;

pub fn parse_root(items: Vec<KeyValuePair>, water_right: &mut WaterRight) -> anyhow::Result<()> {
    for (key, values) in items {
        let mut value = values.into_iter().next();
        match (key.as_str(), value.take()) {
            ("Wasserbuchbehörde", v) => water_right.water_authority = v,
            ("Kennziffer", Some(v)) => {
                let mut split = v.rsplitn(2, " ");
                water_right.state = split.next().map(|state| state[1..state.len() - 1].to_string());
                water_right.external_identifier = split.next().map(|ext_id| ext_id.to_string());
            }
            ("erteilt durch /", _) => (),
            ("eingetragen durch:", v) => water_right.registering_authority = v,
            ("abweichend", _) => (),
            ("erteilt durch:", v) => water_right.granting_authority = v,
            ("erteilt am:", v) => water_right.valid_from = v,
            ("erstmalig ertellt am:", v) => water_right.first_grant = v,
            ("Aktenzeichen:", v) => water_right.file_reference = v,
            ("Das Recht ist befristet bis", v) => water_right.valid_to = v,
            ("und betrifft Rechtsabteilungen", _) => (),
            ("Betreff:", v) => water_right.subject = v,
            (key, value) => {
                panic!("invalid entry for the root:\nkey: {key:?}\nvalue: {value:?}");
            }
        }
    }

    Ok(())
}
