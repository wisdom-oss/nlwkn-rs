use crate::intermediate::text_block::{TextBlock, TextBlockRepr};

pub struct KeyValueRepr(pub Vec<(String, Vec<String>)>);
pub type KeyValuePair = (String, Vec<String>);

impl From<TextBlockRepr> for KeyValueRepr {
    fn from(text_block_repr: TextBlockRepr) -> Self {
        type Pair = (String, Vec<(u32, String)>);
        let mut pairs: Vec<Pair> = Vec::new();

        for page in text_block_repr.0.into_iter() {
            let mut entry: Option<Pair> = None;
            for text_block in page.into_iter() {
                let TextBlock {
                    content: Some(content),
                    font_family: Some(font_family),
                    x,
                    ..
                } = text_block
                    else {
                        continue;
                    };

                let Some(x) = x else {
                    panic!("x missing");
                };
                let x = x.floor() as u32;

                match (font_family.as_str(), entry.as_mut()) {
                    ("F1", None) => entry = Some((content, Vec::new())),
                    ("F3" | "F2", None) => {
                        // found value without key on page
                        // iterate on pairs in reverse to find where the value could belong and
                        // add it
                        let s = pairs.iter_mut().rev().map(|(_, values)| values).flatten().find(|(key_x, _)| *key_x == x).expect("line break without existing previous line?");
                        s.1.push(' ');
                        s.1.push_str(&content);
                    },
                    ("F3" | "F2", Some(entry)) => entry.1.push((x, content)),
                    ("F1", Some(_)) => {
                        pairs.push(entry.take().expect("is some"));
                        entry = Some((content, Vec::new()))
                    }
                    _ => ()
                }
            }

            if let Some(entry) = entry {
                pairs.push(entry);
            }
        }

        KeyValueRepr(pairs.into_iter().map(|(key, values)| (key, values.into_iter().map(|(_, v)| v).collect())).collect())
    }
}
