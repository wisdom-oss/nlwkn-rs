use std::io;
use std::io::Write;

use nlwkn::helper_types::{Duration, OrFallback, Quantity, Rate, SingleOrPair};
use nlwkn::{DamTargets, LandRecord, LegalDepartmentAbbreviation, PHValues};

use crate::export::InjectionLimit;

pub trait PostgresCopy {
    fn copy_to(&self, writer: &mut impl io::Write) -> io::Result<()>;
}

pub fn iter_copy_to<T, I>(iter: I, writer: &mut impl io::Write) -> io::Result<()>
where
    I: Iterator<Item = T>,
    T: PostgresCopy
{
    let mut iter = iter.peekable();
    if iter.peek().is_none() {
        return write!(writer, r"\N");
    }

    writer.write(b"{")?;
    while let Some(it) = iter.next() {
        writer.write(b"\"")?;
        it.copy_to(writer)?;
        writer.write(b"\"")?;
        if iter.peek().is_some() {
            writer.write(b",")?;
        }
    }
    writer.write(b"}")?;
    Ok(())
}

pub fn utm_point_copy_to(
    easting: u64,
    northing: u64,
    writer: &mut impl io::Write
) -> io::Result<()> {
    write!(writer, "POINT({easting} {northing})")
}

macro_rules! impl_postgres_copy {
    ($($type:ty),*) => {$(
        impl PostgresCopy for $type {
            fn copy_to(&self, writer: &mut impl io::Write) -> io::Result<()> {
                write!(writer, "{}", self)
            }
        }
    )*};
}

impl_postgres_copy!(usize, u8, u16, u32, u64, u128);
impl_postgres_copy!(isize, i8, i16, i32, i64, i128);
impl_postgres_copy!(f32, f64);
impl_postgres_copy!(bool);

impl PostgresCopy for str {
    fn copy_to(&self, writer: &mut impl io::Write) -> io::Result<()> {
        let escape_brackets = self.contains('(') || self.contains(')');
        // if escape_brackets {
        //     writer.write(br#"\""#)?;
        // }
        for c in self.chars() {
            match c {
                '\n' => write!(writer, r"\n")?,
                '\t' => write!(writer, r"\t")?,
                _ => write!(writer, "{}", c)?
            }
        }
        // if escape_brackets {
        //     writer.write(br#"\""#)?;
        // }
        Ok(())
    }
}

impl PostgresCopy for String {
    fn copy_to(&self, writer: &mut impl io::Write) -> io::Result<()> {
        self.as_str().copy_to(writer)
    }
}

