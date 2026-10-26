use crate::error::Error;
use crate::format::{FormatOptions, Formatted, format_decimal};
use crate::number::Number;
use crate::parser::{BinOp, Phrase};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Plain,
    /// The stored number is the fraction: 10% is held as 1/10.
    Percent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Value {
    number: Number,
    kind: Kind,
}

const NOT_YET: &str = "this combination of percentages is not defined";

fn one() -> Number {
    Number::from_i64(1)
}

impl Value {
    pub fn plain(number: Number) -> Value {
        Value { number, kind: Kind::Plain }
    }

    pub fn percent(fraction: Number) -> Value {
        Value { number: fraction, kind: Kind::Percent }
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// The stored number. For a percentage this is the fraction, not the figure shown.
    pub fn number(&self) -> &Number {
        &self.number
    }

    pub fn into_plain(self) -> Result<Number, Error> {
        match self.kind {
            Kind::Plain => Ok(self.number),
            Kind::Percent => Err(Error::NotAPlainNumber),
        }
    }

    pub fn neg(&self) -> Value {
        Value { number: self.number.neg(), kind: self.kind }
    }

    pub fn into_percent_literal(self) -> Result<Value, Error> {
        match self.kind {
            Kind::Plain => Ok(Value::percent(self.number.div(&Number::from_i64(100))?)),
            Kind::Percent => Err(Error::UnsupportedOperation("a percentage of a percentage")),
        }
    }

    pub fn binary(op: BinOp, l: &Value, r: &Value) -> Result<Value, Error> {
        use Kind::{Percent, Plain};
        match (op, l.kind, r.kind) {
            (BinOp::Add | BinOp::Sub, Plain, Plain) => Ok(Value::plain(arith(op, &l.number, &r.number)?)),
            (BinOp::Add | BinOp::Sub, Percent, Percent | Plain) => {
                Ok(Value::percent(arith(op, &l.number, &r.number)?))
            }
            (BinOp::Add | BinOp::Sub, Plain, Percent) => {
                Ok(Value::plain(l.number.mul(&adjust(op, &r.number)?)?))
            }
            (BinOp::Mul, Plain, Plain) | (BinOp::Mul, Percent, Plain) | (BinOp::Mul, Plain, Percent) => {
                Ok(Value::plain(l.number.mul(&r.number)?))
            }
            (BinOp::Div, Plain, Plain) => Ok(Value::plain(l.number.div(&r.number)?)),
            (BinOp::Pow, Plain, Plain) => Ok(Value::plain(l.number.pow(&r.number)?)),
            (BinOp::On | BinOp::Off, _, Plain) => {
                let sign = if op == BinOp::On { BinOp::Add } else { BinOp::Sub };
                Ok(Value::plain(r.number.mul(&adjust(sign, &l.number)?)?))
            }
            (BinOp::On | BinOp::Off, ..) => Err(Error::UnsupportedOperation("'on' and 'off' need a plain number after them")),
            _ => Err(Error::UnsupportedOperation(NOT_YET)),
        }
    }

    pub fn phrase(phrase: Phrase, args: &[Value]) -> Result<Value, Error> {
        use Kind::{Percent, Plain};
        match (phrase, args) {
            (Phrase::AsPercent, [v]) => match v.kind {
                Plain => Ok(Value::percent(v.number.clone())),
                Percent => Ok(v.clone()),
            },
            (Phrase::IsPercentOfWhat, [a, p]) if a.kind == Plain => {
                Ok(Value::plain(a.number.div(&p.number)?))
            }
            (Phrase::IsPercentOnWhat, [a, p]) if a.kind == Plain => {
                Ok(Value::plain(a.number.div(&adjust(BinOp::Add, &p.number)?)?))
            }
            (Phrase::IsPercentOffWhat, [a, p]) if a.kind == Plain => {
                Ok(Value::plain(a.number.div(&adjust(BinOp::Sub, &p.number)?)?))
            }
            (Phrase::WhatPercentOf, [a, b]) if a.kind == Plain && b.kind == Plain => {
                Ok(Value::percent(a.number.div(&b.number)?))
            }
            (Phrase::WhatPercentOn, [a, b]) if a.kind == Plain && b.kind == Plain => {
                Ok(Value::percent(a.number.sub(&b.number)?.div(&b.number)?))
            }
            (Phrase::WhatPercentOff, [a, b]) if a.kind == Plain && b.kind == Plain => {
                Ok(Value::percent(b.number.sub(&a.number)?.div(&b.number)?))
            }
            (Phrase::PercentChange, [a, b]) if a.kind == Plain && b.kind == Plain => {
                Ok(Value::percent(b.number.sub(&a.number)?.div(&a.number)?))
            }
            _ => Err(Error::UnsupportedOperation("this phrase needs plain numbers and, where written, a percentage")),
        }
    }
}

fn arith(op: BinOp, a: &Number, b: &Number) -> Result<Number, Error> {
    if op == BinOp::Add { a.add(b) } else { a.sub(b) }
}

/// 1 + p for Add, 1 - p for Sub.
fn adjust(op: BinOp, p: &Number) -> Result<Number, Error> {
    arith(op, &one(), p)
}

pub fn format_value(v: &Value, opts: &FormatOptions) -> Formatted {
    match v.kind {
        Kind::Plain => format_decimal(&v.number, opts),
        Kind::Percent => {
            let mut f = format_decimal(&v.number.times_hundred(), opts);
            f.text.push('%');
            f
        }
    }
}
