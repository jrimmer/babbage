use crate::error::Error;
use crate::format::{FormatOptions, Formatted, format_decimal};
use crate::functions::{Rounding, round_to_multiple};
use crate::number::Number;
use crate::parser::{BinOp, Phrase};
use crate::units::UnitExpr;
use num_traits::Signed;
use num_traits::ToPrimitive;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Plain,
    /// The stored number is the fraction: 10% is held as 1/10.
    Percent,
    /// `true` or `false`, held as 1 or 0.
    Bool,
    /// A ratio shown with an x: `2x`.
    Multiplier,
    /// A named sales tax such as VAT: it adds, removes and extracts by tax rules, not percentage rules.
    TaxRate,
    /// A calendar day, with a time of day when the number has a fraction. Held as days since 1970-01-01, local.
    Date,
    /// A time of day, held as seconds since midnight.
    Clock,
    /// An answer that is a word, such as `Tuesday`.
    Text,
}

/// A number with an optional unit. A percentage never has a unit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Value {
    number: Number,
    kind: Kind,
    unit: UnitExpr,
    /// True when the number is a close rational stand-in for an irrational result (a square root, a sine,
    /// pi), or came from one. It is never silently treated as exact.
    approx: bool,
    /// The base to show a whole number in: 10, or 2, 8, 16 and others after `as hex` or a literal like `0xFF`.
    radix: u8,
    /// The words of a `Text` value.
    text: Option<std::rc::Rc<str>>,
    /// Show a length of time as `3 weeks 5 days` instead of a single unit.
    timespan: bool,
}

const TEMPERATURE: &str = "temperatures can be converted, and a plain number added or subtracted, but not combined further";
const NOT_YET: &str = "this combination of percentages is not defined";
const PHRASE_NEEDS_PLAIN: &str = "this phrase needs plain numbers and, where written, a percentage";

fn one() -> Number {
    Number::from_i64(1)
}

impl Value {
    pub fn plain(number: Number) -> Value {
        Value {
            number,
            kind: Kind::Plain,
            unit: UnitExpr::default(),
            approx: false,
            radix: 10,
            text: None,
            timespan: false,
        }
    }

    pub fn percent(fraction: Number) -> Value {
        Value {
            number: fraction,
            kind: Kind::Percent,
            unit: UnitExpr::default(),
            approx: false,
            radix: 10,
            text: None,
            timespan: false,
        }
    }

    /// A number in a unit. Units that cancel to nothing measurable (km/m) become a plain number.
    pub fn quantity(number: Number, unit: UnitExpr) -> Result<Value, Error> {
        let (scale, unit) = unit.reduce()?;
        let number = number.mul(&scale)?;
        if !unit.is_empty() && unit.dim().is_none() {
            return Ok(Value::plain(number.mul(&unit.factor()?)?));
        }
        Ok(Value {
            number,
            kind: Kind::Plain,
            unit,
            approx: false,
            radix: 10,
            text: None,
            timespan: false,
        })
    }

    /// This value's number restated in the unit of `like`, which must measure the same thing.
    pub fn convert_to_unit_of(self, like: &Value) -> Result<Number, Error> {
        if self.unit == like.unit {
            return Ok(self.number);
        }
        self.convert(&like.unit).map(|v| v.number)
    }

    pub fn date(days: Number) -> Value {
        Value {
            number: days,
            kind: Kind::Date,
            unit: UnitExpr::default(),
            approx: false,
            radix: 10,
            text: None,
            timespan: false,
        }
    }

    pub fn clock(seconds: Number) -> Value {
        Value {
            number: seconds,
            kind: Kind::Clock,
            unit: UnitExpr::default(),
            approx: false,
            radix: 10,
            text: None,
            timespan: false,
        }
    }

    pub fn text(words: &str) -> Value {
        Value {
            number: Number::from_i64(0),
            kind: Kind::Text,
            unit: UnitExpr::default(),
            approx: false,
            radix: 10,
            text: Some(words.into()),
            timespan: false,
        }
    }

    /// This length of time, to be shown as a timespan.
    pub fn as_timespan(mut self) -> Value {
        self.timespan = true;
        self
    }

    pub fn is_timespan(&self) -> bool {
        self.timespan
    }

    pub fn words(&self) -> Option<&str> {
        self.text.as_deref()
    }

    pub fn tax_rate(rate: Number) -> Value {
        Value {
            number: rate,
            kind: Kind::TaxRate,
            unit: UnitExpr::default(),
            approx: false,
            radix: 10,
            text: None,
            timespan: false,
        }
    }

    pub fn multiplier(ratio: Number) -> Value {
        Value {
            number: ratio,
            kind: Kind::Multiplier,
            unit: UnitExpr::default(),
            approx: false,
            radix: 10,
            text: None,
            timespan: false,
        }
    }

    pub fn boolean(b: bool) -> Value {
        Value {
            number: Number::from_i64(i64::from(b)),
            kind: Kind::Bool,
            unit: UnitExpr::default(),
            approx: false,
            radix: 10,
            text: None,
            timespan: false,
        }
    }

