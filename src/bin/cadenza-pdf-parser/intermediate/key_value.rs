use crate::intermediate::text_block::{TextBlock, TextBlockRepr};

pub struct KeyValueRepr(pub Vec<(String, Vec<String>)>);
pub type KeyValuePair = (String, Vec<String>);

impl From<TextBlockRepr> for KeyValueRepr {
    fn from(text_block_repr: TextBlockRepr) -> Self {
        let mut pairs = Vec::new();

        let mut entry: Option<(String, Vec<String>)> = None;
        for text_block in text_block_repr.0.into_iter() {
            let TextBlock { content: Some(content), font_family: Some(font_family), .. } = text_block else {
                continue;
            };

            match (font_family.as_str(), entry.as_mut()) {
                ("F1", None) => entry = Some((content, Vec::new())),
                ("F3" | "F2", Some(entry)) => entry.1.push(content),
                ("F1", Some(_)) => {
                    pairs.push(entry.take().expect("is some"));
                    entry = Some((content, Vec::new()))
                }
                _ => (),
            }
        }

        if let Some(entry) = entry {
            pairs.push(entry);
        }

        KeyValueRepr(pairs)
    }
}
