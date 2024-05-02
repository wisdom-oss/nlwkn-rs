use std::io;

use chrono::{DateTime, TimeZone};
use nlwkn::helper_types::{Duration, OrFallback, Quantity, Rate, SingleOrPair};
use nlwkn::{DamTargets, LandRecord, LegalDepartmentAbbreviation, PHValues, RateRecord};

use crate::export::{InjectionLimit, IsoDate, UtmPoint};

/// Simple macro to make calling an expression n times simpler, also allows the
/// use of [`?`](https://doc.rust-lang.org/std/result/index.html#the-question-mark-operator-).
macro_rules! repeat {
    ($range:expr, $expr:expr) => {
        for _ in $range {
            $expr;
        }
    };
}

pub trait PostgresCopy {
    /// Write `self` on a writer for the `COPY` instruction from PostgreSQL.
    ///
    /// Implementations of this should avoid allocating heap memory for a fast
    /// copy operation.
    ///
    /// The `depth` is used to escape quotes and the correct level. It acts as a
    /// counter how many escapes need to happen in a quote.
    fn copy_to<W: io::Write>(&self, writer: &mut W, ctx: PostgresCopyContext) -> io::Result<()>;
}

/// Separate trait of [`PostgresCopy`] to avoid upstream implementation
/// conflicts.
pub trait IterPostgresCopy {
    fn copy_to(self, writer: &mut impl io::Write, ctx: PostgresCopyContext) -> io::Result<()>;
}

/// Context for [PostgresCopy] copy operations.
///
/// Keeps track of quotation depth level and if a value is inside a composite
/// and array. This context allows implementors of [PostgresCopy] to behave
/// accordingly.
#[derive(Debug, Default, Clone, Copy)]
pub struct PostgresCopyContext {
    pub depth: usize,
    pub in_composite: bool,
    pub in_array: bool
}

impl PostgresCopyContext {
    /// Raises the depth by `1`.
    pub fn deepen(self) -> Self {
        Self {
            depth: self.depth + 1,
            ..self
        }
    }

    /// Marks context as inside a composite.
    pub fn composite(self) -> Self {
        Self {
            in_composite: true,
            ..self
        }
    }

    /// Marks context as inside an array.
    pub fn array(self) -> Self {
        Self {
            in_array: true,
            ..self
        }
    }
}