    /// This whole number, to be shown in `radix` (2 to 36).
    pub fn in_radix(mut self, radix: u8) -> Result<Value, Error> {
        if self.kind != Kind::Plain
            || !self.unit.is_empty()
            || !self.number.is_integer()
            || !(2..=36).contains(&radix)
        {
            return Err(Error::UnsupportedOperation(
                "only a whole number without a unit can be shown in another base",
            ));
        }
        self.radix = radix;
        Ok(self)
    }

    pub fn radix(&self) -> u8 {
        self.radix
    }

    /// The truth value, for a `true` or `false`.
    pub fn as_bool(&self) -> Option<bool> {
        (self.kind == Kind::Bool).then(|| !self.number.is_zero())
    }

    /// This value marked as approximate.
    pub fn approximate(mut self) -> Value {
        self.approx = true;
        self
    }

    pub fn is_approximate(&self) -> bool {
        self.approx
    }

    fn tainted(mut self, by: bool) -> Value {
        self.approx |= by;
        self
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// The stored number. For a percentage this is the fraction, not the figure shown.
    pub fn number(&self) -> &Number {
        &self.number
    }

    pub fn unit(&self) -> &UnitExpr {
        &self.unit
    }

    pub fn into_plain(self) -> Result<Number, Error> {
        if self.kind == Kind::Plain && self.unit.is_empty() {
            Ok(self.number)
        } else {
            Err(Error::NotAPlainNumber)
        }
    }

    /// Applies `f` to the number and keeps the unit and kind.
    pub fn map_number(
        &self,
        f: impl FnOnce(&Number) -> Result<Number, Error>,
    ) -> Result<Value, Error> {
        Ok(Value {
            number: f(&self.number)?,
            kind: self.kind,
            unit: self.unit.clone(),
            approx: self.approx,
            radix: 10,
            text: None,
            timespan: false,
        })
    }

    /// Orders two values of the same kind: plain numbers, quantities of one dimension, or percentages.
    pub fn compare(a: &Value, b: &Value) -> Result<std::cmp::Ordering, Error> {
        if a.kind != b.kind {
            return Err(Error::UnsupportedOperation(
                "a percentage and a plain number cannot be compared",
            ));
        }
        if a.unit.dim() != b.unit.dim() {
            return Err(mismatch(&a.unit, &b.unit));
        }
        Ok(a.si()?.cmp(&b.si()?))
    }

    pub fn neg(&self) -> Value {
        Value {
            number: self.number.neg(),
            kind: self.kind,
            unit: self.unit.clone(),
            approx: self.approx,
            radix: self.radix,
            text: self.text.clone(),
            timespan: self.timespan,
        }
    }

    pub fn apply_unit(self, unit: &UnitExpr) -> Result<Value, Error> {
        let approx = self.approx;
        self.apply_unit_exact(unit).map(|v| v.tainted(approx))
    }

    fn apply_unit_exact(self, unit: &UnitExpr) -> Result<Value, Error> {
        if self.kind != Kind::Plain || !self.unit.is_empty() {
            return Err(Error::UnsupportedOperation(
                "a unit can only follow a plain number",
            ));
        }
        Value::quantity(self.number, unit.clone())
    }

    /// Converts with a rate the user wrote: `50 EUR in USD at 1.05 USD/EUR`.
    pub fn convert_at(self, rate: &Value, target: &UnitExpr) -> Result<Value, Error> {
        Value::binary(BinOp::Mul, &self, rate)?.convert(target)
    }

    pub fn cast(self, target: &UnitExpr) -> Result<Value, Error> {
        let approx = self.approx;
        self.cast_exact(target).map(|v| v.tainted(approx))
    }

    fn cast_exact(self, target: &UnitExpr) -> Result<Value, Error> {
        if self.kind != Kind::Plain {
            return Err(Error::IncompatibleUnits);
        }
        if self.unit.is_empty() {
            return Err(Error::IncompatibleUnits);
        }
        Value::quantity(self.number, target.clone())
    }

    pub fn convert(self, target: &UnitExpr) -> Result<Value, Error> {
        let approx = self.approx
            || ((self.unit.is_approximate() || target.is_approximate()) && self.unit != *target);
        self.convert_exact(target).map(|v| v.tainted(approx))
    }

    fn convert_exact(self, target: &UnitExpr) -> Result<Value, Error> {
        if self.kind != Kind::Plain || self.unit.is_empty() {
            return Err(Error::IncompatibleUnits);
        }
        if self.unit.dim() != target.dim() {
            return Err(mismatch(&self.unit, target));
        }
        if self.unit.has_offset() || target.has_offset() {
            return convert_scale(self, target);
        }
        let number = self
            .number
            .mul(&self.unit.factor()?)?
            .div(&target.factor()?)?;
        Value::quantity(number, target.clone())
    }

    pub fn into_percent_literal(self) -> Result<Value, Error> {
        let approx = self.approx;
        self.into_percent_literal_exact().map(|v| v.tainted(approx))
    }

    fn into_percent_literal_exact(self) -> Result<Value, Error> {
        if self.kind != Kind::Plain || !self.unit.is_empty() {
            return Err(Error::UnsupportedOperation("a percentage of a percentage"));
        }
        Ok(Value::percent(self.number.div(&Number::from_i64(100))?))
    }

    /// The value in SI base units, for comparing quantities in different units.
    fn si(&self) -> Result<Number, Error> {
        if self.unit.has_offset() {
            return Err(Error::UnsupportedOperation(TEMPERATURE));
        }
        if self.unit.is_empty() {
            Ok(self.number.clone())
        } else {
            self.number.mul(&self.unit.factor()?)
        }
    }

    pub fn binary(op: BinOp, l: &Value, r: &Value) -> Result<Value, Error> {
        let approx = l.approx || r.approx || reconciles_approximate_units(&[&l.unit, &r.unit]);
        Value::binary_exact(op, l, r).map(|v| v.tainted(approx))
    }

    fn binary_exact(op: BinOp, l: &Value, r: &Value) -> Result<Value, Error> {
        use Kind::{Percent, Plain};
        match op {
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                return comparison(op, l, r);
            }
            BinOp::And | BinOp::Or | BinOp::Xor | BinOp::LogicAnd | BinOp::LogicOr => {
                return logic(op, l, r);
            }
            BinOp::BitAnd | BinOp::BitOr | BinOp::Shl | BinOp::Shr => return bitwise(op, l, r),
            _ => {}
        }
        if l.kind == Kind::Bool || r.kind == Kind::Bool {
            return Err(Error::UnsupportedOperation(
                "true and false take part only in comparisons and logic",
            ));
        }
        let temporal = |k: Kind| matches!(k, Kind::Date | Kind::Clock | Kind::Text);
        if temporal(l.kind) || temporal(r.kind) {
            return temporal_arithmetic(op, l, r);
        }
        if l.kind == Kind::TaxRate || r.kind == Kind::TaxRate {
            return tax_arithmetic(op, l, r);
        }
        if op == BinOp::Of {
            return Value::binary_exact(BinOp::Mul, l, r);
        }
        if l.kind == Kind::Multiplier || r.kind == Kind::Multiplier {
            return match (op, l.kind, r.kind) {
                (BinOp::Mul, Kind::Multiplier, Kind::Multiplier) => {
                    Ok(Value::multiplier(l.number.mul(&r.number)?))
                }
                (BinOp::Mul, Kind::Multiplier, Kind::Plain) => {
                    Value::quantity(l.number.mul(&r.number)?, r.unit.clone())
                }
                (BinOp::Mul, Kind::Plain, Kind::Multiplier) => {
                    Value::quantity(l.number.mul(&r.number)?, l.unit.clone())
                }
                _ => Err(Error::UnsupportedOperation(
                    "a multiplier can only be multiplied",
                )),
            };
        }
        if l.unit.has_offset() || r.unit.has_offset() {
            let shifts = matches!(op, BinOp::Add | BinOp::Sub | BinOp::Join)
                && l.kind == Plain
                && r.kind == Plain;
            if !shifts || (!l.unit.is_empty() && !r.unit.is_empty()) {
                return Err(Error::UnsupportedOperation(TEMPERATURE));
            }
        }
        match (op, l.kind, r.kind) {
            (BinOp::Add | BinOp::Sub | BinOp::Join, Plain, Plain) => sum(op, l, r),
            (BinOp::Add | BinOp::Sub, Percent, Percent) => {
                Ok(Value::percent(arith(op, &l.number, &r.number)?))
            }
            (BinOp::Add | BinOp::Sub, Percent, Plain) if r.unit.is_empty() => {
                Ok(Value::percent(arith(op, &l.number, &r.number)?))
            }
            (BinOp::Add | BinOp::Sub, Plain, Percent) => {
                Value::quantity(l.number.mul(&adjust(op, &r.number)?)?, l.unit.clone())
            }
            (BinOp::Mul, Plain, Plain) => product(l, r),
            (BinOp::Mul, Percent, Plain) => {
                Value::quantity(l.number.mul(&r.number)?, r.unit.clone())
            }
            (BinOp::Mul, Plain, Percent) => {
                Value::quantity(l.number.mul(&r.number)?, l.unit.clone())
            }
            (BinOp::Div, Plain, Plain) => quotient(l, r),
            (BinOp::Mod, Plain, Plain) => modulo(l, r),
            (BinOp::Pow, Plain, Plain) => power(l, r),
            (BinOp::On | BinOp::Off, rate, Plain) if rate == Percent || l.unit.is_empty() => {
                let sign = if op == BinOp::On {
                    BinOp::Add
                } else {
                    BinOp::Sub
                };
                Value::quantity(r.number.mul(&adjust(sign, &l.number)?)?, r.unit.clone())
            }
            (BinOp::On | BinOp::Off, ..) => Err(Error::UnsupportedOperation(
                "'on' and 'off' need a plain number after them",
            )),
            _ => Err(Error::UnsupportedOperation(NOT_YET)),
        }
    }

