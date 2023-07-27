use crate::intermediate::grouped_key_value::GroupedKeyValueRepr;
use crate::intermediate::key_value::KeyValueRepr;
use crate::intermediate::text_block::{TextBlock, TextBlockRepr};
use lopdf::Document;
use nlwkn_rs::{LegalDepartment, UsageLocation, WaterRight, WaterRightNo};
use std::env;

mod intermediate;
mod parse;

fn main() -> anyhow::Result<()> {
    let document = lopdf::Document::load(
        env::args()
            .nth(1)
            .ok_or(anyhow::Error::msg("no argument passed"))?,
    )?;
    let text_block_repr = TextBlockRepr::try_from(document.clone())?;
    let key_value_repr = KeyValueRepr::from(text_block_repr);

    for (key, values) in key_value_repr.0.iter() {
        print!("{}: ", console::style(key).magenta());
        for value in values {
            print!("{}, ", console::style(value).cyan());
        }
        println!()
    }

    let water_right = parse::parse_document(287209, document)?;
    dbg!(water_right);

    Ok(())
}
