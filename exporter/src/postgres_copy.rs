use std::io;
use std::io::Write;

use nlwkn::helper_types::{Duration, Quantity, Rate, SingleOrPair};
use nlwkn::{DamTargets, LandRecord, LegalDepartmentAbbreviation};

pub trait PostgresCopy {
    fn copy_to(&self, writer: &mut impl Write) -> io::Result<()>;
}

macro_rules! impl_postgres_copy {
    ($($type:ty),*) => {$(
        impl PostgresCopy for $type {
            fn copy_to(&self, writer: &mut impl Write) -> io::Result<()> {
                write!(writer, "{}", self)
            }
        }
    )*};
}

impl_postgres_copy!(usize, u8, u16, u32, u64, u128);
impl_postgres_copy!(isize, i8, i16, i32, i64, i128);
impl_postgres_copy!(f32, f64);

impl PostgresCopy for str {
    fn copy_to(&self, writer: &mut impl Write) -> io::Result<()> {
        write!(writer, "'{}'", self)
    }
}

/// Represents the `water_rights.numeric_keyed_value` in the Postgres DB.
impl PostgresCopy for SingleOrPair<u64, String> {
    fn copy_to(&self, writer: &mut impl Write) -> io::Result<()> {
        write!(writer, "(")?;
        match self {
            SingleOrPair::Single(key) => write!(writer, "key := {key}")?,
            SingleOrPair::Pair(key, name) => write!(writer, "key := {key}, name := {name:?}")?
        }
        write!(writer, ")::water_rights.rate")?;
        Ok(())
    }
}

impl PostgresCopy for Quantity {
    fn copy_to(&self, writer: &mut impl Write) -> io::Result<()> {
        let Quantity { value, unit } = self;
        write!(
            writer,
            "(value := {value}, unit := '{unit}')::water_rights.quantity"
        )
    }
}

impl PostgresCopy for Rate<f64> {
    fn copy_to(&self, writer: &mut impl Write) -> io::Result<()> {
        let Rate { value, unit, per } = self;
        write!(writer, "(value := {value}, unit := '{unit}', per := ")?;
        per.copy_to(writer)?;
        write!(writer, ")::water_rights.rate")?;
        Ok(())
    }
}

impl PostgresCopy for Duration {
    fn copy_to(&self, writer: &mut impl Write) -> io::Result<()> {
        match self {
            Duration::Seconds(s) => write!(writer, "'{s} seconds'"),
            Duration::Minutes(m) => write!(writer, "'{m} minutes'"),
            Duration::Hours(h) => write!(writer, "'{h} hours'"),
            Duration::Days(d) => write!(writer, "'{d} days'"),
            Duration::Weeks(w) => write!(writer, "'{} days'", w * 7.0),
            Duration::Months(m) => write!(writer, "'{m} months'"),
            Duration::Years(y) => write!(writer, "'{y} years'")
        }
    }
}

impl PostgresCopy for DamTargets {
    fn copy_to(&self, writer: &mut impl Write) -> io::Result<()> {
        let DamTargets {
            default,
            steady,
            max,
            ..
        } = self;
        let fields = [(r#""default""#, default), ("steady", steady), ("max", max)];

        write!(writer, "(")?;
        let mut fields = fields
            .iter()
            .filter_map(|(f, q)| q.as_ref().map(|q| (f, q)))
            .peekable();
        while let Some((field, quantity)) = fields.next() {
            write!(writer, "{field} := ")?;
            quantity.copy_to(writer)?;
            if fields.peek().is_some() {
                write!(writer, ", ")?;
            }
        }
        write!(writer, ")::water_rights.dam_target")?;
        Ok(())
    }
}

impl PostgresCopy for LandRecord {
    fn copy_to(&self, writer: &mut impl Write) -> io::Result<()> {
        let LandRecord {district, field} = self;
        write!(writer, "(district := '{district}', field := '{field}')::water_rights.land_record")
    }
}

/// Represents the `water_rights.injection_limit` in the Postgres DB.
impl PostgresCopy for (String, Quantity) {
    fn copy_to(&self, writer: &mut impl Write) -> io::Result<()> {
        let (substance, quantity) = self;
        write!(writer, "(substance := '{substance}', quantity := ")?;
        quantity.copy_to(writer)?;
        write!(writer, ")::water_rights.injection_limit")?;
        Ok(())
    }
}

impl PostgresCopy for LegalDepartmentAbbreviation {
    fn copy_to(&self, writer: &mut impl Write) -> io::Result<()> {
        match self {
            LegalDepartmentAbbreviation::A => write!(writer, "'A'"),
            LegalDepartmentAbbreviation::B => write!(writer, "'B'"),
            LegalDepartmentAbbreviation::C => write!(writer, "'C'"),
            LegalDepartmentAbbreviation::D => write!(writer, "'D'"),
            LegalDepartmentAbbreviation::E => write!(writer, "'E'"),
            LegalDepartmentAbbreviation::F => write!(writer, "'F'"),
            LegalDepartmentAbbreviation::K => write!(writer, "'K'"),
            LegalDepartmentAbbreviation::L => write!(writer, "'L'"),
        }
    }
}

impl<T> PostgresCopy for [T] where T: PostgresCopy {
    fn copy_to(&self, writer: &mut impl Write) -> io::Result<()> {
        write!(writer, "{{")?;
        let mut iter = self.iter().peekable();
        while let Some(next) = iter.next() {
            next.copy_to(writer)?;
            if iter.peek().is_some() {
                write!(writer, ",")?;
            }
        }
        write!(writer, "}}")?;
        Ok(())
    }
}