    pub fn phrase(phrase: Phrase, args: &[Value]) -> Result<Value, Error> {
        let units: Vec<&UnitExpr> = args.iter().map(|a| &a.unit).collect();
        let approx = args.iter().any(|a| a.approx) || reconciles_approximate_units(&units);
        Value::phrase_exact(phrase, args).map(|v| v.tainted(approx))
    }

    fn phrase_exact(phrase: Phrase, args: &[Value]) -> Result<Value, Error> {
        use Kind::{Percent, Plain};
        let refuse = || Err(Error::UnsupportedOperation(PHRASE_NEEDS_PLAIN));
        if let (
            Phrase::WhatPercentOf
            | Phrase::WhatPercentOn
            | Phrase::WhatPercentOff
            | Phrase::PercentChange,
            [a, b],
        ) = (phrase, args)
            && a.kind == Plain
            && b.kind == Plain
            && a.unit.dim() != b.unit.dim()
            && a.unit.dim().has_money()
            && b.unit.dim().has_money()
        {
            return Err(Error::CurrencyRateNeeded);
        }
        match (phrase, args) {
            (Phrase::RoundPlaces, [v, places])
                if places.kind == Plain && places.unit.is_empty() =>
            {
                let places = places
                    .number
                    .numer()
                    .to_u32()
                    .filter(|p| *p <= 1000 && places.number.is_integer())
                    .ok_or(Error::OutOfDomain)?;
                let step = Number::from_i64(1)
                    .div(&Number::from_i64(10).pow(&Number::from_i64(i64::from(places)))?)?;
                v.map_number(|n| round_to_multiple(n, &step, Rounding::Nearest))
            }
            (Phrase::RoundNearest | Phrase::RoundUp | Phrase::RoundDown, [v, step])
                if step.kind == Plain && step.unit.is_empty() =>
            {
                let mode = match phrase {
                    Phrase::RoundNearest => Rounding::Nearest,
                    Phrase::RoundUp => Rounding::Up,
                    _ => Rounding::Down,
                };
                v.map_number(|n| round_to_multiple(n, &step.number, mode))
            }
            (Phrase::AsMultiplier, [v]) if v.kind == Plain && v.unit.is_empty() => {
                Ok(Value::multiplier(v.number.clone()))
            }
            (
                Phrase::MultiplierOf
                | Phrase::MultiplierOn
                | Phrase::MultiplierOff
                | Phrase::MultiplierChange,
                [a, b],
            ) if a.kind == Plain && b.kind == Plain && a.unit.dim() == b.unit.dim() => {
                let (x, y) = (a.si()?, b.si()?);
                Ok(Value::multiplier(match phrase {
                    Phrase::MultiplierOf => x.div(&y)?,
                    Phrase::MultiplierOn => x.sub(&y)?.div(&y)?,
                    Phrase::MultiplierOff => y.sub(&x)?.div(&y)?,
                    _ => y.div(&x)?,
                }))
            }
            (Phrase::AsPercent, [v]) => match v.kind {
                Plain if v.unit.is_empty() => Ok(Value::percent(v.number.clone())),
                Percent => Ok(v.clone()),
                Plain
                | Kind::Bool
                | Kind::Multiplier
                | Kind::TaxRate
                | Kind::Date
                | Kind::Clock
                | Kind::Text => refuse(),
            },
            (
                Phrase::IsPercentOfWhat | Phrase::IsPercentOnWhat | Phrase::IsPercentOffWhat,
                [a, p],
            ) if a.kind == Plain && p.unit.is_empty() => {
                let divisor = match phrase {
                    Phrase::IsPercentOfWhat => p.number.clone(),
                    Phrase::IsPercentOnWhat => adjust(BinOp::Add, &p.number)?,
                    _ => adjust(BinOp::Sub, &p.number)?,
                };
                Value::quantity(a.number.div(&divisor)?, a.unit.clone())
            }
            (
                Phrase::WhatPercentOf
                | Phrase::WhatPercentOn
                | Phrase::WhatPercentOff
                | Phrase::PercentChange,
                [a, b],
            ) if a.kind == Plain && b.kind == Plain && a.unit.dim() == b.unit.dim() => {
                let (x, y) = (a.si()?, b.si()?);
                let fraction = match phrase {
                    Phrase::WhatPercentOf => x.div(&y)?,
                    Phrase::WhatPercentOn => x.sub(&y)?.div(&y)?,
                    Phrase::WhatPercentOff => y.sub(&x)?.div(&y)?,
                    _ => y.sub(&x)?.div(&x)?,
                };
                Ok(Value::percent(fraction))
            }
            _ => refuse(),
        }
    }
}

