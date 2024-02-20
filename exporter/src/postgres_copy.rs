use std::collections::BTreeSet;
use std::hash::Hasher;
use std::io;
use std::io::Write;

use nlwkn::helper_types::{Duration, OrFallback, Quantity, Rate, SingleOrPair};
use nlwkn::{DamTargets, LandRecord, LegalDepartmentAbbreviation, PHValues, RateRecord};

use crate::export::{InjectionLimit, IsoDate, UtmPoint};

pub trait PostgresCopy {
    /// Write `self` on a writer for the `COPY` instruction from PostgreSQL.
    ///
    /// Implementations of this should avoid allocating heap memory for a fast
    /// copy operation.
    ///
    /// The `depth` is used to escape quotes and the correct level. It acts as a
    /// counter how many escapes need to happen in a quote.
    fn copy_to(&self, writer: &mut impl io::Write, depth: usize) -> io::Result<()>;
}

/// Separate trait of [`PostgresCopy`] to avoid upstream implementation
/// conflicts.
pub trait IterPostgresCopy {
    fn copy_to(self, writer: &mut impl io::Write, depth: usize) -> io::Result<()>;
}

/// Quotes the passed `to_copy` borrow.
///
/// This writes quotes to `writer` and escapes them according to the `depth`
/// value. The `depth` value is the same as the amount of escapes.
/// As this function will quote, it will also increase the depth level.
///
/// Between the quotes will be written whatever `write_op` does with the
/// `writer`.
fn quoted<F, W>(write_op: F, writer: &mut W, depth: usize) -> io::Result<()>
where
    F: FnOnce(&mut W, usize) -> io::Result<()>,
    W: io::Write
{
    for _ in 0..depth {
        writer.write(br"\")?;
    }
    writer.write(b"\"")?;

    write_op(writer, depth + 1)?;

    for _ in 0..depth {
        writer.write(br"\")?;
    }
    writer.write(b"\"")?;

    Ok(())
}

struct Null;

impl PostgresCopy for Null {
    fn copy_to(&self, writer: &mut impl Write, depth: usize) -> io::Result<()> {
        write!(writer, r"\N")
    }
}

impl<I, T> IterPostgresCopy for I
where
    I: Iterator<Item = T>,
    T: PostgresCopy
{
    fn copy_to(self, writer: &mut impl io::Write, depth: usize) -> io::Result<()> {
        let mut iter = self.peekable();
        if iter.peek().is_none() {
            return Null.copy_to(writer, depth);
        }

        writer.write(b"{")?;
        while let Some(it) = iter.next() {
            quoted(|w, d| it.copy_to(w, d), writer, depth)?;
            if iter.peek().is_some() {
                writer.write(b",")?;
            }
        }
        writer.write(b"}")?;

        Ok(())
    }
}

macro_rules! impl_postgres_copy {
    ($($type:ty),*) => {$(
        impl PostgresCopy for $type {
            fn copy_to(&self, writer: &mut impl io::Write, depth: usize) -> io::Result<()> {
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
    fn copy_to(&self, writer: &mut impl io::Write, depth: usize) -> io::Result<()> {
        quoted(|w, d| write!(w, "{}", self.escape_debug()), writer, depth)
    }
}

impl PostgresCopy for String {
    fn copy_to(&self, writer: &mut impl io::Write, depth: usize) -> io::Result<()> {
        self.as_str().copy_to(writer, depth)
    }
}

impl<T> PostgresCopy for &T
where
    T: PostgresCopy
{
    fn copy_to(&self, writer: &mut impl io::Write, depth: usize) -> io::Result<()> {
        (*self).copy_to(writer, depth)
    }
}

impl<T> PostgresCopy for Option<T>
where
    T: PostgresCopy
{
    fn copy_to(&self, writer: &mut impl io::Write, depth: usize) -> io::Result<()> {
        match self {
            None => Null.copy_to(writer, depth),
            Some(v) => v.copy_to(writer, depth)
        }
    }
}

impl<T> PostgresCopy for (T, T)
where
    T: PostgresCopy
{
    fn copy_to(&self, writer: &mut impl io::Write, depth: usize) -> io::Result<()> {
        write!(writer, "{{")?;
        self.0.copy_to(writer, depth)?;
        write!(writer, ",")?;
        self.1.copy_to(writer, depth)?;
        write!(writer, "}}")?;
        Ok(())
    }
}

macro_rules! composite {
    // Match the macro invocation pattern with writer, depth, and a list of elements
    ($writer:expr, $depth:expr, [$first:expr, $($rest:expr),* $(,)?]) => {{
        write!($writer, "(")?;
        // Process the first element without a leading comma
        $first.copy_to($writer, $depth)?;
        // Process the rest of the elements, if any, with leading commas
        $(
            write!($writer, ",")?;
            $rest.copy_to($writer, $depth)?;
        )*
        write!($writer, ")")?;
    }};
}

/// Represents the `water_rights.injection_limit` in the Postgres DB.
impl PostgresCopy for (String, Quantity) {
    fn copy_to(&self, writer: &mut impl io::Write, depth: usize) -> io::Result<()> {
        composite!(writer, depth, [self.0, self.1]);
        Ok(())
    }
}

/// Represents the `water_rights.numeric_keyed_value` in the Postgres DB.
impl PostgresCopy for (u64, String) {
    fn copy_to(&self, writer: &mut impl io::Write, depth: usize) -> io::Result<()> {
        composite!(writer, depth, [self.0, &self.1]);
        Ok(())
    }
}

impl PostgresCopy for UtmPoint {
    fn copy_to(&self, writer: &mut impl Write, depth: usize) -> io::Result<()> {
        let UtmPoint { easting, northing } = self;
        write!(writer, "POINT({easting} {northing})")
    }
}

/// Represents the `water_rights.numeric_keyed_value` in the Postgres DB.
impl PostgresCopy for SingleOrPair<u64, String> {
    fn copy_to(&self, writer: &mut impl io::Write, depth: usize) -> io::Result<()> {
        let (key, name) = match self {
            SingleOrPair::Single(key) => (key, None),
            SingleOrPair::Pair(key, name) => (key, Some(name))
        };
        composite!(writer, depth, [key, name]);
        Ok(())
    }
}

impl PostgresCopy for Quantity {
    fn copy_to(&self, writer: &mut impl io::Write, depth: usize) -> io::Result<()> {
        composite!(writer, depth, [self.value, self.unit]);
        Ok(())
    }
}

impl PostgresCopy for Rate<f64> {
    fn copy_to(&self, writer: &mut impl io::Write, depth: usize) -> io::Result<()> {
        composite!(writer, depth, [self.value, self.unit, self.per]);
        Ok(())
    }
}

impl PostgresCopy for RateRecord {
    fn copy_to(&self, writer: &mut impl Write, depth: usize) -> io::Result<()> {
        self.iter()
            .filter_map(|or_fallback| match or_fallback {
                OrFallback::Expected(v) => Some(v),
                OrFallback::Fallback(_) => None
            })
            .copy_to(writer, depth)
    }
}

impl PostgresCopy for Duration {
    fn copy_to(&self, writer: &mut impl io::Write, depth: usize) -> io::Result<()> {
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
    fn copy_to(&self, writer: &mut impl io::Write, depth: usize) -> io::Result<()> {
        if self.default.is_none() && self.steady.is_none() && self.max.is_none() {
            return Null.copy_to(writer, depth);
        }
        composite!(writer, depth, [self.default, self.steady, self.max]);
        Ok(())
    }
}

impl PostgresCopy for OrFallback<LandRecord> {
    fn copy_to(&self, writer: &mut impl io::Write, depth: usize) -> io::Result<()> {
        match self {
            OrFallback::Expected(lr) => composite!(writer, depth, [lr.district, lr.field, Null]),
            OrFallback::Fallback(s) => composite!(writer, depth, [Null, Null, s])
        }
        Ok(())
    }
}

impl PostgresCopy for LegalDepartmentAbbreviation {
    fn copy_to(&self, writer: &mut impl io::Write, depth: usize) -> io::Result<()> {
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
    fn copy_to(&self, writer: &mut impl Write, depth: usize) -> io::Result<()> {
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
    fn copy_to(&self, writer: &mut impl io::Write, depth: usize) -> io::Result<()> {
        composite!(writer, depth, [self.substance, self.quantity]);
        Ok(())
    }
}

impl PostgresCopy for IsoDate<'_> {
    fn copy_to(&self, writer: &mut impl io::Write, depth: usize) -> io::Result<()> {
        match self.0 {
            "unbefristet" => write!(writer, "infinity"),
            s => write!(writer, "{s}")
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use crate::postgres_copy::quoted;

    #[test]
    fn quoted_works() {
        let mut buffer = String::new();
        unsafe {
            let mut buffer_vec = buffer.as_mut_vec();
            quoted(|w, d| w.write(b"123").map(|_| ()), &mut buffer_vec, 0).unwrap();
        }
        assert_eq!(buffer, r#""123""#);

        let mut buffer = String::new();
        unsafe {
            let mut buffer_vec = buffer.as_mut_vec();
            quoted(|w, d| w.write(b"123").map(|_| ()), &mut buffer_vec, 2).unwrap();
        }
        assert_eq!(buffer, r#"\\"123\\""#);
    }
}