/// Represents the `water_rights.numeric_keyed_value` in the Postgres DB.
impl PostgresCopy for SingleOrPair<u64, String> {
    fn copy_to(&self, writer: &mut impl io::Write) -> io::Result<()> {
        let (key, name) = match self {
            SingleOrPair::Single(key) => (key, None),
            SingleOrPair::Pair(key, name) => (key, Some(name))
        };
        writer.write(b"(")?;
        key.copy_to(writer)?;
        writer.write(br#",\""#)?;
        name.copy_to(writer)?;
        writer.write(br#"\")"#)?;
        Ok(())
    }
}

/// Represents the `water_rights.numeric_keyed_value` in the Postgres DB.
impl PostgresCopy for (u64, String) {
    fn copy_to(&self, writer: &mut impl io::Write) -> io::Result<()> {
        let (key, name) = self;
        writer.write(b"(")?;
        key.copy_to(writer)?;
        writer.write(br#",\""#)?;
        name.copy_to(writer)?;
        writer.write(br#"\")"#)?;
        Ok(())
    }
}

impl PostgresCopy for Quantity {
    fn copy_to(&self, writer: &mut impl io::Write) -> io::Result<()> {
        let Quantity { value, unit } = self;
        writer.write(b"(")?;
        value.copy_to(writer)?;
        writer.write(b",")?;
        unit.copy_to(writer)?;
        writer.write(b")")?;
        Ok(())
    }
}

impl PostgresCopy for Rate<f64> {
    fn copy_to(&self, writer: &mut impl io::Write) -> io::Result<()> {
        let Rate { value, unit, per } = self;
        writer.write(b"(")?;
        value.copy_to(writer)?;
        writer.write(b",")?;
        unit.copy_to(writer)?;
        writer.write(b",")?;
        per.copy_to(writer)?;
        writer.write(b")")?;
        Ok(())
    }
}

impl PostgresCopy for Duration {
    fn copy_to(&self, writer: &mut impl io::Write) -> io::Result<()> {
        match self {
            Duration::Seconds(s) => write!(writer, "{s} seconds"),
            Duration::Minutes(m) => write!(writer, "{m} minutes"),
            Duration::Hours(h) => write!(writer, "{h} hours"),
            Duration::Days(d) => write!(writer, "{d} days"),
            Duration::Weeks(w) => write!(writer, "{} days", w * 7.0),
            Duration::Months(m) => write!(writer, "{m} months"),
            Duration::Years(y) => write!(writer, "{y} years")
        }
    }
}

impl PostgresCopy for DamTargets {
    fn copy_to(&self, writer: &mut impl io::Write) -> io::Result<()> {
        let DamTargets {
            default,
            steady,
            max,
            ..
        } = self;
        if let (None, None, None) = (default, steady, max) {
            return write!(writer, r"\N");
        }

        writer.write(b"(")?;
        default.copy_to(writer)?;
        writer.write(b",")?;
        steady.copy_to(writer)?;
        writer.write(b",")?;
        max.copy_to(writer)?;
        writer.write(b")")?;
        Ok(())
    }
}

impl PostgresCopy for LandRecord {
    fn copy_to(&self, writer: &mut impl io::Write) -> io::Result<()> {
        let LandRecord { district, field } = self;
        writer.write(b"(")?;
        district.copy_to(writer)?;
        writer.write(b",")?;
        field.copy_to(writer)?;
        writer.write(b")")?;
        Ok(())
    }
}

/// Represents the `water_rights.injection_limit` in the Postgres DB.
impl PostgresCopy for (String, Quantity) {
    fn copy_to(&self, writer: &mut impl io::Write) -> io::Result<()> {
        writer.write(b"(")?;
        self.0.copy_to(writer)?;
        writer.write(b",")?;
        self.1.copy_to(writer)?;
        writer.write(b")")?;
        Ok(())
    }
}

impl PostgresCopy for LegalDepartmentAbbreviation {
    fn copy_to(&self, writer: &mut impl io::Write) -> io::Result<()> {
        match self {
            LegalDepartmentAbbreviation::A => write!(writer, "A"),
            LegalDepartmentAbbreviation::B => write!(writer, "B"),
            LegalDepartmentAbbreviation::C => write!(writer, "C"),
            LegalDepartmentAbbreviation::D => write!(writer, "D"),
            LegalDepartmentAbbreviation::E => write!(writer, "E"),
            LegalDepartmentAbbreviation::F => write!(writer, "F"),
            LegalDepartmentAbbreviation::K => write!(writer, "K"),
            LegalDepartmentAbbreviation::L => write!(writer, "L")
        }
    }
}

impl PostgresCopy for PHValues {
    fn copy_to(&self, writer: &mut impl Write) -> io::Result<()> {
        let PHValues { min, max } = self;
        match min {
            Some(min) => write!(writer, "[{min}")?,
            None => write!(writer, "(-infinity")?
        };
        write!(writer, ",")?;
        match max {
            Some(max) => write!(writer, "{max}]")?,
            None => write!(writer, "infinity)")?
        };
        Ok(())
    }
}

impl<'il> PostgresCopy for InjectionLimit<'il> {
    fn copy_to(&self, writer: &mut impl Write) -> io::Result<()> {
        let InjectionLimit {
            substance,
            quantity
        } = self;
        writer.write(br#"(\\""#)?;
        substance.copy_to(writer)?;
        writer.write(br#"\\",\\""#)?;
        quantity.copy_to(writer)?;
        writer.write(br#"\\")"#)?;
        Ok(())
    }
}

impl<T> PostgresCopy for &T
where
    T: PostgresCopy
{
    fn copy_to(&self, writer: &mut impl Write) -> io::Result<()> {
        (*self).copy_to(writer)
    }
}

impl<T> PostgresCopy for [T]
where
    T: PostgresCopy
{
    fn copy_to(&self, writer: &mut impl io::Write) -> io::Result<()> {
        writer.write(b"{")?;
        let mut iter = self.iter().peekable();
        while let Some(next) = iter.next() {
            next.copy_to(writer)?;
            if iter.peek().is_some() {
                write!(writer, ",")?;
            }
        }
        writer.write(b"}")?;
        Ok(())
    }
}

impl<T> PostgresCopy for Option<T>
where
    T: PostgresCopy
{
    fn copy_to(&self, writer: &mut impl Write) -> io::Result<()> {
        match self {
            None => write!(writer, r"\N"),
            Some(v) => v.copy_to(writer)
        }
    }
}

impl<T> PostgresCopy for OrFallback<T>
where
    T: PostgresCopy
{
    fn copy_to(&self, writer: &mut impl Write) -> io::Result<()> {
        match self {
            OrFallback::Expected(v) => v.copy_to(writer),
            OrFallback::Fallback(_) => writer.write(br"\N").map(|_| ())
        }
    }
}

impl<T> PostgresCopy for (T, T)
where
    T: PostgresCopy
{
    fn copy_to(&self, writer: &mut impl Write) -> io::Result<()> {
        writer.write(b"{")?;
        self.0.copy_to(writer)?;
        writer.write(b",")?;
        self.1.copy_to(writer)?;
        writer.write(b"}")?;
        Ok(())
    }
}

pub struct IsoDate<'s>(pub &'s str);

impl PostgresCopy for IsoDate<'_> {
    fn copy_to(&self, writer: &mut impl Write) -> io::Result<()> {
        match self.0 {
            "unbefristet" => write!(writer, "infinity"),
            s => write!(writer, "{s}")
        }
    }
}
