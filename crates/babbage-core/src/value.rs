use crate::error::Error;
use crate::format::{FormatOptions, Formatted, format_decimal};
use crate::number::Number;
use crate::parser::{BinOp, Phrase};
use crate::units::UnitExpr;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Plain,
    /// The stored number is the fraction: 10% is held as 1/10.
    Percent,
}

/// A number with an optional unit. A percentage never has a unit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Value {
    number: Number,
    kind: Kind,
    unit: UnitExpr,
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
        }
    }

    pub fn percent(fraction: Number) -> Value {
        Value {
            number: fraction,
            kind: Kind::Percent,
            unit: UnitExpr::default(),
        }
    }

    /// A number in a unit. Units that cancel to nothing measurable (km/m) become a plain number.
    pub fn quantity(number: Number, unit: UnitExpr) -> Result<Value, Error> {
        if !unit.is_empty() && unit.dim().is_none() {
            return Ok(Value::plain(number.mul(&unit.factor()?)?));
        }
        Ok(Value {
            number,
            kind: Kind::Plain,
            unit,
        })
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

    pub fn neg(&self) -> Value {
        Value {
            number: self.number.neg(),
            kind: self.kind,
            unit: self.unit.clone(),
        }
    }

    pub fn apply_unit(self, unit: &UnitExpr) -> Result<Value, Error> {
        if self.kind != Kind::Plain || !self.unit.is_empty() {
            return Err(Error::UnsupportedOperation(
                "a unit can only follow a plain number",
            ));
        }
        Value::quantity(self.number, unit.clone())
    }

    pub fn cast(self, target: &UnitExpr) -> Result<Value, Error> {
        if self.kind != Kind::Plain || self.unit.is_empty() {
            return Err(Error::IncompatibleUnits);
        }
        Value::quantity(self.number, target.clone())
    }

    pub fn convert(self, target: &UnitExpr) -> Result<Value, Error> {
        if self.kind != Kind::Plain || self.unit.is_empty() || self.unit.dim() != target.dim() {
            return Err(Error::IncompatibleUnits);
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
        use Kind::{Percent, Plain};
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
        use Kind::{Percent, Plain};
        let refuse = || Err(Error::UnsupportedOperation(PHRASE_NEEDS_PLAIN));
        match (phrase, args) {
            (Phrase::AsPercent, [v]) => match v.kind {
                Plain if v.unit.is_empty() => Ok(Value::percent(v.number.clone())),
                Percent => Ok(v.clone()),
                Plain => refuse(),
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
                return Err(Error::IncompatibleUnits);
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
    let same_kind_different_unit =
        !l.unit.is_empty() && l.unit != r.unit && l.unit.dim() == r.unit.dim();
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

fn quotient(l: &Value, r: &Value) -> Result<Value, Error> {
    let number = l.number.div(&r.number)?;
    if !l.unit.is_empty() && l.unit != r.unit && l.unit.dim() == r.unit.dim() {
        let ratio = l.unit.factor()?.div(&r.unit.factor()?)?;
        return Ok(Value::plain(number.mul(&ratio)?));
    }
    Value::quantity(number, l.unit.times(&r.unit.recip()))
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
    match v.kind {
        Kind::Plain => {
            let mut f = format_decimal(&v.number, opts);
            if !v.unit.is_empty() {
                f.text.push(' ');
                f.text.push_str(&v.unit.symbol());
            }
            f
        }
        Kind::Percent => {
            let mut f = format_decimal(&v.number.times_hundred(), opts);
            f.text.push('%');
            f
        }
    }
}
