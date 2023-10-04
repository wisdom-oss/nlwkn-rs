use std::iter::Peekable;

use itertools::Itertools;

use crate::intermediate::key_value::{KeyValuePair, KeyValueRepr};

#[derive(Debug)]
pub struct GroupedKeyValueRepr {
    pub root: Vec<KeyValuePair>,
    pub departments: Vec<(String, Vec<Vec<KeyValuePair>>)>,
    pub annotation: Option<String>
}

impl From<KeyValueRepr> for GroupedKeyValueRepr {
    fn from(mut key_value_repr: KeyValueRepr) -> Self {
        // take the last keys as annotation of the values of them are empty
        let mut annotation: Vec<String> = Vec::new();
        for (key, values) in key_value_repr.0.iter().rev() {
            match values.is_empty() {
                true => annotation.push(key.clone()),
                false => break
            }
        }

        // remove these keys
        for _ in annotation.iter() {
            key_value_repr.0.pop();
        }

        let annotation = match annotation.is_empty() {
            true => None,
            false => Some(annotation.into_iter().rev().join(" "))
        };

        let mut root = Vec::new();
        let mut key_value_repr_iter = key_value_repr.0.into_iter().peekable();
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
            "Abteilung:" => break,
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