fn convert_scale(v: Value, target: &UnitExpr) -> Result<Value, Error> {
    if !(v.unit.is_single() && target.is_single()) {
        return Err(Error::UnsupportedOperation(TEMPERATURE));
    }
    let kelvin = v.unit.to_kelvin(&v.number)?;
    Value::quantity(target.from_kelvin(&kelvin)?, target.clone())
}

/// Different currencies need a rate; anything else that does not match is simply incompatible.
fn mismatch(a: &UnitExpr, b: &UnitExpr) -> Error {
    if a.dim().has_money() && b.dim().has_money() {
        Error::CurrencyRateNeeded
    } else {
        Error::IncompatibleUnits
    }
}

/// True when two different units are brought together and one of them has an approximate factor.
fn reconciles_approximate_units(units: &[&UnitExpr]) -> bool {
    units.iter().enumerate().any(|(i, a)| {
        units[i + 1..].iter().any(|b| {
            !a.is_empty() && !b.is_empty() && a != b && (a.is_approximate() || b.is_approximate())
        })
    })
}

/// `$300 + VAT`, `$300 - VAT`, `VAT on $300`, `VAT off $300`, `VAT of $300` and `VAT in $300`.
/// Removing tax divides by 1 plus the rate: it is not a percentage taken off.
fn tax_arithmetic(op: BinOp, l: &Value, r: &Value) -> Result<Value, Error> {
    let with_tax = |rate: &Number| one().add(rate);
    match (op, l.kind, r.kind) {
        (BinOp::Add, Kind::Plain, Kind::TaxRate) => {
            Value::quantity(l.number.mul(&with_tax(&r.number)?)?, l.unit.clone())
        }
        (BinOp::Sub, Kind::Plain, Kind::TaxRate) => {
            Value::quantity(l.number.div(&with_tax(&r.number)?)?, l.unit.clone())
        }
        (BinOp::On, Kind::TaxRate, Kind::Plain) => {
            Value::quantity(r.number.mul(&l.number)?, r.unit.clone())
        }
        (BinOp::Off, Kind::TaxRate, Kind::Plain) => {
            Value::quantity(r.number.div(&with_tax(&l.number)?)?, r.unit.clone())
        }
        (BinOp::Of, Kind::TaxRate, Kind::Plain) => {
            let tax = r.number.mul(&l.number)?.div(&with_tax(&l.number)?)?;
            Value::quantity(tax, r.unit.clone())
        }
        _ => Err(Error::UnsupportedOperation(
            "a tax is added, removed or extracted from an amount",
        )),
    }
}

