use std::fmt;

use crate::error::Error;
use crate::lexer::{Tok, Token};
use crate::number::Number;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
}

impl BinOp {
    fn precedence(self) -> u8 {
        match self {
            BinOp::Add | BinOp::Sub => 1,
            BinOp::Mul | BinOp::Div => 2,
            BinOp::Pow => 4,
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
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Num(Number),
    Neg,
    Bin(BinOp),
}

/// A well-formed expression in reverse Polish order. Parsing, evaluation and
/// printing are all iterative, so nesting depth cannot overflow the stack.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    rpn: Vec<Node>,
}

impl Expr {
    pub fn num(n: Number) -> Expr {
        Expr { rpn: vec![Node::Num(n)] }
    }

    pub fn neg(mut self) -> Expr {
        self.rpn.push(Node::Neg);
        self
    }

    pub fn bin(op: BinOp, mut lhs: Expr, rhs: Expr) -> Expr {
        lhs.rpn.extend(rhs.rpn);
        lhs.rpn.push(Node::Bin(op));
        lhs
    }

    pub fn nodes(&self) -> &[Node] {
        &self.rpn
    }
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

pub fn parse(toks: &[Token]) -> Result<Expr, Error> {
    if toks.is_empty() {
        return Err(Error::Empty);
    }
    let mut out: Vec<Node> = Vec::with_capacity(toks.len());
    let mut ops: Vec<Pending> = Vec::new();
    let mut want_operand = true;

    for t in toks {
        match (&t.tok, want_operand) {
            (Tok::Num(n), true) => {
                out.push(Node::Num(n.clone()));
                want_operand = false;
            }
            (Tok::LParen, true) => ops.push(Pending::LParen),
            (Tok::Minus, true) => ops.push(Pending::Neg),
            (Tok::Plus, true) => {}
            (Tok::Plus | Tok::Minus | Tok::Star | Tok::Slash | Tok::Caret, false) => {
                let op = match t.tok {
                    Tok::Plus => BinOp::Add,
                    Tok::Minus => BinOp::Sub,
                    Tok::Star => BinOp::Mul,
                    Tok::Slash => BinOp::Div,
                    _ => BinOp::Pow,
                };
                while let Some(top) = ops.last() {
                    let (tp, p) = (top.precedence(), op.precedence());
                    if tp > p || (tp == p && !op.right_associative()) {
                        ops.pop().expect("last() was Some").emit(&mut out);
                    } else {
                        break;
                    }
                }
                ops.push(Pending::Bin(op));
                want_operand = true;
            }
            (Tok::RParen, false) => loop {
                match ops.pop() {
                    Some(Pending::LParen) => break,
                    Some(p) => p.emit(&mut out),
                    None => return Err(Error::UnexpectedToken { found: t.tok.describe(), at: t.at }),
                }
            },
            (tok, _) => return Err(Error::UnexpectedToken { found: tok.describe(), at: t.at }),
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
