use std::fmt;

use crate::error::Error;
use crate::lexer::{Tok, Token};
use crate::number::Number;
use crate::units::UnitExpr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    On,
    Off,
    /// Adjacent quantities such as `1 h 30 min`: a sum that binds tighter than everything but a unit's own exponent.
    Join,
}

impl BinOp {
    fn precedence(self) -> u8 {
        match self {
            BinOp::Add | BinOp::Sub => 1,
            BinOp::Mul | BinOp::Div | BinOp::On | BinOp::Off => 2,
            BinOp::Pow => 4,
            BinOp::Join => 5,
        }
    }

    fn right_associative(self) -> bool {
        self == BinOp::Pow
    }
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Pow => "^",
            BinOp::On => "on",
            BinOp::Off => "off",
            BinOp::Join => "+",
        })
    }
}

/// Multi-word forms such as `20 is 10% of what`. Each pops its operands
/// (first-written operand deepest) and pushes one value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phrase {
    IsPercentOfWhat,
    IsPercentOnWhat,
    IsPercentOffWhat,
    WhatPercentOf,
    WhatPercentOn,
    WhatPercentOff,
    PercentChange,
    AsPercent,
}

impl Phrase {
    pub fn arity(self) -> usize {
        if self == Phrase::AsPercent { 1 } else { 2 }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Num(Number),
    Neg,
    Bin(BinOp),
    /// Postfix `%`: the top value becomes a percentage of itself divided by 100.
    Percent,
    /// Postfix unit: `5 km` is the number 5 followed by this.
    Unit(UnitExpr),
    /// `... in m`: restate the value in another unit of the same kind.
    Convert(UnitExpr),
    /// `... cast m`: relabel the number with another unit without converting.
    Cast(UnitExpr),
    Phrase(Phrase),
}

/// A well-formed expression in reverse Polish order. Parsing, evaluation and
/// printing are all iterative, so nesting depth cannot overflow the stack.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    rpn: Vec<Node>,
}

impl Expr {
    pub fn num(n: Number) -> Expr {
        Expr {
            rpn: vec![Node::Num(n)],
        }
    }

    pub fn negated(mut self) -> Expr {
        self.rpn.push(Node::Neg);
        self
    }

    pub fn bin(op: BinOp, mut lhs: Expr, rhs: Expr) -> Expr {
        lhs.rpn.extend(rhs.rpn);
        lhs.rpn.push(Node::Bin(op));
        lhs
    }

    pub fn with_unit(mut self, unit: UnitExpr) -> Expr {
        self.rpn.push(Node::Unit(unit));
        self
    }

    pub fn converted_to(mut self, unit: UnitExpr) -> Expr {
        self.rpn.push(Node::Convert(unit));
        self
    }

    pub fn percent(mut self) -> Expr {
        self.rpn.push(Node::Percent);
        self
    }

    pub(crate) fn from_nodes(rpn: Vec<Node>) -> Expr {
        assert!(
            is_balanced(&rpn),
            "internal error: expression does not reduce to exactly one value"
        );
        Expr { rpn }
    }

    pub fn nodes(&self) -> &[Node] {
        &self.rpn
    }
}

/// True when evaluating `rpn` never underflows and leaves exactly one value.
fn is_balanced(rpn: &[Node]) -> bool {
    let mut depth: usize = 0;
    for node in rpn {
        let (needs, leaves) = match node {
            Node::Num(_) => (0, 1),
            Node::Neg | Node::Percent | Node::Unit(_) | Node::Convert(_) | Node::Cast(_) => (1, 1),
            Node::Bin(_) => (2, 1),
            Node::Phrase(p) => (p.arity(), 1),
        };
        if depth < needs {
            return false;
        }
        depth = depth - needs + leaves;
    }
    depth == 1
}

enum Pending {
    Neg,
    Bin(BinOp),
    LParen,
}

const NEG_PRECEDENCE: u8 = 3;

impl Pending {
    fn precedence(&self) -> u8 {
        match self {
            Pending::Neg => NEG_PRECEDENCE,
            Pending::Bin(op) => op.precedence(),
            Pending::LParen => 0,
        }
    }

    fn emit(self, out: &mut Vec<Node>) {
        match self {
            Pending::Neg => out.push(Node::Neg),
            Pending::Bin(op) => out.push(Node::Bin(op)),
            Pending::LParen => unreachable!("parentheses are consumed, never emitted"),
        }
    }
}

/// Emits every pending operator that binds at least as tightly as `op`.
fn pop_for(op: BinOp, ops: &mut Vec<Pending>, out: &mut Vec<Node>) {
    while let Some(top) = ops.last() {
        let (tp, p) = (top.precedence(), op.precedence());
        if tp > p || (tp == p && !op.right_associative()) {
            ops.pop().expect("last() was Some").emit(out);
        } else {
            break;
        }
    }
}