fn is_length_of_time(v: &Value) -> bool {
    let seconds = crate::units::lookup("s").expect("seconds are defined");
    v.kind == Kind::Plain && !v.unit.is_empty() && v.unit.dim() == seconds.dim()
}

fn seconds_of(v: &Value) -> Result<Number, Error> {
    v.number.mul(&v.unit.factor()?)
}

/// A date shifted by a length of time. Months and years move the calendar, so 31 January plus a month is the
/// last day of February; workdays skip weekends; everything else is an exact number of seconds.
fn shift_date(date: &Value, by: &Value, sign: i64) -> Result<Value, Error> {
    use crate::calendar;
    let sign_n = Number::from_i64(sign);
    let lookup = |name: &str| crate::units::lookup(name).expect("unit is defined");
    let whole_day = date.number.floor();
    let day = i64::try_from(&whole_day).map_err(|_| Error::Overflow)?;
    let fraction = date
        .number
        .sub(&Number::from_ratio(whole_day, num_bigint::BigInt::from(1))?)?;
    let months = if by.unit == lookup("mo") {
        Some(by.number.clone())
    } else if by.unit == lookup("yr") {
        Some(by.number.mul(&Number::from_i64(12))?)
    } else {
        None
    };
    if let Some(m) = months {
        if !m.is_integer() {
            return Err(Error::UnsupportedOperation(
                "a date moves by whole months or years",
            ));
        }
        let n = i64::try_from(m.numer()).map_err(|_| Error::Overflow)? * sign;
        let moved =
            calendar::days_from_civil(calendar::add_months(calendar::civil_from_days(day), n));
        return Ok(Value::date(Number::from_i64(moved).add(&fraction)?));
    }
    if by.unit == lookup("workday") {
        if !by.number.is_integer() {
            return Err(Error::UnsupportedOperation(
                "a date moves by whole workdays",
            ));
        }
        let n = i64::try_from(by.number.numer()).map_err(|_| Error::Overflow)? * sign;
        return Ok(Value::date(
            Number::from_i64(calendar::add_workdays(day, n)).add(&fraction)?,
        ));
    }
    let days = seconds_of(by)?
        .div(&Number::from_i64(86_400))?
        .mul(&sign_n)?;
    Ok(Value::date(date.number.add(&days)?))
}

/// A time of day shifted by a length of time, wrapping past midnight.
fn shift_clock(clock: &Value, by: &Value, sign: i64) -> Result<Value, Error> {
    let moved = clock
        .number
        .add(&seconds_of(by)?.mul(&Number::from_i64(sign))?)?;
    let day = Number::from_i64(86_400);
    let wrapped = moved.sub(&day.mul(&Number::from_ratio(
        moved.div(&day)?.floor(),
        num_bigint::BigInt::from(1),
    )?)?)?;
    Ok(Value::clock(wrapped))
}

