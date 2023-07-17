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
