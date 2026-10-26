use crate::error::Error;
use crate::lexer::{Tok, Token};
use crate::parser::{Expr, Node, Phrase, parse};
use crate::units::UnitExpr;

struct Form {
    keywords: &'static [&'static str],
    /// One char per run of tokens around the keywords: 'E' must hold an expression, '.' must be empty.
    shape: &'static str,
    phrase: Phrase,
}

const fn form(keywords: &'static [&'static str], shape: &'static str, phrase: Phrase) -> Form {
    Form {
        keywords,
        shape,
        phrase,
    }
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

const VOCABULARY: &[&str] = &[
    "is", "what", "as", "a", "to", "in", "into", "cast", "at", "of", "on", "off", "percent",
];

/// Words that turn `<value> <word> <unit>` into a conversion.
const CONVERSION_WORDS: &[&str] = &["to", "in", "as", "into"];

fn parse_unit_run(run: &[Token]) -> Result<UnitExpr, Error> {
    if run.is_empty() {
        return Err(Error::UnexpectedEnd);
    }
    let mut unit = UnitExpr::default();
    let mut dividing = false;
    for t in run {
        match &t.tok {
            Tok::Unit(u) => unit = unit.times(&if dividing { u.recip() } else { u.clone() }),
            Tok::Star => {}
            Tok::Slash => dividing = true,
            other => {
                return Err(Error::UnexpectedToken {
                    found: other.describe(),
                    at: t.at,
                });
            }
        }
    }
    Ok(unit)
}

/// The phrase keywords at the top level of a line, as (token index, keyword), with `percent` and a loose `%` as "%".
pub fn keyword_signature(toks: &[Token]) -> Vec<(usize, &'static str)> {
    let mut keywords = Vec::new();
    let mut depth: i64 = 0;
    for (i, t) in toks.iter().enumerate() {
        match &t.tok {
            Tok::LParen => depth += 1,
            Tok::RParen => depth -= 1,
            Tok::Word(w) if depth == 0 => {
                if let Some(k) = VOCABULARY
                    .iter()
                    .copied()
                    .find(|k| w.eq_ignore_ascii_case(k))
                {
                    keywords.push((i, if k == "percent" { "%" } else { k }));
                }
            }
            Tok::Percent { attached: false } if depth == 0 => keywords.push((i, "%")),
            _ => {}
        }
    }
    keywords
}

/// True when the keywords of a line spell a phrase form, a conversion, or a conversion at a rate.
pub fn keywords_form_a_phrase(keywords: &[(usize, &str)]) -> bool {
    let names: Vec<&str> = keywords.iter().map(|(_, k)| *k).collect();
    match names.as_slice() {
        [one] if CONVERSION_WORDS.contains(one) || *one == "cast" => true,
        [conv, "at"] if CONVERSION_WORDS.contains(conv) => true,
        _ => FORMS.iter().any(|f| f.keywords == names.as_slice()),
    }
}

/// Words the arithmetic parser reads as operators when they stand alone.
pub fn is_infix_word(word: &str) -> bool {
    ["of", "on", "off", "and", "or", "xor"]
        .iter()
        .any(|k| word.eq_ignore_ascii_case(k))
}

pub fn is_vocabulary_word(word: &str) -> bool {
    VOCABULARY.iter().any(|k| word.eq_ignore_ascii_case(k))
}

/// Parses one line, recognising the multi-word forms before falling back to plain expressions.
pub fn parse_line(toks: &[Token]) -> Result<Expr, Error> {
    if let Some(parsed) = crate::words::parse_words(toks) {
        return parsed;
    }
    let mut keywords: Vec<(usize, &str)> = Vec::new();
    let mut depth: i64 = 0;
    for (i, t) in toks.iter().enumerate() {
        match &t.tok {
            Tok::LParen => depth += 1,
            Tok::RParen => depth -= 1,
            Tok::Word(w) if depth == 0 && VOCABULARY.contains(&w.to_lowercase().as_str()) => {
                keywords.push((
                    i,
                    VOCABULARY
                        .iter()
                        .copied()
                        .find(|k| w.eq_ignore_ascii_case(k))
                        .map_or("", |k| if k == "percent" { "%" } else { k }),
                ));
            }
            Tok::Percent { attached: false } if depth == 0 => keywords.push((i, "%")),
            _ => {}
        }
    }
    let names: Vec<&str> = keywords.iter().map(|(_, k)| *k).collect();
    if let [(at, word)] = keywords.as_slice() {
        let converts = CONVERSION_WORDS.contains(word);
        let has_both_sides = *at > 0 && *at + 1 < toks.len();
        let target = if (converts || *word == "cast") && has_both_sides {
            parse_unit_run(&toks[at + 1..]).ok()
        } else {
            None
        };
        if let Some(target) = target {
            let mut rpn: Vec<Node> = parse(&toks[..*at])?.nodes().to_vec();
            rpn.push(if converts {
                Node::Convert(target)
            } else {
                Node::Cast(target)
            });
            return Ok(Expr::from_nodes(rpn));
        }
    }
    if let [(c, word), (r, at_word)] = keywords.as_slice()
        && CONVERSION_WORDS.contains(word)
        && *at_word == "at"
        && *c > 0
        && *r > c + 1
        && *r + 1 < toks.len()
    {
        let target = parse_unit_run(&toks[c + 1..*r])?;
        let mut rpn: Vec<Node> = parse(&toks[..*c])?.nodes().to_vec();
        rpn.extend(parse(&toks[r + 1..])?.nodes().iter().cloned());
        rpn.push(Node::ConvertAt(target));
        return Ok(Expr::from_nodes(rpn));
    }
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
    assert_eq!(
        form.shape.len(),
        runs.len(),
        "phrase table entry {:?} has the wrong shape length",
        form.keywords
    );
    let shape_matches = form
        .shape
        .chars()
        .zip(&runs)
        .all(|(want, run)| (want == 'E') != run.is_empty());
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
            assert!(
                f.shape.chars().all(|c| c == 'E' || c == '.'),
                "{:?}",
                f.keywords
            );
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