fn temporal_arithmetic(op: BinOp, l: &Value, r: &Value) -> Result<Value, Error> {
    let seconds = || crate::units::lookup("s").expect("seconds are defined");
    match (op, l.kind, r.kind) {
        (BinOp::Add | BinOp::Join, Kind::Date, Kind::Plain) if is_length_of_time(r) => {
            shift_date(l, r, 1)
        }
        (BinOp::Sub, Kind::Date, Kind::Plain) if is_length_of_time(r) => shift_date(l, r, -1),
        (BinOp::Add | BinOp::Join, Kind::Plain, Kind::Date) if is_length_of_time(l) => {
            shift_date(r, l, 1)
        }
        (BinOp::Sub, Kind::Date, Kind::Date) => {
            let seconds_apart = l.number.sub(&r.number)?.mul(&Number::from_i64(86_400))?;
            Ok(Value::quantity(seconds_apart, seconds())?.as_timespan())
        }
        (BinOp::Add | BinOp::Join, Kind::Clock, Kind::Plain) if is_length_of_time(r) => {
            shift_clock(l, r, 1)
        }
        (BinOp::Sub, Kind::Clock, Kind::Plain) if is_length_of_time(r) => shift_clock(l, r, -1),
        (BinOp::Add | BinOp::Join, Kind::Plain, Kind::Clock) if is_length_of_time(l) => {
            shift_clock(r, l, 1)
        }
        (BinOp::Sub, Kind::Clock, Kind::Clock) => {
            Ok(Value::quantity(l.number.sub(&r.number)?, seconds())?.as_timespan())
        }
        _ => Err(Error::UnsupportedOperation(
            "a date or time can be shifted by a length of time, and two of them can be subtracted",
        )),
    }
}

fn comparison(op: BinOp, l: &Value, r: &Value) -> Result<Value, Error> {
    use std::cmp::Ordering::{Equal, Greater, Less};
    if l.kind == Kind::Bool || r.kind == Kind::Bool {
        return match (l.as_bool(), r.as_bool(), op) {
            (Some(a), Some(b), BinOp::Eq) => Ok(Value::boolean(a == b)),
            (Some(a), Some(b), BinOp::Ne) => Ok(Value::boolean(a != b)),
            (Some(_), Some(_), _) => Err(Error::UnsupportedOperation(
                "true and false can only be compared for equality",
            )),
            _ => Err(Error::UnsupportedOperation(
                "a true or false cannot be compared with a number",
            )),
        };
    }
    if l.kind == Kind::Text || r.kind == Kind::Text {
        return match (l.words(), r.words(), op) {
            (Some(a), Some(b), BinOp::Eq) => Ok(Value::boolean(a.eq_ignore_ascii_case(b))),
            (Some(a), Some(b), BinOp::Ne) => Ok(Value::boolean(!a.eq_ignore_ascii_case(b))),
            _ => Err(Error::UnsupportedOperation(
                "words can only be compared for equality",
            )),
        };
    }
    let order = Value::compare(l, r)?;
    if l.approx || r.approx {
        let gap = l.si()?.sub(&r.si()?)?.abs();
        let scale = if r.si()?.abs() > Number::from_i64(1) {
            r.si()?.abs()
        } else {
            Number::from_i64(1)
        };
        let tolerance = Number::from_decimal_str("1e-60")?.mul(&scale)?;
        if gap <= tolerance {
            return Err(Error::UnsupportedOperation(
                "these approximate values are too close to tell apart",
            ));
        }
    }
    let result = match op {
        BinOp::Eq => order == Equal,
        BinOp::Ne => order != Equal,
        BinOp::Lt => order == Less,
        BinOp::Gt => order == Greater,
        BinOp::Le => order != Greater,
        _ => order != Less,
    };
    Ok(Value::boolean(result))
}

fn logic(op: BinOp, l: &Value, r: &Value) -> Result<Value, Error> {
    if let (Some(a), Some(b)) = (l.as_bool(), r.as_bool()) {
        return Ok(Value::boolean(match op {
            BinOp::And | BinOp::LogicAnd => a && b,
            BinOp::Or | BinOp::LogicOr => a || b,
            _ => a != b,
        }));
    }
    match op {
        BinOp::LogicAnd | BinOp::LogicOr => Err(Error::UnsupportedOperation(
            "&& and || work on true and false",
        )),
        BinOp::And => bitwise(BinOp::BitAnd, l, r),
        BinOp::Or => bitwise(BinOp::BitOr, l, r),
        _ => bitwise(BinOp::Xor, l, r),
    }
}

const BITWISE_NEEDS_WHOLE: &str = "bitwise operators work on whole numbers of zero or more";

