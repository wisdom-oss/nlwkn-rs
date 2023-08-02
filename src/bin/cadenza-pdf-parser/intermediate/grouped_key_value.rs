use std::iter::Peekable;

use crate::intermediate::key_value::{KeyValuePair, KeyValueRepr};

#[derive(Debug)]
pub struct GroupedKeyValueRepr {
    pub root: Vec<KeyValuePair>,
    pub departments: Vec<(String, Vec<Vec<KeyValuePair>>)>,
    pub annotation: Option<String>
}

impl From<KeyValueRepr> for GroupedKeyValueRepr {
    fn from(mut key_value_repr: KeyValueRepr) -> Self {
        // check if last pair may be the annotation
        let annotation = match key_value_repr.0.pop() {
            None => None,
            Some((key, values)) if values.is_empty() => Some(key),
            Some(entry) => {
                key_value_repr.0.push(entry);
                None
            }
        };

        let mut key_value_repr_iter = key_value_repr.0.into_iter().peekable();

        let mut root = Vec::new();
        while key_value_repr_iter.peek().map(|(key, _)| key != "Abteilung:").unwrap_or(false) {
            if let Some(pair) = key_value_repr_iter.next() {
                root.push(pair);
            }
        }

        let departments = group_departments(&mut key_value_repr_iter);

        Self {
            root,
            departments,
            annotation
        }
    }
}

fn group_departments(
    iter: &mut Peekable<impl Iterator<Item = KeyValuePair>>
) -> Vec<(String, Vec<Vec<KeyValuePair>>)> {
    let mut departments = Vec::new();
    while let Some(next) = iter.next() {
        if next.0.as_str() != "Abteilung:" {
            panic!(
                "did not get 'Abteilung', only pass to this function of next element is \
                 'Abteilung'"
            );
        }

        departments.push((next.1.join(""), group_usage_locations(iter)));
    }

    departments
}

fn group_usage_locations(
    iter: &mut Peekable<impl Iterator<Item = KeyValuePair>>
) -> Vec<Vec<KeyValuePair>> {
    let mut usage_locations = Vec::new();
    let mut usage_location = Vec::new();

    while let Some(peek) = iter.peek() {
        match peek.0.as_str() {
            "Abteilung:" => return usage_locations,
            "Nutzungsort Lfd. Nr.:" => {
                if !usage_location.is_empty() {
                    usage_locations.push(usage_location);
                    usage_location = Vec::new();
                }
            }
            _ => ()
        }

        let next = iter.next().expect("cannot peek if next is none");
        usage_location.push(next);
    }

    usage_locations.push(usage_location);
    usage_locations
}
