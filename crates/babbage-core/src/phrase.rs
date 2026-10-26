use crate::error::Error;
use crate::lexer::{Tok, Token};
use crate::parser::{Expr, Node, Phrase, parse};
use crate::units::UnitExpr;
use crate::value::Shown;

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

pub(crate) fn parse_unit_run(run: &[Token]) -> Result<UnitExpr, Error> {
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

/// What follows `as` or `in` when it names a layout (`timespan`, `laptime`) or several units (`hours and minutes`).
fn parse_shown(run: &[Token]) -> Option<Shown> {
    if let [
        Token {
            tok: Tok::Word(w), ..
        },
    ] = run
    {
        if w.eq_ignore_ascii_case("timespan") {
            return Some(Shown::Timespan);
        }
        if w.eq_ignore_ascii_case("laptime") {
            return Some(Shown::Laptime);
        }
        if w.eq_ignore_ascii_case("fraction") {
            return Some(Shown::Fraction);
        }
    }
    let joined = |t: &Token| matches!(&t.tok, Tok::Word(w) if w.eq_ignore_ascii_case("and"));
    let parts: Vec<&[Token]> = run.split(joined).collect();
    if parts.len() < 2 {
        return None;
    }
    parts
        .iter()
        .map(|p| parse_unit_run(p).ok())
        .collect::<Option<Vec<_>>>()
        .map(Shown::Parts)
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
    ["of", "on", "off", "and", "or", "xor", "per", "over", "from"]
        .iter()
        .any(|k| word.eq_ignore_ascii_case(k))
}

/// `in`, `to`, `as` and `into`: the words that start a conversion.
pub fn is_conversion_word(word: &str) -> bool {
    CONVERSION_WORDS
        .iter()
        .any(|k| word.eq_ignore_ascii_case(k))
}

pub fn is_vocabulary_word(word: &str) -> bool {
    VOCABULARY.iter().any(|k| word.eq_ignore_ascii_case(k))
}

/// What one open parenthesis has seen at its own level so far: where it opened in the output, its phrase
/// keywords and its commas (token indices into the output).
struct Level {
    open: usize,
    keywords: Vec<(usize, &'static str)>,
    commas: Vec<usize>,
}

/// Replaces each comma-separated run inside a pair of parentheses that spells a phrase form with one
/// `Tok::Group` holding its parsed expression, innermost pairs first: `(20 as %) x 2` and
/// `max(20 as %, 5)`. One pass over the tokens with an explicit stack, so deep nesting cannot overflow.
fn collapse_groups(toks: &[Token]) -> Result<Vec<Token>, Error> {
    let mut out: Vec<Token> = Vec::with_capacity(toks.len());
    let mut open: Vec<Level> = Vec::new();
    for t in toks {
        match &t.tok {
            Tok::LParen => open.push(Level {
                open: out.len(),
                keywords: Vec::new(),
                commas: Vec::new(),
            }),
            Tok::Comma => {
                if let Some(level) = open.last_mut() {
                    level.commas.push(out.len());
                }
            }
            Tok::Word(w) => {
                if let Some(level) = open.last_mut()
                    && let Some(k) = VOCABULARY
                        .iter()
                        .copied()
                        .find(|k| w.eq_ignore_ascii_case(k))
                {
                    level
                        .keywords
                        .push((out.len(), if k == "percent" { "%" } else { k }));
                }
            }
            Tok::Percent { attached: false } => {
                if let Some(level) = open.last_mut() {
                    level.keywords.push((out.len(), "%"));
                }
            }
            Tok::RParen => {
                if let Some(level) = open.pop() {
                    let mut starts = vec![level.open + 1];
                    starts.extend(level.commas.iter().map(|c| c + 1));
                    let mut ends = level.commas.clone();
                    ends.push(out.len());
                    for (&from, &to) in starts.iter().zip(&ends).rev() {
                        let names: Vec<(usize, &str)> = level
                            .keywords
                            .iter()
                            .copied()
                            .filter(|(i, _)| (from..to).contains(i))
                            .collect();
                        if keywords_form_a_phrase(&names) {
                            let expr = parse_line(&out[from..to])?;
                            let at = out[from].at;
                            out.splice(
                                from..to,
                                [Token {
                                    tok: Tok::Group(expr),
                                    at,
                                }],
                            );
                        }
                    }
                }
            }
            _ => {}
        }
        out.push(t.clone());
    }
    Ok(out)
}

/// Parses one line, recognising the multi-word forms before falling back to plain expressions.
pub fn parse_line(toks: &[Token]) -> Result<Expr, Error> {
    let grouped;
    let toks = if toks.iter().any(|t| t.tok == Tok::LParen) {
        grouped = collapse_groups(toks)?;
        &grouped[..]
    } else {
        toks
    };
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
        if converts
            && has_both_sides
            && let Some(shown) = parse_shown(&toks[at + 1..])
        {
            let mut rpn: Vec<Node> = parse(&toks[..*at])?.nodes().to_vec();
            rpn.push(Node::Show(shown));
            return Ok(Expr::from_nodes(rpn));
        }
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