pub fn parse(toks: &[Token]) -> Result<Expr, Error> {
    if toks.is_empty() {
        return Err(Error::Empty);
    }
    let mut out: Vec<Node> = Vec::with_capacity(toks.len());
    let mut ops: Vec<Pending> = Vec::new();
    let mut want_operand = true;

    for (index, t) in toks.iter().enumerate() {
        match (&t.tok, want_operand) {
            (Tok::Num(n), true) => {
                out.push(Node::Num(n.clone()));
                want_operand = false;
            }
            (Tok::LParen, true) => ops.push(Pending::LParen),
            (Tok::Minus, true) => ops.push(Pending::Neg),
            (Tok::Plus, true) => {}
            (Tok::Percent { attached: true }, false) => out.push(Node::Percent),
            (Tok::Unit(u), false) => out.push(Node::Unit(u.clone())),
            (Tok::Unit(u), true) => {
                out.push(Node::Num(Number::from_i64(1)));
                out.push(Node::Unit(u.clone()));
                want_operand = false;
            }
            (Tok::Num(n), false)
                if matches!(out.last(), Some(Node::Unit(_)))
                    && matches!(
                        toks.get(index + 1),
                        Some(Token {
                            tok: Tok::Unit(_),
                            ..
                        })
                    ) =>
            {
                // `5 hours 30 minutes` means 5 hours + 30 minutes.
                pop_for(BinOp::Join, &mut ops, &mut out);
                ops.push(Pending::Bin(BinOp::Join));
                out.push(Node::Num(n.clone()));
            }
            (Tok::Plus | Tok::Minus | Tok::Star | Tok::Slash | Tok::Caret, false)
            | (Tok::Word(_), false) => {
                let op = match &t.tok {
                    Tok::Plus => BinOp::Add,
                    Tok::Minus => BinOp::Sub,
                    Tok::Star => BinOp::Mul,
                    Tok::Slash => BinOp::Div,
                    Tok::Caret => BinOp::Pow,
                    Tok::Word(w) if w.eq_ignore_ascii_case("of") => BinOp::Mul,
                    Tok::Word(w) if w.eq_ignore_ascii_case("on") => BinOp::On,
                    Tok::Word(w) if w.eq_ignore_ascii_case("off") => BinOp::Off,
                    Tok::Word(w) => {
                        return Err(Error::UnexpectedWord {
                            word: w.clone(),
                            at: t.at,
                        });
                    }
                    _ => unreachable!("outer pattern admits only operators and words"),
                };
                pop_for(op, &mut ops, &mut out);
                ops.push(Pending::Bin(op));
                want_operand = true;
            }
            (Tok::RParen, false) => loop {
                match ops.pop() {
                    Some(Pending::LParen) => break,
                    Some(p) => p.emit(&mut out),
                    None => {
                        return Err(Error::UnexpectedToken {
                            found: t.tok.describe(),
                            at: t.at,
                        });
                    }
                }
            },
            (Tok::Word(w), true) => {
                return Err(Error::UnexpectedWord {
                    word: w.clone(),
                    at: t.at,
                });
            }
            (tok, _) => {
                return Err(Error::UnexpectedToken {
                    found: tok.describe(),
                    at: t.at,
                });
            }
        }
    }
    if want_operand {
        return Err(Error::UnexpectedEnd);
    }
    while let Some(p) = ops.pop() {
        if matches!(p, Pending::LParen) {
            return Err(Error::UnexpectedEnd);
        }
        p.emit(&mut out);
    }
    Ok(Expr { rpn: out })
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut stack: Vec<String> = Vec::new();
        for node in &self.rpn {
            match node {
                Node::Num(n) => stack.push(match n.to_exact_decimal() {
                    Some(s) => s,
                    None => format!("({}/{})", n.numer(), n.denom()),
                }),
                Node::Neg => {
                    let x = stack.pop().expect("well-formed");
                    stack.push(format!("(-{x})"));
                }
                Node::Percent => {
                    let x = stack.pop().expect("well-formed");
                    stack.push(format!("({x}%)"));
                }
                Node::Unit(u) => {
                    let x = stack.pop().expect("well-formed");
                    stack.push(format!("({x} {})", u.symbol()));
                }
                Node::Convert(u) => {
                    let x = stack.pop().expect("well-formed");
                    stack.push(format!("({x} in {})", u.symbol()));
                }
                Node::Cast(u) => {
                    let x = stack.pop().expect("well-formed");
                    stack.push(format!("({x} cast {})", u.symbol()));
                }
                Node::Phrase(p) => {
                    let args = stack.split_off(stack.len() - p.arity());
                    stack.push(format!("{p:?}({})", args.join(", ")));
                }
                Node::Bin(op) => {
                    let r = stack.pop().expect("well-formed");
                    let l = stack.pop().expect("well-formed");
                    stack.push(format!("({l} {op} {r})"));
                }
            }
        }
        f.write_str(&stack.pop().expect("well-formed"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(v: i64) -> Node {
        Node::Num(Number::from_i64(v))
    }

    #[test]
    fn balance_check_accepts_only_expressions_that_reduce_to_one_value() {
        assert!(is_balanced(&[n(1)]));
        assert!(is_balanced(&[n(1), Node::Neg]));
        assert!(is_balanced(&[n(1), Node::Percent]));
        assert!(is_balanced(&[n(1), n(2), Node::Bin(BinOp::Add)]));
        assert!(is_balanced(&[
            n(1),
            n(2),
            n(3),
            Node::Bin(BinOp::Add),
            Node::Bin(BinOp::Mul)
        ]));
        assert!(is_balanced(&[n(1), Node::Phrase(Phrase::AsPercent)]));
        assert!(is_balanced(&[
            n(1),
            n(2),
            Node::Phrase(Phrase::PercentChange)
        ]));
        assert!(!is_balanced(&[]));
        assert!(!is_balanced(&[Node::Neg]));
        assert!(!is_balanced(&[Node::Bin(BinOp::Add)]));
        assert!(!is_balanced(&[n(1), Node::Bin(BinOp::Add)]));
        assert!(!is_balanced(&[n(1), n(2)]));
        assert!(!is_balanced(&[n(1), n(2), n(3), Node::Bin(BinOp::Add)]));
        assert!(!is_balanced(&[n(1), Node::Phrase(Phrase::PercentChange)]));
        assert!(!is_balanced(&[n(1), n(2), Node::Phrase(Phrase::AsPercent)]));
    }

    #[test]
    #[should_panic(expected = "does not reduce to exactly one value")]
    fn constructing_an_unbalanced_expression_panics() {
        let _ = Expr::from_nodes(vec![n(1), n(2)]);
    }
}
