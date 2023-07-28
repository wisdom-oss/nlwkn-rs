use lopdf::Document;
use nlwkn_rs::{WaterRight, WaterRightNo};

use crate::intermediate::grouped_key_value::GroupedKeyValueRepr;
use crate::intermediate::key_value::KeyValueRepr;
use crate::intermediate::text_block::TextBlockRepr;

mod departments;
mod root;

pub fn parse_document(
    water_right_no: WaterRightNo,
    document: Document
) -> anyhow::Result<WaterRight> {
    let text_block_repr = TextBlockRepr::try_from(document)?;
    let key_value_repr = KeyValueRepr::from(text_block_repr);
    let GroupedKeyValueRepr {
        root,
        departments,
        annotation
    } = key_value_repr.into();

    let mut water_right = WaterRight::new(water_right_no);
    root::parse_root(root, &mut water_right)?;
    departments::parse_departments(departments, &mut water_right)?;
    water_right.annotation = annotation;

    Ok(water_right)
}
