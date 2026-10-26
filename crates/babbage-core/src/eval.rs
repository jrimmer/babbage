use crate::error::Error;
use crate::number::Number;
use crate::parser::{BinOp, Expr, Node};

pub fn eval(expr: &Expr) -> Result<Number, Error> {
    let mut stack: Vec<Number> = Vec::new();
    for node in expr.nodes() {
        match node {
            Node::Num(n) => stack.push(n.clone()),
            Node::Neg => {
                let x = stack.pop().expect("well-formed expression");
                stack.push(x.neg());
            }
            Node::Bin(op) => {
                let r = stack.pop().expect("well-formed expression");
                let l = stack.pop().expect("well-formed expression");
                stack.push(match op {
                    BinOp::Add => l.add(&r)?,
                    BinOp::Sub => l.sub(&r)?,
                    BinOp::Mul => l.mul(&r)?,
                    BinOp::Div => l.div(&r)?,
                    BinOp::Pow => l.pow(&r)?,
                });
            }
        }
    }
    Ok(stack.pop().expect("well-formed expression"))
}
