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
    /// How to write a quantity when it is not a single number and unit: `3 weeks 5 days`, `03:04:05`.
    layout: Option<Shown>,
}

/// A way of writing a length (usually of time) as several parts, chosen with `as timespan`,
/// `as laptime` or `in hours and minutes`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Shown {
    /// `3 weeks 5 days`
    Timespan,
    /// `03:04:05` and `00:00:01.5`
    Laptime,
    /// `235 h 12 min`: whole amounts of the larger units, then the rest in the last.
    Parts(Vec<UnitExpr>),
    /// Every digit, with no separators and no k, M or G: a Unix timestamp.
    Digits,
    /// `01:02:03:04` at this many whole frames per second.
    Timecode(Number),
    /// `1/5`: the exact numerator and denominator, as `as fraction` shows a number.
    Fraction,
    /// `1/20` for `5% is 1 in what`: this top number over the top number divided by the value.
    Ratio(Number),
    /// A date written with a time that happens to be midnight (`April 1, 2019 12:00am`), so it is not a bare date.
    Timed,
    /// `5 km in 25 min`: minutes and seconds per this unit, written `05:00/km`.
    Pace(String),
    /// A time of day converted into UTC reads on the 24-hour clock: `19:00`.
    Clock24,
    /// `10500 in sci` reads `1.05e4`.
    Scientific,
    /// `5' 6"` reads `5 feet 6 inches`.
    FeetInches,
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
            layout: None,
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
            layout: None,
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
            layout: None,
        })
    }

    /// This value's number restated in the unit of `like`, which must measure the same thing.
    pub fn convert_to_unit_of(self, like: &Value) -> Result<Number, Error> {
        if self.unit == like.unit {
            return Ok(self.number);
        }
        self.convert(&like.unit).map(|v| v.number)
    }

    /// A date and time. A date written with no time counts as midday when it is turned into a timestamp, so
    /// a time that is exactly midnight is marked, to stay midnight.
    pub fn date_at(days: Number, explicit_time: bool) -> Value {
        let mut v = Value::date(days);
        if explicit_time && v.number.is_integer() {
            v.layout = Some(Shown::Timed);
        }
        v
    }

    /// True for a date written with no time of day.
    pub fn is_bare_date(&self) -> bool {
        self.kind == Kind::Date && self.number.is_integer() && self.layout != Some(Shown::Timed)
    }

    pub fn date(days: Number) -> Value {
        Value {
            number: days,
            kind: Kind::Date,
            unit: UnitExpr::default(),
            approx: false,
            radix: 10,
            text: None,
            layout: None,
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
            layout: None,
        }
    }

    /// A length written `5' 6"`, shown as feet and inches.
    pub fn feet_inches(feet: Number, inches: Number) -> Result<Value, Error> {
        let total = feet.add(&inches.div(&Number::from_i64(12))?)?;
        let foot = crate::units::lookup("ft").ok_or(Error::IncompatibleUnits)?;
        let mut v = Value::quantity(total, foot)?;
        v.layout = Some(Shown::FeetInches);
        Ok(v)
    }

    /// A time of day read on the 24-hour clock.
    pub fn clock_24(seconds: Number) -> Value {
        let mut v = Value::clock(seconds);
        v.layout = Some(Shown::Clock24);
        v
    }

    pub fn text(words: &str) -> Value {
        Value {
            number: Number::from_i64(0),
            kind: Kind::Text,
            unit: UnitExpr::default(),
            approx: false,
            radix: 10,
            text: Some(words.into()),
            layout: None,
        }
    }

    /// This length of time, to be shown as a timespan.
    pub fn as_timespan(mut self) -> Value {
        self.layout = Some(Shown::Timespan);
        self
    }

    pub fn as_laptime(mut self) -> Value {
        self.layout = Some(Shown::Laptime);
        self
    }

    /// A length of time shown as hours, minutes, seconds and frames at a whole number of frames per second.
    pub fn as_timecode(mut self, frames_per_second: Number) -> Value {
        self.layout = Some(Shown::Timecode(frames_per_second));
        self
    }

    /// The frame rate this value is written at, when it is a timecode.
    pub fn timecode_rate(&self) -> Option<&Number> {
        match &self.layout {
            Some(Shown::Timecode(rate)) => Some(rate),
            _ => None,
        }
    }

    /// This number as `top/(top / this)`, for `5% is 1 in what`.
    pub fn as_ratio_of(self, top: Number) -> Value {
        Value {
            layout: Some(Shown::Ratio(top)),
            ..self
        }
    }

    /// A whole number shown with every digit, like a Unix timestamp.
    pub fn written_in_full(mut self) -> Value {
        self.layout = Some(Shown::Digits);
        self
    }

    /// Chooses how a quantity is written, refusing a layout its dimension cannot have.
    pub fn show(self, shown: Shown) -> Result<Value, Error> {
        if shown == Shown::Fraction {
            return match (self.kind, self.unit.is_empty()) {
                (Kind::Plain | Kind::Percent, true) => Ok(Value {
                    kind: Kind::Plain,
                    layout: Some(shown),
                    ..self
                }),
                _ => Err(Error::UnsupportedOperation(
                    "a fraction needs a plain number or a percentage",
                )),
            };
        }
        if shown == Shown::Scientific {
            return match (self.kind, self.unit.is_empty()) {
                (Kind::Plain, true) => Ok(Value {
                    layout: Some(shown),
                    ..self
                }),
                _ => Err(Error::UnsupportedOperation(
                    "scientific form is for plain numbers",
                )),
            };
        }
        if self.kind != Kind::Plain || self.unit.is_empty() {
            return Err(Error::IncompatibleUnits);
        }
        match &shown {
            Shown::Timespan | Shown::Laptime if !is_length_of_time(&self) => {
                return Err(Error::UnsupportedOperation("this needs a length of time"));
            }
            Shown::Parts(units) => {
                for u in units {
                    if u.dim() != self.unit.dim() {
                        return Err(mismatch(&self.unit, u));
                    }
                }
            }
            _ => {}
        }
        Ok(Value {
            layout: Some(shown),
            ..self
        })
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
            layout: None,
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
            layout: None,
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
            layout: None,
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
    /// An amount of one currency rounded to that currency's own digits (`$3.33`, `¥33`), which is what a line
    /// reference or a subtotal uses. Anything that is not a plain amount of one currency comes back unchanged.
    pub fn rounded_to_currency(&self) -> Value {
        let Some((currency, rest)) = self.unit.currency_split() else {
            return self.clone();
        };
        let Some(minor) = currency.minor_digits else {
            return self.clone();
        };
        if self.kind != Kind::Plain || !rest.is_empty() {
            return self.clone();
        }
        let step = Number::from_i64(1).div(
            &Number::from_i64(10)
                .pow(&Number::from_i64(i64::from(minor)))
                .unwrap_or_else(|_| Number::from_i64(1)),
        );
        match step.and_then(|s| self.map_number(|n| round_to_multiple(n, &s, Rounding::Nearest))) {
            Ok(v) => Value {
                approx: self.approx,
                ..v
            },
            Err(_) => self.clone(),
        }
    }

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
            layout: None,
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
        Ok(a.si_against(b)?.cmp(&b.si_against(a)?))
    }

    pub fn neg(&self) -> Value {
        Value {
            number: self.number.neg(),
            kind: self.kind,
            unit: self.unit.clone(),
            approx: self.approx,
            radix: self.radix,
            text: self.text.clone(),
            layout: self.layout.clone(),
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
        // A plain number shown as a rate of time over time: `3 hours / 3 days as hour/day` is `1 hour/day`.
        // A ratio of two amounts shown as a rate keeps its units: `8g/2 lb as grams/lb` is `4 g/lb`.
        let is_ratio_of_units =
            target.dim().is_none() && !target.is_empty() && target.has_division();
        if self.kind == Kind::Plain
            && self.unit.is_empty()
            && (target.is_time_over_time() || is_ratio_of_units)
        {
            return Ok(Value {
                number: self.number.div(&target.factor()?)?,
                kind: Kind::Plain,
                unit: target.clone(),
                approx: self.approx,
                radix: 10,
                text: None,
                layout: None,
            });
        }
        if self.kind != Kind::Plain || self.unit.is_empty() {
            return Err(Error::IncompatibleUnits);
        }
        if self.unit.dim() != target.dim() {
            if let Some(adapted) = self.unit.adapt_target(target) {
                return self.convert_exact(&adapted);
            }
            return Err(mismatch(&self.unit, target));
        }
        if self.unit.has_offset() || target.has_offset() {
            return convert_scale(self, target);
        }
        let number = self
            .number
            .mul(&self.unit.factor_against(target)?)?
            .div(&target.factor_against(&self.unit)?)?;
        // A rate of one length of time over another (`hours/day`) stays a rate when it is converted.
        if !target.is_empty() && target.dim().is_none() && is_time_rate(&self.unit) {
            return Ok(Value {
                number,
                kind: Kind::Plain,
                unit: target.clone(),
                approx: self.approx,
                radix: 10,
                text: None,
                layout: None,
            });
        }
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

    /// The value in SI base units, for comparing quantities in different units. Set against `other`, calendar
    /// units are counted in working time when either side holds a workday.
    fn si_against(&self, other: &Value) -> Result<Number, Error> {
        if self.unit.has_offset() {
            return Err(Error::UnsupportedOperation(TEMPERATURE));
        }
        if self.unit.is_empty() {
            Ok(self.number.clone())
        } else {
            self.number.mul(&self.unit.factor_against(&other.unit)?)
        }
    }

    /// `$30/day is what per year`: this rate restated per another period.
    pub fn per_period(self, period: &UnitExpr) -> Result<Value, Error> {
        if self.kind != Kind::Plain {
            return Err(Error::IncompatibleUnits);
        }
        let target = self
            .unit
            .with_period(period)
            .ok_or(Error::UnsupportedOperation(
                "this needs a rate with a period, such as $30/day",
            ))?;
        self.convert(&target)
    }

    /// `3 hours / day`, `$99 per week`, `30 / week`: the value over one of a unit. Over a different unit of time
    /// this is a rate that keeps its units; anything else divides as usual.
    pub fn per_unit(self, unit: &UnitExpr) -> Result<Value, Error> {
        let unit_value = Value::quantity(Number::from_i64(1), unit.clone())?;
        if is_length_of_time(&self) && is_length_of_time(&unit_value) && self.unit != *unit {
            return Ok(Value {
                number: self.number,
                kind: Kind::Plain,
                unit: self.unit.times(&unit.recip()),
                approx: self.approx,
                radix: 10,
                text: None,
                layout: None,
            });
        }
        Value::binary(BinOp::Div, &self, &unit_value)
    }

    pub fn binary(op: BinOp, l: &Value, r: &Value) -> Result<Value, Error> {
        let approx = l.approx || r.approx || reconciles_approximate_units(&[&l.unit, &r.unit]);
        Value::binary_exact(op, l, r).map(|v| inherit_layout(op, l, r, v.tainted(approx)))
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
            (Phrase::Pace, [distance, time])
                if distance.kind == Plain
                    && time.kind == Plain
                    && is_length_of_distance(distance)
                    && is_length_of_time(time) =>
            {
                // Seconds for one of the distance's unit, written as minutes and seconds per that unit.
                let seconds = time
                    .number
                    .mul(&time.unit.factor()?)?
                    .div(&distance.number)?;
                let seconds_unit = crate::units::lookup("s").ok_or(Error::OutOfDomain)?;
                Ok(Value {
                    number: seconds,
                    kind: Plain,
                    unit: seconds_unit.times(&distance.unit.recip()),
                    approx: distance.approx || time.approx,
                    radix: 10,
                    text: None,
                    layout: Some(Shown::Pace(distance.unit.symbol())),
                })
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
                let (x, y) = (a.si_against(b)?, b.si_against(a)?);
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
                let (x, y) = (a.si_against(b)?, b.si_against(a)?);
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

/// `n` for a sign of 1 and `-n` for a sign of -1.
fn signed(n: i64, sign: i64) -> i64 {
    if sign.is_negative() { -n } else { n }
}

fn is_length_of_time(v: &Value) -> bool {
    let seconds = crate::units::lookup("s").expect("seconds are defined");
    v.kind == Kind::Plain && !v.unit.is_empty() && v.unit.dim() == seconds.dim()
}

/// True for a length such as `5 km` or `10 mi` (a quantity measured in a unit of distance).
fn is_length_of_distance(v: &Value) -> bool {
    let metre = crate::units::lookup("m").expect("metres are defined");
    v.kind == Kind::Plain && !v.unit.is_empty() && v.unit.dim() == metre.dim()
}

/// True for a unit such as `hours/day`: time over time, which has no dimension left.
fn is_time_rate(unit: &UnitExpr) -> bool {
    unit.is_time_over_time()
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
        let n = signed(i64::try_from(m.numer()).map_err(|_| Error::Overflow)?, sign);
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
        let n = signed(
            i64::try_from(by.number.numer()).map_err(|_| Error::Overflow)?,
            sign,
        );
        // Far enough that the date could not be written down; refused before it can wrap.
        if n.unsigned_abs() > 1_000_000_000_000 {
            return Err(Error::Overflow);
        }
        return Ok(Value::date(
            Number::from_i64(calendar::add_workdays(day, n)).add(&fraction)?,
        ));
    }
    let days = seconds_of(by)?
        .div(&Number::from_i64(86_400))?
        .mul(&sign_n)?;
    // A date written with no time is midday, so hours added to it make a time: `Tuesday next week + 72 hours`
    // is `November 6, 2026 at 12:00 PM`, as the reference app prints it.
    let sub_day = ["h", "min", "s"].iter().any(|u| by.unit == lookup(u));
    let start = if date.is_bare_date() && sub_day {
        date.number.add(&Number::from_ratio(
            num_bigint::BigInt::from(1),
            num_bigint::BigInt::from(2),
        )?)?
    } else {
        date.number.clone()
    };
    Ok(Value::date(start.add(&days)?))
}

/// A time of day shifted by a length of time. Past midnight it is a time on the day before or after today:
/// `9:45 am - 15 hours 10 minutes` is `Yesterday at 6:35 PM`.
fn shift_clock(clock: &Value, by: &Value, sign: i64) -> Result<Value, Error> {
    let moved = clock
        .number
        .add(&seconds_of(by)?.mul(&Number::from_i64(sign))?)?;
    let day = Number::from_i64(86_400);
    let days_over = i64::try_from(&moved.div(&day)?.floor()).map_err(|_| Error::Overflow)?;
    if days_over == 0 {
        return Ok(Value::clock(moved));
    }
    let (today, _) = crate::settings::local_now();
    Ok(Value::date(Number::from_i64(today + days_over).add(
        &moved.div(&day)?.sub(&Number::from_i64(days_over))?,
    )?))
}

/// A sum or difference of two lengths of time is written the way its operands were: a timespan stays a
/// timespan and a lap time stays a lap time.
fn inherit_layout(op: BinOp, l: &Value, r: &Value, v: Value) -> Value {
    let wanted = |x: &Value| {
        matches!(
            x.layout,
            Some(Shown::Timespan | Shown::Laptime | Shown::Timecode(_))
        )
    };
    if !matches!(op, BinOp::Add | BinOp::Sub | BinOp::Join) || !is_length_of_time(&v) {
        return v;
    }
    // Several time units written one after another (`3 hours 5 minutes 10 seconds`) print as a timespan.
    if op == BinOp::Join && !wanted(l) && !wanted(r) && v.layout.is_none() {
        return Value {
            layout: Some(Shown::Timespan),
            ..v
        };
    }
    let layout = if wanted(l) {
        l.layout.clone()
    } else if wanted(r) {
        r.layout.clone()
    } else {
        return v;
    };
    Value { layout, ..v }
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
            Ok(Value::quantity(l.number.sub(&r.number)?.abs(), seconds())?.as_timespan())
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
        let gap = l.si_against(r)?.sub(&r.si_against(l)?)?.abs();
        let scale = r.si_against(l)?.abs().max(Number::from_i64(1));
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
            let (fl, fr) = (
                l.unit.factor_against(&r.unit)?,
                r.unit.factor_against(&l.unit)?,
            );
            // Two rates meet in the one with the longer period (`$20/day + $300/week` is per week); other
            // units meet in the larger unit.
            let rates = l.unit.has_denominator() && r.unit.has_denominator();
            let right_wins = if rates { fr < fl } else { fr > fl };
            let target = if right_wins { &r.unit } else { &l.unit };
            let ft = if right_wins { &fr } else { &fl };
            let (a, b) = (l.number.mul(&fl)?.div(ft)?, r.number.mul(&fr)?.div(ft)?);
            Value::quantity(arith(op, &a, &b)?, target.clone())
        }
    }
}

/// For `ft²` times `in` (powers of two different length units), the product's unit in the left length unit and
/// the factor that restates the right number in it.
fn length_powers_in_left_unit(l: &Value, r: &Value) -> Result<Option<(UnitExpr, Number)>, Error> {
    let (Some((lu, la)), Some((ru, ra))) = (l.unit.single_power(), r.unit.single_power()) else {
        return Ok(None);
    };
    if lu == ru || la <= 0 || ra <= 0 || !lu.is_length() || !ru.is_length() {
        return Ok(None);
    }
    // The right number, one length unit at a time: (right unit / left unit) to the power of its exponent.
    let per = ru.factor()?.div(&lu.factor()?)?;
    let scale = per.pow(&Number::from_i64(i64::from(ra)))?;
    Ok(Some((lu.with_exponent(la + ra), scale)))
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
    // Lengths of different units multiply in the left one's length unit: `10 ft x 12 ft x 6 in` is 60 ft³.
    if let Some((unit, scale)) = length_powers_in_left_unit(l, r)? {
        return Value::quantity(l.number.mul(&r.number)?.mul(&scale)?, unit);
    }
    let same_kind_different_unit = l.unit != r.unit && l.unit.dim() == r.unit.dim();
    let right = if same_kind_different_unit {
        r.number
            .mul(&r.unit.factor_against(&l.unit)?)?
            .div(&l.unit.factor_against(&r.unit)?)?
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
        r.number
            .mul(&r.unit.factor_against(&l.unit)?)?
            .div(&l.unit.factor_against(&r.unit)?)?
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
        Kind::Plain if v.layout == Some(Shown::Digits) => {
            format_decimal(&v.number, &opts.every_digit())
        }
        Kind::Plain if v.layout == Some(Shown::Scientific) && v.unit.is_empty() => {
            crate::format::format_scientific(&v.number, opts)
        }
        Kind::Plain if v.layout == Some(Shown::Fraction) => {
            let whole = |n: &num_bigint::BigInt| {
                Number::from_ratio(n.clone(), num_bigint::BigInt::from(1))
                    .map(|n| format_decimal(&n, &opts.unabbreviated()).text)
                    .unwrap_or_else(|_| n.to_string())
            };
            Formatted {
                text: if v.number.is_integer() {
                    whole(v.number.numer())
                } else {
                    format!("{}/{}", whole(v.number.numer()), whole(v.number.denom()))
                },
                exact: true,
            }
        }
        Kind::Plain if matches!(v.layout, Some(Shown::Ratio(_))) => {
            let Some(Shown::Ratio(top)) = &v.layout else {
                unreachable!("matched by the guard")
            };
            let below = top.div(&v.number).unwrap_or_else(|_| top.clone());
            let (a, b) = (
                format_decimal(top, &opts.unabbreviated()),
                format_decimal(&below, &opts.unabbreviated()),
            );
            Formatted {
                text: format!("{}/{}", a.text, b.text),
                exact: a.exact && b.exact,
            }
        }
        Kind::Plain if v.layout.is_some() && !v.unit.is_empty() => format_layout(v, opts),
        Kind::Plain => {
            if let Some(f) = format_money(v, opts) {
                return f;
            }
            let mut f = if v.unit.is_empty() || v.unit.currency_symbol().is_some() {
                format_decimal(&v.number, opts)
            } else {
                crate::format::format_amount(&v.number, opts)
            };
            if !v.unit.is_empty() {
                let plural = v.number.abs() != Number::from_i64(1);
                let symbol = v.unit.display(plural);
                // the reference app writes pixels and degrees with no space (`128.3464566929px`, `180°`), CSS
                // units the same way, and a rate that has no numerator unit straight after the number (`30/week`).
                if !matches!(symbol.as_str(), "px" | "em" | "rem" | "°") && !symbol.starts_with('/')
                {
                    f.text.push(' ');
                }
                f.text.push_str(&symbol);
            }
            f
        }
        Kind::Date => Formatted {
            text: crate::dates::format_date(&v.number),
            exact: true,
        },
        Kind::Clock => Formatted {
            text: if v.layout == Some(Shown::Clock24) {
                crate::dates::format_clock_24(&v.number)
            } else {
                crate::dates::format_clock(&v.number)
            },
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
            f.text = pad_fraction(&f.text, minor, opts.number_format.decimal_mark());
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
        f.text.push_str(&rest.recip().display(false));
    }
    Some(f)
}

/// Adds zeros so `text` has at least `places` digits after the point.
fn pad_fraction(text: &str, places: u32, mark: char) -> String {
    let places = places as usize;
    match text.split_once(mark) {
        Some((whole, frac)) if frac.len() >= places => format!("{whole}{mark}{frac}"),
        Some((whole, frac)) => format!("{whole}{mark}{frac}{}", "0".repeat(places - frac.len())),
        None if places == 0 => text.to_string(),
        None => format!("{text}{mark}{}", "0".repeat(places)),
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

/// A quantity written as `3 weeks 5 days`, `03:04:05` or `235 h 12 min`.
fn format_layout(v: &Value, opts: &FormatOptions) -> Formatted {
    if let Some(Shown::Pace(per)) = &v.layout {
        let (text, exact) = crate::dates::format_laptime(&v.number);
        let text = text.strip_prefix("00:").unwrap_or(&text);
        return Formatted {
            text: format!("{text}/{per}"),
            exact,
        };
    }
    let base = v
        .number
        .mul(&v.unit.factor().unwrap_or_else(|_| Number::from_i64(1)))
        .unwrap_or_else(|_| v.number.clone());
    match v.layout.as_ref().expect("checked by the caller") {
        Shown::Timespan => {
            let (text, exact) = crate::dates::format_timespan(&base);
            Formatted { text, exact }
        }
        Shown::Laptime => {
            let (text, exact) = crate::dates::format_laptime(&base);
            Formatted { text, exact }
        }
        Shown::Parts(units) => format_parts(&base, units, opts),
        Shown::FeetInches => {
            let inches = base
                .div(&Number::from_decimal_str("0.0254").expect("a decimal"))
                .unwrap_or_else(|_| base.clone());
            let feet = Number::from_ratio(
                inches
                    .div(&Number::from_i64(12))
                    .map_or_else(|_| num_bigint::BigInt::from(0), |n| n.floor()),
                num_bigint::BigInt::from(1),
            )
            .unwrap_or_else(|_| Number::from_i64(0));
            let rest = inches
                .sub(
                    &feet
                        .mul(&Number::from_i64(12))
                        .unwrap_or_else(|_| feet.clone()),
                )
                .unwrap_or_else(|_| inches.clone());
            let foot = |n: &Number| {
                let f = crate::format::format_decimal(n, opts);
                (f.text.clone(), f.exact, *n == Number::from_i64(1))
            };
            let mut text = Vec::new();
            let mut exact = true;
            if !feet.is_zero() || rest.is_zero() {
                let (t, e, one) = foot(&feet);
                exact &= e;
                text.push(format!("{t} {}", if one { "foot" } else { "feet" }));
            }
            if !rest.is_zero() {
                let (t, e, one) = foot(&rest);
                exact &= e;
                text.push(format!("{t} {}", if one { "inch" } else { "inches" }));
            }
            Formatted {
                text: text.join(" "),
                exact,
            }
        }
        Shown::Timecode(rate) => {
            let (text, exact) = crate::dates::format_timecode(&base, rate);
            Formatted { text, exact }
        }
        Shown::Digits
        | Shown::Fraction
        | Shown::Ratio(_)
        | Shown::Timed
        | Shown::Pace(_)
        | Shown::Scientific
        | Shown::Clock24 => {
            unreachable!("written out before this is called")
        }
    }
}

/// Whole amounts of each larger unit in turn, then what is left in the last unit.
fn format_parts(base: &Number, units: &[UnitExpr], opts: &FormatOptions) -> Formatted {
    let mut sized: Vec<(Number, &UnitExpr)> = units
        .iter()
        .filter_map(|u| u.factor().ok().map(|f| (f, u)))
        .collect();
    sized.sort_by(|a, b| b.0.cmp(&a.0));
    let negative = base.is_negative();
    let mut rest = base.abs();
    let mut parts: Vec<String> = Vec::new();
    let mut exact = true;
    let last = sized.len().saturating_sub(1);
    for (i, (factor, unit)) in sized.iter().enumerate() {
        let amount = rest.div(factor).unwrap_or_else(|_| Number::from_i64(0));
        let (shown, left) = if i == last {
            (amount, Number::from_i64(0))
        } else {
            let whole = Number::from_ratio(amount.floor(), num_bigint::BigInt::from(1))
                .unwrap_or_else(|_| Number::from_i64(0));
            let left = rest
                .sub(&whole.mul(factor).unwrap_or_else(|_| Number::from_i64(0)))
                .unwrap_or_else(|_| Number::from_i64(0));
            (whole, left)
        };
        if !shown.is_zero() || (i == last && parts.is_empty()) {
            let f = format_decimal(&shown, &opts.unabbreviated());
            exact &= f.exact;
            parts.push(format!(
                "{} {}",
                f.text,
                unit.display(shown != Number::from_i64(1))
            ));
        }
        rest = left;
    }
    Formatted {
        text: format!("{}{}", if negative { "-" } else { "" }, parts.join(" ")),
        exact,
    }
}