fn whole_number(v: &Value) -> Result<num_bigint::BigInt, Error> {
    if v.kind != Kind::Plain
        || !v.unit.is_empty()
        || !v.number.is_integer()
        || v.number.is_negative()
    {
        return Err(Error::UnsupportedOperation(BITWISE_NEEDS_WHOLE));
    }
    Ok(v.number.numer().clone())
}

fn bitwise(op: BinOp, l: &Value, r: &Value) -> Result<Value, Error> {
    let (a, b) = (whole_number(l)?, whole_number(r)?);
    let result = match op {
        BinOp::BitAnd => a & b,
        BinOp::BitOr => a | b,
        BinOp::Xor => a ^ b,
        BinOp::Shl | BinOp::Shr => {
            let by = b
                .to_usize()
                .filter(|n| *n as u64 <= crate::number::MAX_BITS)
                .ok_or(Error::Overflow)?;
            if op == BinOp::Shl { a << by } else { a >> by }
        }
        _ => unreachable!("only bitwise operators reach here"),
    };
    let value = Value::plain(Number::from_ratio(result, num_bigint::BigInt::from(1))?);
    // Two numbers written in the same base give an answer in that base: `0xFF & 0x0F` is `0xF`.
    if l.radix == r.radix && !matches!(op, BinOp::Shl | BinOp::Shr) {
        value.in_radix(l.radix)
    } else {
        Ok(value)
    }
}

fn arith(op: BinOp, a: &Number, b: &Number) -> Result<Number, Error> {
    if matches!(op, BinOp::Add | BinOp::Join) {
        a.add(b)
    } else {
        a.sub(b)
    }
}

/// 1 + p for Add, 1 - p for Sub.
fn adjust(op: BinOp, p: &Number) -> Result<Number, Error> {
    arith(op, &one(), p)
}

/// Plain numbers take the unit of the other side; two units of one kind meet in the larger one.
fn sum(op: BinOp, l: &Value, r: &Value) -> Result<Value, Error> {
    match (l.unit.is_empty(), r.unit.is_empty()) {
        (true, true) => Ok(Value::plain(arith(op, &l.number, &r.number)?)),
        (true, false) => Value::quantity(arith(op, &l.number, &r.number)?, r.unit.clone()),
        (false, true) => Value::quantity(arith(op, &l.number, &r.number)?, l.unit.clone()),
        (false, false) => {
            if l.unit.dim() != r.unit.dim() {
                return Err(mismatch(&l.unit, &r.unit));
            }
            let (fl, fr) = (l.unit.factor()?, r.unit.factor()?);
            let target = if fr > fl { &r.unit } else { &l.unit };
            let ft = if fr > fl { &fr } else { &fl };
            let (a, b) = (l.number.mul(&fl)?.div(ft)?, r.number.mul(&fr)?.div(ft)?);
            Value::quantity(arith(op, &a, &b)?, target.clone())
        }
    }
}

/// Two quantities of one kind in different units multiply in the left one's unit.
fn product(l: &Value, r: &Value) -> Result<Value, Error> {
    // `$30 x 4 days`: a price times a count of something is that price per thing.
    for (price, other) in [(l, r), (r, l)] {
        if price.unit.single_currency().is_some()
            && !other.unit.is_empty()
            && !other.unit.dim().has_money()
        {
            return Value::quantity(l.number.mul(&r.number)?, price.unit.clone());
        }
    }
    let same_kind_different_unit = l.unit != r.unit && l.unit.dim() == r.unit.dim();
    let right = if same_kind_different_unit {
        r.number.mul(&r.unit.factor()?)?.div(&l.unit.factor()?)?
    } else {
        r.number.clone()
    };
    let unit = if same_kind_different_unit {
        l.unit.times(&l.unit)
    } else {
        l.unit.times(&r.unit)
    };
    Value::quantity(l.number.mul(&right)?, unit)
}

fn modulo(l: &Value, r: &Value) -> Result<Value, Error> {
    let right = if l.unit == r.unit {
        r.number.clone()
    } else if !l.unit.is_empty() && l.unit.dim() == r.unit.dim() {
        r.number.mul(&r.unit.factor()?)?.div(&l.unit.factor()?)?
    } else {
        return Err(Error::IncompatibleUnits);
    };
    Value::quantity(l.number.rem(&right)?, l.unit.clone())
}

fn quotient(l: &Value, r: &Value) -> Result<Value, Error> {
    // Pairs such as km over m cancel inside `quantity`, which scales the number.
    Value::quantity(l.number.div(&r.number)?, l.unit.times(&r.unit.recip()))
}

fn power(l: &Value, r: &Value) -> Result<Value, Error> {
    if !r.unit.is_empty() {
        return Err(Error::UnsupportedOperation(
            "an exponent cannot have a unit",
        ));
    }
    let number = l.number.pow(&r.number)?;
    if l.unit.is_empty() {
        return Ok(Value::plain(number));
    }
    let exponent = i8::try_from(r.number.numer().clone()).map_err(|_| Error::Overflow)?;
    Value::quantity(number, l.unit.with_exponent(exponent))
}

