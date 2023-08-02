use lopdf::content::Operation;
use lopdf::{Object, StringFormat};

const ENCODING: &str = "WinAnsiEncoding";

#[derive(Debug)]
pub struct TextBlockRepr(pub Vec<TextBlock>);

#[derive(Debug, Default)]
pub struct TextBlock {
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
    pub fill_color: Option<(f32, f32, f32)>,
    pub content: Option<String>
}

impl TryFrom<lopdf::Document> for TextBlockRepr {
    type Error = anyhow::Error;

    fn try_from(document: lopdf::Document) -> anyhow::Result<Self> {
        let mut text_blocks = Vec::new();
        let mut text_block: Option<TextBlock> = None;
        for page_object_id in document.page_iter() {
            for Operation { operator, operands } in
                document.get_and_decode_page_content(page_object_id)?.operations.iter()
            {
                match (operator.as_str(), text_block.as_mut()) {
                    // expected states
                    ("BT", None) => text_block = Some(TextBlock::default()),
                    ("Tm", Some(text_block)) => handle_tm(text_block, operands)?,
                    ("Tf", Some(text_block)) => handle_tf(text_block, operands),
                    ("rg", Some(text_block)) => handle_rg(text_block, operands),
                    ("Tj", Some(text_block)) => handle_tj(text_block, operands),
                    ("ET", Some(_)) => {
                        text_blocks.push(text_block.take().expect("text block is some"));
                    }

                    // unexpected states
                    ("BT", Some(_)) => {
                        eprintln!("warning: text block did already begin, got '{operator}'")
                    }
                    ("Tm" | "Tf" | "Tj" | "ET", None) => {
                        eprintln!("warning: no text block opened, got '{operator}'")
                    }

                    // ignore rest
                    _ => ()
                }
            }
        }

        Ok(TextBlockRepr(text_blocks))
    }
}

#[inline]
fn handle_tm(text_block: &mut TextBlock, operands: &[Object]) -> anyhow::Result<()> {
    // only take the first x and y coordinates
    if text_block.x.is_some() || text_block.y.is_some() {
        return Ok(());
    }

    text_block.x = match operands.get(4) {
        Some(Object::Real(r)) => Some(*r),
        Some(Object::Integer(i)) => Some(*i as f32),
        Some(_) => {
            eprintln!("warning: expected number for 'Tm' operand[4]");
            None
        }
        _ => None
    };

    text_block.y = match operands.get(5) {
        Some(Object::Real(r)) => Some(*r),
        Some(Object::Integer(i)) => Some(*i as f32),
        Some(_) => {
            eprintln!("warning: expected number for 'Tm' operand[5]");
            None
        }
        _ => None
    };

    Ok(())
}

#[inline]
fn handle_tf(text_block: &mut TextBlock, operands: &[Object]) {
    // take only the first font configuration
    if text_block.font_family.is_some() || text_block.font_size.is_some() {
        return;
    }

    text_block.font_family = match operands.get(0) {
        Some(Object::String(s, StringFormat::Literal)) => {
            Some(lopdf::Document::decode_text(Some(ENCODING), s))
        }
        Some(Object::String(_, _)) => {
            eprintln!("warning: cannot handle non-string-literal for 'Tf' operand[0]");
            None
        }
        Some(Object::Name(n)) => Some(lopdf::Document::decode_text(Some(ENCODING), n)),
        Some(_) => {
            eprintln!("warning: expected string for 'Tf' operand[0]");
            None
        }
        _ => None
    };

    text_block.font_size = match operands.get(1) {
        Some(Object::Real(r)) => Some(*r),
        Some(Object::Integer(i)) => Some(*i as f32),
        Some(_) => {
            eprintln!("warning: expected number for 'Tf' operand[1]");
            None
        }
        _ => None
    };
}

#[inline]
fn handle_rg(text_block: &mut TextBlock, operands: &[Object]) {
    // take only the first fill color
    if text_block.fill_color.is_some() {
        return;
    }

    let r = match operands.get(0) {
        Some(Object::Real(r)) => Some(*r),
        Some(Object::Integer(i)) => Some(*i as f32),
        Some(_) => {
            eprintln!("warning: expected number for 'rg' operand[0]");
            None
        }
        _ => None
    };

    let g = match operands.get(1) {
        Some(Object::Real(r)) => Some(*r),
        Some(Object::Integer(i)) => Some(*i as f32),
        Some(_) => {
            eprintln!("warning: expected number for 'rg' operand[1]");
            None
        }
        _ => None
    };

    let b = match operands.get(0) {
        Some(Object::Real(r)) => Some(*r),
        Some(Object::Integer(i)) => Some(*i as f32),
        Some(_) => {
            eprintln!("warning: expected number for 'rg' operand[2]");
            None
        }
        _ => None
    };

    if let (Some(r), Some(g), Some(b)) = (r, g, b) {
        text_block.fill_color = Some((r, g, b));
    }
}

#[inline]
fn handle_tj(text_block: &mut TextBlock, operands: &[Object]) {
    let mut content = String::new();

    for operand in operands {
        match operand {
            Object::String(s, StringFormat::Literal) => {
                content.push_str(lopdf::Document::decode_text(Some(ENCODING), s).as_str());
            }
            Object::String(_, _) => {
                eprintln!("warning: expected string literal for 'Tj'");
            }
            _ => ()
        }
    }

    text_block.content = match (text_block.content.take(), !content.is_empty()) {
        (Some(prev), true) => match prev.chars().last() {
            // this is only a heuristic
            Some('-' | '/') => Some(format!("{prev}{content}")),
            Some('.' | ';') => Some(format!("{prev}\n{content}")),
            _ => Some(format!("{prev} {content}"))
        },
        (Some(prev), false) => Some(prev),
        (None, true) => Some(content),
        (None, false) => None
    };
}
