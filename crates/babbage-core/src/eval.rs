use crate::error::Error;
use crate::parser::{Expr, Node};
use crate::value::Value;

pub fn eval(expr: &Expr) -> Result<Value, Error> {
    let mut stack: Vec<Value> = Vec::new();
    for node in expr.nodes() {
        match node {
            Node::Num(n) => stack.push(Value::plain(n.clone())),
            Node::Neg => {
                let x = stack.pop().expect("well-formed expression");
                stack.push(x.neg());
            }
            Node::Percent => {
                let x = stack.pop().expect("well-formed expression");
                stack.push(x.into_percent_literal()?);
            }
            Node::Unit(u) => {
                let x = stack.pop().expect("well-formed expression");
                stack.push(x.apply_unit(u)?);
            }
            Node::Convert(u) => {
                let x = stack.pop().expect("well-formed expression");
                stack.push(x.convert(u)?);
            }
            Node::Cast(u) => {
                let x = stack.pop().expect("well-formed expression");
                stack.push(x.cast(u)?);
            }
            Node::Bin(op) => {
                let r = stack.pop().expect("well-formed expression");
                let l = stack.pop().expect("well-formed expression");
                stack.push(Value::binary(*op, &l, &r)?);
            }
            Node::Phrase(p) => {
                let args = stack.split_off(stack.len() - p.arity());
                stack.push(Value::phrase(*p, &args)?);
            }
        }
    }
    Ok(stack.pop().expect("well-formed expression"))
}
