use crate::error::Error;
use crate::lexer::{Tok, Token};
use crate::parser::{Expr, Node, Phrase, parse};

struct Form {
    keywords: &'static [&'static str],
    /// One char per run of tokens around the keywords: 'E' must hold an expression, '.' must be empty.
    shape: &'static str,
    phrase: Phrase,
}

const fn form(keywords: &'static [&'static str], shape: &'static str, phrase: Phrase) -> Form {
    Form { keywords, shape, phrase }
}

const FORMS: &[Form] = &[
    form(&["is", "of", "what"], "EE..", Phrase::IsPercentOfWhat),
    form(&["is", "on", "what"], "EE..", Phrase::IsPercentOnWhat),
    form(&["is", "off", "what"], "EE..", Phrase::IsPercentOffWhat),
    form(&["is", "what", "%", "of"], "E...E", Phrase::WhatPercentOf),
    form(&["is", "what", "%", "on"], "E...E", Phrase::WhatPercentOn),
    form(&["is", "what", "%", "off"], "E...E", Phrase::WhatPercentOff),
    form(&["as", "a", "%", "of"], "E...E", Phrase::WhatPercentOf),
    form(&["as", "%", "of"], "E..E", Phrase::WhatPercentOf),
    form(&["in", "is", "what", "%"], "EE...", Phrase::WhatPercentOf),
    form(&["to", "is", "what", "%"], "EE...", Phrase::PercentChange),
    form(&["to", "as", "%"], "EE..", Phrase::PercentChange),
    form(&["is", "what", "%"], "E...", Phrase::AsPercent),
    form(&["as", "%"], "E..", Phrase::AsPercent),
    form(&["%"], "E.", Phrase::AsPercent),
];

const VOCABULARY: &[&str] = &["is", "what", "as", "a", "to", "in", "of", "on", "off", "percent"];

/// Parses one line, recognising the multi-word forms before falling back to plain expressions.
pub fn parse_line(toks: &[Token]) -> Result<Expr, Error> {
    let mut keywords: Vec<(usize, &str)> = Vec::new();
    let mut depth: i64 = 0;
    for (i, t) in toks.iter().enumerate() {
        match &t.tok {
            Tok::LParen => depth += 1,
            Tok::RParen => depth -= 1,
            Tok::Word(w) if depth == 0 && VOCABULARY.contains(&w.as_str()) => {
                keywords.push((i, if w == "percent" { "%" } else { w.as_str() }));
            }
            Tok::Percent { attached: false } if depth == 0 => keywords.push((i, "%")),
            _ => {}
        }
    }
    let names: Vec<&str> = keywords.iter().map(|(_, k)| *k).collect();
    let Some(form) = FORMS.iter().find(|f| f.keywords == names.as_slice()) else {
        return parse(toks);
    };

    let mut bounds = vec![0];
    for (i, _) in &keywords {
        bounds.push(*i);
        bounds.push(i + 1);
    }
    bounds.push(toks.len());
    let runs: Vec<&[Token]> = bounds.chunks(2).map(|w| &toks[w[0]..w[1]]).collect();
    assert_eq!(form.shape.len(), runs.len(), "phrase table entry {:?} has the wrong shape length", form.keywords);
    let shape_matches = form.shape.chars().zip(&runs).all(|(want, run)| (want == 'E') != run.is_empty());
    if !shape_matches {
        return parse(toks);
    }

    let mut rpn: Vec<Node> = Vec::new();
    for run in runs.iter().filter(|r| !r.is_empty()) {
        rpn.extend(parse(run)?.nodes().iter().cloned());
    }
    rpn.push(Node::Phrase(form.phrase));
    Ok(Expr::from_nodes(rpn))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shape_has_one_slot_per_run_around_its_keywords() {
        for f in FORMS {
            assert_eq!(f.shape.len(), f.keywords.len() + 1, "{:?}", f.keywords);
            let expressions = f.shape.chars().filter(|c| *c == 'E').count();
            assert_eq!(expressions, f.phrase.arity(), "{:?}", f.keywords);
            assert!(f.shape.chars().all(|c| c == 'E' || c == '.'), "{:?}", f.keywords);
        }
    }

    #[test]
    fn no_two_forms_share_the_same_keywords() {
        for (i, a) in FORMS.iter().enumerate() {
            for b in &FORMS[i + 1..] {
                assert_ne!(a.keywords, b.keywords);
            }
        }
    }
}
