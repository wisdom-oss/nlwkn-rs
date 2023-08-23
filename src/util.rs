macro_rules! data_structs {
    {$(
        $(#[$struct_attr:meta])*
        struct $struct:ident {
            $(
                $(#[$field_attr:meta])*
                $field:ident
                $(? $([$($_:tt)* $optional:tt])?)?:
                $type:ty,
            )*
        }
    )+} => {
        $(
            #[serde_with::skip_serializing_none]
            #[derive(Debug, serde::Serialize, serde::Deserialize)]
            $(#[$struct_attr])*
            pub struct $struct {
                $(
                    $(#[$field_attr])*
                    pub $field:
                        $($(if $optional)? Option<)?
                            $type
                        $($(if $optional)? >)?
                ),*
            }
        )+
    }
}

pub(crate) use data_structs;

pub trait StringOption {
    fn sanitize(self) -> Option<String>;
}

impl StringOption for Option<String> {
    fn sanitize(self) -> Option<String> {
        match self.as_ref() {
            None => None,
            Some(s) => match s.trim() {
                "" | "-" => None,
                s_trim if s_trim == s => self,
                s_trim => Some(s_trim.to_owned())
            }
        }
    }
}

pub trait OptionUpdate<T: Clone> {
    fn update_if_none(&mut self, other: Option<T>);
    fn update_if_none_clone(&mut self, other: Option<&T>);
    fn update_if_none_with<F>(&mut self, other: F)
    where
        F: FnOnce() -> Option<T>;
}

impl<T: Clone> OptionUpdate<T> for Option<T> {
    fn update_if_none(&mut self, other: Option<T>) {
        if self.is_none() {
            *self = other;
        }
    }

    fn update_if_none_clone(&mut self, other: Option<&T>) {
        if self.is_none() {
            *self = other.cloned();
        }
    }

    fn update_if_none_with<F>(&mut self, other: F)
    where
        F: FnOnce() -> Option<T>
    {
        if self.is_none() {
            *self = other();
        }
    }
}

pub fn zero_is_none(value: u64) -> Option<u64> {
    match value {
        0 => None,
        _ => Some(value)
    }
}

pub trait Near {
    type Rhs;
    const THRESHOLD: f64;

    fn is_near(&self, other: &Self::Rhs) -> bool;
}

impl Near for f64 {
    type Rhs = f64;

    const THRESHOLD: Self = 0.001;

    fn is_near(&self, other: &Self::Rhs) -> bool {
        (self - other).abs() < Self::THRESHOLD
    }
}