/// Quote some values for [PostgresCopy].
///
/// Depending on the quote depth, given by the [`ctx`](PostgresCopyContext),
/// quotation marks will be written to the `writer` before and after the
/// `write_op`. A `ctx.depth == 0` indicates that no quoting is necessary.
/// No matter the depth, the passed `write_op` will have one quote level higher
/// than passed into this function call.
///
/// # Quotation Rules
/// On **depth 0**, no quotation marks are necessary.
/// The `COPY FROM` from statement uses a dedicated separation character, so
/// grouping by quotes is not necessary on depth level 0.
///
/// On **depth 1**, quotation marks will be placed around the given `write_op`,
/// this is necessary for example for composite values inside an array.
///
/// On **depth 2**, quotation marks will need to be escaped as they are already
/// part of some quoted string. For that a double backslash is necessary.
/// For regular control characters a singular backslash is necessary to escape
/// the control character, but for quotation marks it seems that this escape
/// step is done and then, in a later step, the backslashes are parsed.
/// Therefore, the backslashes are part of the passed values and need to be
/// escaped. And escaping a backslash requires another backslash, hence `\\` to
/// escape a quotation mark at this depth.
///
/// On **depth 3** and onward more escaping is necessary.
/// But escaping is not done by adding more backslashes, but by placing more
/// quotation marks. But these doubled quotation marks need to be escaped, so we
/// get a sequence like this `\\"\\"`.
fn quoted<F, W>(write_op: F, writer: &mut W, ctx: PostgresCopyContext) -> io::Result<()>
where
    F: FnOnce(&mut W, PostgresCopyContext) -> io::Result<()>,
    W: io::Write
{
    let quote = |writer: &mut W, ctx: PostgresCopyContext| {
        match ctx.depth {
            0 => (),
            1 => write!(writer, r#"""#)?,
            2 => write!(writer, r#"\\""#)?,
            d => repeat!(1..d, write!(writer, r#"\\""#)?)
        }
        Ok::<_, io::Error>(())
    };

    quote(writer, ctx)?;
    write_op(writer, ctx.deepen())?;
    quote(writer, ctx)?;

    Ok(())
}

pub struct Null;

impl PostgresCopy for Null {
    fn copy_to<W: io::Write>(&self, writer: &mut W, ctx: PostgresCopyContext) -> io::Result<()> {
        // inside a composite nothing needs to be printed
        if !ctx.in_composite {
            write!(writer, r"\N")?;
        }

        Ok(())
    }
}

impl<I, T> IterPostgresCopy for I
where
    I: Iterator<Item = T>,
    T: PostgresCopy
{
    fn copy_to(self, writer: &mut impl io::Write, ctx: PostgresCopyContext) -> io::Result<()> {
        let mut iter = self.peekable();
        if iter.peek().is_none() {
            return Null.copy_to(writer, ctx);
        }

        write!(writer, "{{")?;
        while let Some(it) = iter.next() {
            it.copy_to(writer, ctx.array())?;
            if iter.peek().is_some() {
                writer.write_all(b",")?;
            }
        }
        write!(writer, "}}")?;

        Ok(())
    }
}

macro_rules! impl_postgres_copy {
    ($($type:ty),*) => {$(
        impl PostgresCopy for $type {
            fn copy_to<W: io::Write>(&self, writer: &mut W, _: PostgresCopyContext) -> io::Result<()> {
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
    fn copy_to<W: io::Write>(&self, writer: &mut W, ctx: PostgresCopyContext) -> io::Result<()> {
        let inner = |w: &mut W, ctx: PostgresCopyContext| {
            // this needs custom escaping as postgres demands certain rules
            // https://www.postgresql.org/docs/current/sql-copy.html#id-1.9.3.55.9.2

            // the depth here is always increased by one as quoted will push the depth
            let d = ctx.depth;
            for c in self.chars() {
                match c {
                    '"' if d <= 1 => write!(w, r#"""#),
                    '"' => {
                        // same double backslash as in `quoted`
                        repeat!(2..d, w.write_all(br"\\")?);
                        write!(w, r#"""#)?;
                        repeat!(2..d, w.write_all(br"\\")?);
                        write!(w, r#"""#)
                    }
                    '\\' => write!(w, r"\"),
                    '\n' => write!(w, r"\n"),
                    '\r' => write!(w, r"\r"),
                    _ => write!(w, "{c}")
                }?;
            }
            Ok(())
        };
        quoted(inner, writer, ctx)
    }
}

impl PostgresCopy for String {
    fn copy_to<W: io::Write>(&self, writer: &mut W, ctx: PostgresCopyContext) -> io::Result<()> {
        self.as_str().copy_to(writer, ctx)
    }
}

impl<T> PostgresCopy for &T
where
    T: PostgresCopy
{
    fn copy_to<W: io::Write>(&self, writer: &mut W, ctx: PostgresCopyContext) -> io::Result<()> {
        (*self).copy_to(writer, ctx)
    }
}

impl<T> PostgresCopy for Option<T>
where
    T: PostgresCopy
{
    fn copy_to<W: io::Write>(&self, writer: &mut W, ctx: PostgresCopyContext) -> io::Result<()> {
        match self {
            None => Null.copy_to(writer, ctx),
            Some(v) => v.copy_to(writer, ctx)
        }
    }
}

impl<T> PostgresCopy for (T, T)
where
    T: PostgresCopy
{
    fn copy_to<W: io::Write>(&self, writer: &mut W, ctx: PostgresCopyContext) -> io::Result<()> {
        write!(writer, "{{")?;
        self.0.copy_to(writer, ctx)?;
        write!(writer, ",")?;
        self.1.copy_to(writer, ctx)?;
        write!(writer, "}}")?;
        Ok(())
    }
}

macro_rules! composite {
    // Match the macro invocation pattern with writer, depth, and a list of elements
    ($writer:expr, $ctx:expr, ($first:expr, $($rest:expr),* $(,)?)) => {{
        let ctx = match $ctx.in_array && $ctx.depth == 0 {
            true => $ctx.deepen(),
            false => $ctx
        };
        quoted(|w, ctx| {
            write!(w, "(")?;
            // Process the first element without a leading comma
            $first.copy_to(w, ctx.composite())?;
            // Process the rest of the elements, if any, with leading commas
            $(
                write!(w, ",")?;
                $rest.copy_to(w, ctx.composite())?;
            )*
            write!(w, ")")?;
            Ok(())
        }, $writer, ctx)?;
    }};
}

/// Represents the `water_rights.injection_limit` in the Postgres DB.
impl PostgresCopy for (String, Quantity) {
    fn copy_to<W: io::Write>(&self, writer: &mut W, ctx: PostgresCopyContext) -> io::Result<()> {
        composite!(writer, ctx, (self.0, self.1));
        Ok(())
    }
}

/// Represents the `water_rights.numeric_keyed_value` in the Postgres DB.
impl PostgresCopy for (u64, String) {
    fn copy_to<W: io::Write>(&self, writer: &mut W, ctx: PostgresCopyContext) -> io::Result<()> {
        composite!(writer, ctx, (self.0, &self.1));
        Ok(())
    }
}

impl PostgresCopy for UtmPoint {
    fn copy_to<W: io::Write>(&self, writer: &mut W, _ctx: PostgresCopyContext) -> io::Result<()> {
        let UtmPoint { easting, northing } = self;
        write!(writer, "POINT({easting} {northing})")
    }
}

/// Represents the `water_rights.numeric_keyed_value` in the Postgres DB.
impl PostgresCopy for SingleOrPair<u64, String> {
    fn copy_to<W: io::Write>(&self, writer: &mut W, ctx: PostgresCopyContext) -> io::Result<()> {
        let (key, name) = match self {
            SingleOrPair::Single(key) => (key, None),
            SingleOrPair::Pair(key, name) => (key, Some(name))
        };
        composite!(writer, ctx, (key, name));
        Ok(())
    }
}

impl PostgresCopy for Quantity {
    fn copy_to<W: io::Write>(&self, writer: &mut W, ctx: PostgresCopyContext) -> io::Result<()> {
        composite!(writer, ctx, (self.value, self.unit));
        Ok(())
    }
}

impl PostgresCopy for Rate<f64> {
    fn copy_to<W: io::Write>(&self, writer: &mut W, ctx: PostgresCopyContext) -> io::Result<()> {
        composite!(writer, ctx, (self.value, self.unit, self.per));
        Ok(())
    }
}

impl PostgresCopy for RateRecord {
    fn copy_to<W: io::Write>(&self, writer: &mut W, ctx: PostgresCopyContext) -> io::Result<()> {
        self.iter()
            .filter_map(|or_fallback| match or_fallback {
                OrFallback::Expected(v) => Some(v),
                OrFallback::Fallback(_) => None
            })
            .copy_to(writer, ctx)
    }
}

impl PostgresCopy for Duration {
    fn copy_to<W: io::Write>(&self, writer: &mut W, ctx: PostgresCopyContext) -> io::Result<()> {
        quoted(
            |writer, _| match *self {
                Duration::Seconds(s) => write!(writer, "{s} seconds"),
                Duration::Minutes(m) => write!(writer, "{m} minutes"),
                Duration::Hours(h) => write!(writer, "{h} hours"),
                Duration::Days(d) => write!(writer, "{d} days"),
                Duration::Weeks(w) => write!(writer, "{} days", w * 7.0),
                Duration::Months(m) => write!(writer, "{m} months"),
                Duration::Years(y) => write!(writer, "{y} years")
            },
            writer,
            ctx
        )
    }
}

impl PostgresCopy for DamTargets {
    fn copy_to<W: io::Write>(&self, writer: &mut W, ctx: PostgresCopyContext) -> io::Result<()> {
        if self.default.is_none() && self.steady.is_none() && self.max.is_none() {
            return Null.copy_to(writer, ctx);
        }
        composite!(writer, ctx, (self.default, self.steady, self.max));
        Ok(())
    }
}

impl PostgresCopy for OrFallback<LandRecord> {
    fn copy_to<W: io::Write>(&self, writer: &mut W, ctx: PostgresCopyContext) -> io::Result<()> {
        match self {
            OrFallback::Expected(lr) => composite!(writer, ctx, (lr.district, lr.field, Null)),
            OrFallback::Fallback(s) => composite!(writer, ctx, (Null, Null, s))
        }
        Ok(())
    }
}

impl PostgresCopy for LegalDepartmentAbbreviation {
    fn copy_to<W: io::Write>(&self, writer: &mut W, _: PostgresCopyContext) -> io::Result<()> {
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
    fn copy_to<W: io::Write>(&self, writer: &mut W, _: PostgresCopyContext) -> io::Result<()> {
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
    fn copy_to<W: io::Write>(&self, writer: &mut W, ctx: PostgresCopyContext) -> io::Result<()> {
        composite!(writer, ctx, (self.substance, self.quantity));
        Ok(())
    }
}

impl PostgresCopy for IsoDate<'_> {
    fn copy_to<W: io::Write>(&self, writer: &mut W, _ctx: PostgresCopyContext) -> io::Result<()> {
        match self.0 {
            "unbefristet" => write!(writer, "infinity"),
            s => write!(writer, "{s}")
        }
    }
}

impl<Tz> PostgresCopy for DateTime<Tz>
where
    Tz: TimeZone
{
    fn copy_to<W: io::Write>(&self, writer: &mut W, ctx: PostgresCopyContext) -> io::Result<()> {
        write!(writer, "{}", self.to_rfc3339())
    }
}

#[cfg(test)]
mod tests {

    use std::io::Write;

    use crate::postgres_copy::{quoted, PostgresCopy, PostgresCopyContext};

    fn ctx_depth(depth: usize) -> PostgresCopyContext {
        PostgresCopyContext {
            depth,
            ..Default::default()
        }
    }

    #[test]
    fn quoted_works() {
        let mut buffer = String::new();
        unsafe {
            let mut buffer_vec = buffer.as_mut_vec();
            quoted(
                |w, _| w.write_all(b"123").map(|_| ()),
                &mut buffer_vec,
                ctx_depth(0)
            )
            .unwrap();
        }
        assert_eq!(buffer, r#"123"#, "depth 0");

        let mut buffer = String::new();
        unsafe {
            let mut buffer_vec = buffer.as_mut_vec();
            quoted(
                |w, _| w.write_all(b"123").map(|_| ()),
                &mut buffer_vec,
                ctx_depth(1)
            )
            .unwrap();
        }
        assert_eq!(buffer, r#""123""#, "depth 1");

        let mut buffer = String::new();
        unsafe {
            let mut buffer_vec = buffer.as_mut_vec();
            quoted(
                |w, _| w.write_all(b"123").map(|_| ()),
                &mut buffer_vec,
                ctx_depth(2)
            )
            .unwrap();
        }
        assert_eq!(buffer, r#"\\"123\\""#, "depth 2");

        let mut buffer = String::new();
        unsafe {
            let mut buffer_vec = buffer.as_mut_vec();
            quoted(
                |w, _| w.write_all(b"123").map(|_| ()),
                &mut buffer_vec,
                ctx_depth(3)
            )
            .unwrap();
        }
        assert_eq!(buffer, r#"\\"\\"123\\"\\""#, "depth 3");
    }

    #[test]
    fn composite_works() -> anyhow::Result<()> {
        let mut buffer = String::new();
        unsafe {
            let buffer_vec = buffer.as_mut_vec();
            composite!(buffer_vec, ctx_depth(0), ("lol", 69));
        }
        assert_eq!(buffer, r#"("lol",69)"#, "depth 0");

        let mut buffer = String::new();
        unsafe {
            let buffer_vec = buffer.as_mut_vec();
            composite!(buffer_vec, ctx_depth(1), ("lol", 69));
        }
        assert_eq!(buffer, r#""(\\"lol\\",69)""#, "depth 1");

        Ok(())
    }

    #[test]
    fn str_copy_to_works() {
        let mut buffer = String::new();
        unsafe {
            let buffer_vec = buffer.as_mut_vec();
            let input = r#"some "quoted" text"#;
            input.copy_to(buffer_vec, ctx_depth(0)).unwrap();
        }
        assert_eq!(buffer, r#"some "quoted" text"#, "depth 0");

        let mut buffer = String::new();
        unsafe {
            let buffer_vec = buffer.as_mut_vec();
            let input = r#"some "quoted" text"#;
            input.copy_to(buffer_vec, ctx_depth(1)).unwrap();
        }
        assert_eq!(buffer, r#""some ""quoted"" text""#, "depth 1");

        let mut buffer = String::new();
        unsafe {
            let buffer_vec = buffer.as_mut_vec();
            let input = r#"some "quoted" text"#;
            input.copy_to(buffer_vec, ctx_depth(2)).unwrap();
        }
        assert_eq!(buffer, r#"\\"some \\"\\"quoted\\"\\" text\\""#, "depth 2");
    }
}