pub fn format_value(v: &Value, opts: &FormatOptions) -> Formatted {
    let mut f = format_value_digits(v, opts);
    f.exact &= !v.approx;
    f
}

fn format_value_digits(v: &Value, opts: &FormatOptions) -> Formatted {
    match v.kind {
        Kind::Plain if v.radix != 10 => Formatted {
            text: radix_text(&v.number, v.radix),
            exact: true,
        },
        Kind::Plain if v.timespan && is_length_of_time(v) => Formatted {
            text: crate::dates::format_timespan(
                &v.number
                    .mul(&v.unit.factor().unwrap_or_else(|_| Number::from_i64(1)))
                    .unwrap_or_else(|_| v.number.clone()),
            ),
            exact: true,
        },
        Kind::Plain => {
            if let Some(f) = format_money(v, opts) {
                return f;
            }
            let mut f = format_decimal(&v.number, opts);
            if !v.unit.is_empty() {
                f.text.push(' ');
                f.text.push_str(&v.unit.symbol());
            }
            f
        }
        Kind::Date => Formatted {
            text: crate::dates::format_date(&v.number),
            exact: true,
        },
        Kind::Clock => Formatted {
            text: crate::dates::format_clock(&v.number),
            exact: true,
        },
        Kind::Text => Formatted {
            text: v.text.as_deref().unwrap_or("").to_string(),
            exact: true,
        },
        Kind::TaxRate => {
            let mut f = format_decimal(&v.number.times_hundred(), &opts.unabbreviated());
            f.text.push('%');
            f
        }
        Kind::Multiplier => {
            let mut f = format_decimal(&v.number, &opts.unabbreviated());
            f.text.push('x');
            f
        }
        Kind::Bool => Formatted {
            text: if v.number.is_zero() { "false" } else { "true" }.to_string(),
            exact: true,
        },
        Kind::Percent => {
            let mut f = format_decimal(&v.number.times_hundred(), &opts.unabbreviated());
            f.text.push('%');
            f
        }
    }
}

/// `$2,985.00`, `-$5.00`, `$386.8M`, `5.00 CHF`, `$30.00/day`; None when the value is not money.
fn format_money(v: &Value, opts: &FormatOptions) -> Option<Formatted> {
    let (currency, rest) = v.unit.currency_split()?;
    if rest.symbol().contains('·') || rest.dim().has_money() {
        return None;
    }
    let abbreviated = opts.abbreviate_large && v.number.abs() >= Number::from_i64(100_000);
    let mut f = match (currency.minor_digits, abbreviated, opts.fixed_money_digits) {
        (Some(minor), false, true) => {
            let mut fixed = opts.with_places(minor.min(opts.decimal_places())).ok()?;
            fixed.trim_trailing_zeros = false;
            format_decimal(&v.number, &fixed)
        }
        (Some(minor), false, false) => {
            // Unrounded, but never fewer digits than the currency shows: `$45,000.00`, `$1.005`.
            let mut f = format_decimal(&v.number, opts);
            f.text = pad_fraction(&f.text, minor);
            f
        }
        _ => format_decimal(&v.number, opts),
    };
    if currency.prefix_symbol {
        let symbol = v.unit.currency_symbol()?;
        let (sign, digits) = match f.text.strip_prefix('-') {
            Some(d) => ("-", d.to_string()),
            None => ("", f.text.clone()),
        };
        f.text = format!("{sign}{symbol}{digits}");
    } else {
        f.text = format!("{} {}", f.text, currency.code);
    }
    if !rest.is_empty() {
        f.text.push('/');
        f.text.push_str(&rest.recip().symbol());
    }
    Some(f)
}

/// Adds zeros so `text` has at least `places` digits after the point.
fn pad_fraction(text: &str, places: u32) -> String {
    let places = places as usize;
    match text.split_once('.') {
        Some((whole, frac)) if frac.len() >= places => format!("{whole}.{frac}"),
        Some((whole, frac)) => format!("{whole}.{frac}{}", "0".repeat(places - frac.len())),
        None if places == 0 => text.to_string(),
        None => format!("{text}.{}", "0".repeat(places)),
    }
}

/// `0xFF`, `0b101`, `0o17`, or digits with a subscript base for any other base.
fn radix_text(n: &Number, radix: u8) -> String {
    let digits = n
        .numer()
        .abs()
        .to_str_radix(u32::from(radix))
        .to_uppercase();
    let sign = if n.is_negative() { "-" } else { "" };
    match radix {
        16 => format!("{sign}0x{digits}"),
        2 => format!("{sign}0b{digits}"),
        8 => format!("{sign}0o{digits}"),
        _ => {
            let subscript: String = radix
                .to_string()
                .chars()
                .map(|c| {
                    ['₀', '₁', '₂', '₃', '₄', '₅', '₆', '₇', '₈', '₉']
                        [c.to_digit(10).unwrap_or(0) as usize]
                })
                .collect();
            format!("{sign}{digits}{subscript}")
        }
    }
}
