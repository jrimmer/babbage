//! Whole sheets: variables, line references, subtotals, tags, labels, comments and headings.
//!
//! A sheet is evaluated top to bottom. Everything a line can use comes from the lines above it.

use std::collections::HashMap;

use crate::error::Error;
use crate::eval::eval;
use crate::format::FormatOptions;
use crate::lexer::{Tok, Token, lex_with_ends};
use crate::parser::BinOp;
use crate::phrase;
use crate::units;
use crate::value::{Kind, Value};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SheetOptions {
    /// Ignore words the engine does not know, as the product does, and report each one. Off refuses them.
    pub natural: bool,
    /// A `---` line ends the scope of variables and tags above it.
    pub dividers_reset_scope: bool,
    /// Whether `name = value` lines count in subtotals and the floating total.
    pub declarations_in_totals: bool,
    /// Settings that apply to every line: the named sales tax.
    pub settings: crate::settings::Settings,
}

impl Default for SheetOptions {
    fn default() -> Self {
        SheetOptions {
            natural: true,
            dividers_reset_scope: true,
            declarations_in_totals: true,
            settings: crate::settings::Settings::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum LineKind {
    Blank,
    Heading(String),
    Divider,
    /// A `//` line.
    Comment,
    /// Text with nothing to calculate.
    Text,
    Value(Value),
    Error(Error),
}

/// A word the engine skipped, with its byte offset in the line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ignored {
    pub text: String,
    pub at: usize,
}

/// A skipped word that is one letter away from a unit, so it may be a typo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Warning {
    pub text: String,
    pub at: usize,
    pub looks_like: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LineResult {
    pub kind: LineKind,
    pub label: Option<String>,
    pub tags: Vec<String>,
    /// The variable this line sets.
    pub declares: Option<String>,
    pub ignored: Vec<Ignored>,
    pub warnings: Vec<Warning>,
    pub subtotal: bool,
}

impl LineResult {
    fn new(kind: LineKind) -> LineResult {
        LineResult {
            kind,
            label: None,
            tags: Vec::new(),
            declares: None,
            ignored: Vec::new(),
            warnings: Vec::new(),
            subtotal: false,
        }
    }

    /// The value other lines see: money is rounded to its currency's digits for line references, subtotals and
    /// the floating total, but a declaration (`rent = $10/3`) keeps every digit.
    pub fn counted_value(&self) -> Option<Value> {
        self.value().map(|v| {
            if self.declares.is_none() {
                v.rounded_to_currency()
            } else {
                v.clone()
            }
        })
    }

    pub fn value(&self) -> Option<&Value> {
        match &self.kind {
            LineKind::Value(v) => Some(v),
            _ => None,
        }
    }

    /// Declaration lines show money without rounding; everything else uses `base` as given.
    pub fn display_options(&self, base: &FormatOptions) -> FormatOptions {
        let mut o = base.clone();
        if self.declares.is_some() && !self.subtotal {
            o.fixed_money_digits = false;
        }
        o
    }
}

struct State {
    results: Vec<LineResult>,
    vars: HashMap<String, Value>,
    section_start: usize,
    subtotal_start: usize,
}

pub fn evaluate_sheet(lines: &[&str], opts: &SheetOptions) -> Vec<LineResult> {
    crate::settings::scoped(&opts.settings, || evaluate_sheet_in_scope(lines, opts))
}

fn evaluate_sheet_in_scope(lines: &[&str], opts: &SheetOptions) -> Vec<LineResult> {
    let mut st = State {
        results: Vec::with_capacity(lines.len()),
        vars: HashMap::new(),
        section_start: 0,
        subtotal_start: 0,
    };
    for raw in lines {
        let index = st.results.len();
        let result = evaluate_line(raw, index, &mut st, opts);
        st.results.push(result);
    }
    st.results
}

/// The floating total: every line with a result, apart from subtotal lines.
pub fn floating_total(results: &[LineResult], opts: &SheetOptions) -> Option<Result<Value, Error>> {
    let counted: Vec<Value> = results
        .iter()
        .filter(|r| !r.subtotal && (opts.declarations_in_totals || r.declares.is_none()))
        .filter_map(LineResult::counted_value)
        .collect();
    let values: Vec<&Value> = counted.iter().collect();
    (!values.is_empty()).then(|| add_up(&values))
}

fn normalise_name(name: &str) -> String {
    name.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Sums values, applying any percentages to the running result at the end.
fn add_up(values: &[&Value]) -> Result<Value, Error> {
    let mut total: Option<Value> = None;
    for v in values.iter().filter(|v| v.kind() == Kind::Plain) {
        total = Some(match total {
            None => (*v).clone(),
            Some(t) => Value::binary(BinOp::Add, &t, v)?,
        });
    }
    let Some(mut total) = total else {
        return Err(Error::UnsupportedOperation("there is nothing to add up"));
    };
    for p in values.iter().filter(|v| v.kind() == Kind::Percent) {
        total = Value::binary(BinOp::Add, &total, p)?;
    }
    Ok(total)
}

/// Replaces `from..to` (char indexes) with spaces of the same byte length, so offsets stay meaningful.
fn blank_text(text: &str, ranges: &[(usize, usize)]) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    for (i, c) in chars.iter().enumerate() {
        if ranges.iter().any(|(a, b)| i >= *a && i < *b) {
            out.push_str(&" ".repeat(c.len_utf8()));
        } else {
            out.push(*c);
        }
    }
    out
}

fn evaluate_line(raw: &str, index: usize, st: &mut State, opts: &SheetOptions) -> LineResult {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return LineResult::new(LineKind::Blank);
    }
    if trimmed.starts_with('#') && trimmed.chars().nth(1).is_none_or(|c| c == ' ' || c == '#') {
        st.subtotal_start = index + 1;
        return LineResult::new(LineKind::Heading(
            trimmed.trim_start_matches('#').trim().to_string(),
        ));
    }
    if trimmed.len() >= 3 && trimmed.chars().all(|c| c == '-') {
        st.subtotal_start = index + 1;
        st.section_start = index + 1;
        if opts.dividers_reset_scope {
            st.vars.clear();
        }
        return LineResult::new(LineKind::Divider);
    }

    let chars: Vec<char> = raw.chars().collect();
    let mut blanks: Vec<(usize, usize)> = Vec::new();

    // `//` ends the line's calculation, unless it sits inside quotes.
    let mut in_quote = false;
    let mut quote_start = 0;
    let mut quoted: Vec<(usize, usize)> = Vec::new();
    let mut comment_from = None;
    for (i, c) in chars.iter().enumerate() {
        if *c == '"' {
            if in_quote {
                quoted.push((quote_start, i + 1));
            } else {
                quote_start = i;
            }
            in_quote = !in_quote;
        } else if !in_quote && *c == '/' && chars.get(i + 1) == Some(&'/') {
            comment_from = Some(i);
            break;
        }
    }
    if let Some(from) = comment_from {
        blanks.push((from, chars.len()));
    }
    blanks.extend(
        quoted
            .iter()
            .copied()
            .filter(|(a, _)| comment_from.is_none_or(|c| *a < c)),
    );
    let after_comments = blank_text(raw, &blanks);
    if after_comments.trim().is_empty() {
        return LineResult::new(LineKind::Comment);
    }

    // A label is the text before the first colon, unless the colon is part of a clock time.
    let chars: Vec<char> = after_comments.chars().collect();
    let mut label = None;
    let mut depth = 0i32;
    for (i, c) in chars.iter().enumerate() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ':' if depth == 0 => {
                let prefix: String = chars[..i].iter().collect();
                let next_is_digit = chars.get(i + 1).is_some_and(char::is_ascii_digit);
                if prefix.chars().any(char::is_alphabetic) && !next_is_digit {
                    label = Some(prefix.trim().to_string());
                    blanks.push((0, i + 1));
                }
                break;
            }
            _ => {}
        }
    }
    let after_label = blank_text(raw, &blanks);

    // Tags: `#word` preceded by space or the start of the line.
    let chars: Vec<char> = after_label.chars().collect();
    let mut tags = Vec::new();
    let mut tag_ranges = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '#' && (i == 0 || chars[i - 1].is_whitespace()) {
            let mut j = i + 1;
            while j < chars.len()
                && (chars[j].is_alphanumeric() || chars[j] == '_' || chars[j] == '-')
            {
                j += 1;
            }
            if j > i + 1 {
                tags.push(chars[i + 1..j].iter().collect::<String>().to_lowercase());
                tag_ranges.push((i, j));
            }
            i = j;
        } else {
            i += 1;
        }
    }

    let lowered = after_label.trim().to_lowercase();
    let mut result = LineResult::new(LineKind::Text);
    result.label = label;

    // `total of #food`, `average of #food`.
    if let Some((function, tag)) = aggregate_request(&lowered) {
        let found = st.results[st.section_start..]
            .iter()
            .filter(|r| r.tags.contains(&tag))
            .filter_map(LineResult::counted_value)
            .collect::<Vec<_>>();
        let found: Vec<&Value> = found.iter().collect();
        result.kind = match aggregate(function, &found) {
            Ok(v) => LineKind::Value(v),
            Err(e) => LineKind::Error(e),
        };
        return result;
    }

    blanks.extend(tag_ranges);
    let text = blank_text(raw, &blanks);
    result.tags = tags;

    match split_conditional(&text) {
        Some(c) => run_conditional(&c, index, st, opts, &mut result),
        None => run_statement(&text, index, st, opts, &mut result),
    }
    result
}

/// `if C then A else B`, `A if C` and `A unless C`.
struct Conditional {
    condition: String,
    then_branch: String,
    else_branch: Option<String>,
    /// `unless`: run the first branch when the condition is false.
    negate: bool,
    /// The prefix form must have a true or false condition; the suffix form falls back to plain evaluation.
    strict: bool,
}

/// Byte offsets of the standalone, top-level words in `text` equal to `word` (any case).
fn top_level_words(text: &str, word: &str) -> Vec<usize> {
    let mut found = Vec::new();
    let mut depth = 0i64;
    let mut start: Option<usize> = None;
    for (i, c) in text
        .char_indices()
        .chain(std::iter::once((text.len(), ' ')))
    {
        if c.is_alphabetic() {
            start.get_or_insert(i);
            continue;
        }
        if let Some(from) = start.take()
            && depth == 0
            && text[from..i].eq_ignore_ascii_case(word)
        {
            found.push(from);
        }
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
    }
    found
}

fn split_conditional(text: &str) -> Option<Conditional> {
    let trimmed_start = text.len() - text.trim_start().len();
    if let Some(&0) = top_level_words(text.trim_start(), "if").first() {
        let rest = &text[trimmed_start + 2..];
        let then = *top_level_words(rest, "then").first()?;
        let condition = rest[..then].to_string();
        let after = &rest[then + 4..];
        let (then_branch, else_branch) = match top_level_words(after, "else").first() {
            Some(&e) => (after[..e].to_string(), Some(after[e + 4..].to_string())),
            None => (after.to_string(), None),
        };
        return Some(Conditional {
            condition,
            then_branch,
            else_branch,
            negate: false,
            strict: true,
        });
    }
    let last = |word: &str| top_level_words(text, word).last().copied();
    let (at, len, negate) = match (last("if"), last("unless")) {
        (Some(i), Some(u)) if u > i => (u, 6, true),
        (Some(i), _) => (i, 2, false),
        (None, Some(u)) => (u, 6, true),
        (None, None) => return None,
    };
    if text[..at].trim().is_empty() {
        return None;
    }
    Some(Conditional {
        condition: text[at + len..].to_string(),
        then_branch: text[..at].to_string(),
        else_branch: None,
        negate,
        strict: false,
    })
}

fn run_conditional(
    c: &Conditional,
    index: usize,
    st: &mut State,
    opts: &SheetOptions,
    result: &mut LineResult,
) {
    let condition = evaluate_expression(&c.condition, index, st, opts);
    let truth = match &condition.outcome {
        Ok(v) => v.as_bool(),
        Err(_) => None,
    };
    let Some(truth) = truth else {
        if c.strict {
            result.kind = LineKind::Error(match condition.outcome {
                Err(e) => e,
                Ok(_) => Error::UnsupportedOperation("a condition must come out true or false"),
            });
        } else {
            run_statement(
                &format!("{} if {}", c.then_branch, c.condition),
                index,
                st,
                opts,
                result,
            );
        }
        return;
    };
    let branch = if truth != c.negate {
        Some(&c.then_branch)
    } else {
        c.else_branch.as_ref()
    };
    match branch {
        Some(text) => run_statement(text, index, st, opts, result),
        None => result.kind = LineKind::Text,
    }
}

/// One statement: `name = value`, `name += value`, `name =` after a list of lines, or a plain expression.
fn run_statement(
    text: &str,
    index: usize,
    st: &mut State,
    opts: &SheetOptions,
    result: &mut LineResult,
) {
    let lowered = text.trim().to_lowercase();
    let (target, op, rhs_text) = split_assignment(text);
    if lowered == "subtotal" || (target.is_some() && rhs_text.trim().is_empty() && op.is_none()) {
        result.subtotal = true;
        let counted: Vec<Value> = st.results[st.subtotal_start.min(index)..]
            .iter()
            .filter(|r| !r.subtotal && (opts.declarations_in_totals || r.declares.is_none()))
            .filter_map(LineResult::counted_value)
            .collect();
        let range: Vec<&Value> = counted.iter().collect();
        st.subtotal_start = index + 1;
        result.kind = match add_up(&range) {
            Ok(v) => {
                if let Some(name) = &target {
                    st.vars.insert(normalise_name(name), v.clone());
                    result.declares = Some(normalise_name(name));
                }
                LineKind::Value(v)
            }
            Err(e) => LineKind::Error(e),
        };
        return;
    }

    let evaluated = evaluate_expression(&rhs_text, index, st, opts);
    result.ignored = evaluated.ignored;
    result.warnings = evaluated.warnings;
    match evaluated.outcome {
        Err(Error::Empty) => {}
        Err(e) => result.kind = LineKind::Error(e),
        Ok(mut value) => {
            if let Some(name) = &target {
                let key = normalise_name(name);
                if let Some(BinOp::Add | BinOp::Sub) = op {
                    let old = lookup_variable(&st.vars, &key).cloned();
                    value = match old {
                        Some(old) => match Value::binary(op.unwrap_or(BinOp::Add), &old, &value) {
                            Ok(v) => v,
                            Err(e) => {
                                result.kind = LineKind::Error(e);
                                return;
                            }
                        },
                        None => {
                            result.kind = LineKind::Error(Error::UnsupportedOperation(
                                "this variable has no value yet",
                            ));
                            return;
                        }
                    };
                }
                st.vars.insert(key.clone(), value.clone());
                result.declares = Some(key);
            }
            result.kind = LineKind::Value(value);
        }
    }
}

#[derive(Clone, Copy)]
enum Aggregate {
    Total,
    Average,
    Count,
}

fn aggregate_request(lowered: &str) -> Option<(Aggregate, String)> {
    let (function, rest) = lowered.split_once(char::is_whitespace)?;
    let function = match function {
        "total" | "sum" => Aggregate::Total,
        "average" | "avg" => Aggregate::Average,
        "count" => Aggregate::Count,
        _ => return None,
    };
    let rest = rest.trim().strip_prefix("of")?.trim();
    let tag = rest.strip_prefix('#')?;
    (!tag.is_empty()
        && tag
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-'))
    .then(|| (function, tag.to_string()))
}

fn aggregate(function: Aggregate, found: &[&Value]) -> Result<Value, Error> {
    match function {
        Aggregate::Total => add_up(found),
        Aggregate::Count => Ok(Value::plain(crate::number::Number::from_i64(
            found.len() as i64
        ))),
        Aggregate::Average => {
            let sum = add_up(found)?;
            Value::binary(
                BinOp::Div,
                &sum,
                &Value::plain(crate::number::Number::from_i64(found.len() as i64)),
            )
        }
    }
}

/// Splits `name = value`, `name += value` and `name -= value`. Anything else has no target.
fn split_assignment(text: &str) -> (Option<String>, Option<BinOp>, String) {
    let Some(eq) = text.find('=') else {
        return (None, None, text.to_string());
    };
    if text[eq + 1..].starts_with('=') || text[..eq].ends_with(['=', '<', '>', '!']) {
        return (None, None, text.to_string());
    }
    let (mut name, mut op) = (&text[..eq], None);
    if let Some(stripped) = name.trim_end().strip_suffix('+') {
        (name, op) = (stripped, Some(BinOp::Add));
    } else if let Some(stripped) = name.trim_end().strip_suffix('-') {
        (name, op) = (stripped, Some(BinOp::Sub));
    }
    let name = name.trim();
    let valid = name.chars().next().is_some_and(char::is_alphabetic)
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == ' ' || c == '_');
    if !valid {
        return (None, None, text.to_string());
    }
    (
        Some(name.to_string()),
        op,
        " ".repeat(eq + 1) + &text[eq + 1..],
    )
}

fn lookup_variable<'a>(vars: &'a HashMap<String, Value>, name: &str) -> Option<&'a Value> {
    if let Some(v) = vars.get(name) {
        return Some(v);
    }
    let mut hits = vars.iter().filter(|(k, _)| k.eq_ignore_ascii_case(name));
    match (hits.next(), hits.next()) {
        (Some((_, v)), None) => Some(v),
        _ => None,
    }
}

struct Evaluated {
    outcome: Result<Value, Error>,
    ignored: Vec<Ignored>,
    warnings: Vec<Warning>,
}

fn evaluate_expression(text: &str, index: usize, st: &State, opts: &SheetOptions) -> Evaluated {
    let mut ignored = Vec::new();
    let mut warnings = Vec::new();
    let outcome = (|| {
        let (toks, ends) = lex_with_ends(text)?;
        let toks = substitute(text, toks, &ends, index, st)?;
        let is_word_form = crate::words::parse_words(&toks).is_some();
        let toks = if opts.natural && !is_word_form {
            drop_unknown(text, toks, &mut ignored, &mut warnings)?
        } else {
            toks
        };
        eval(&phrase::parse_line(&toks)?)
    })();
    Evaluated {
        outcome,
        ignored,
        warnings,
    }
}

fn is_operand_start(t: &Tok) -> bool {
    matches!(
        t,
        Tok::Num(_) | Tok::Value(_) | Tok::LParen | Tok::Unit(_) | Tok::Word(_)
    )
}

fn is_operand_end(t: &Tok) -> bool {
    matches!(
        t,
        Tok::Num(_)
            | Tok::Value(_)
            | Tok::Group(_)
            | Tok::RParen
            | Tok::Unit(_)
            | Tok::Percent { .. }
    )
}

/// Replaces `line3` and variable names with their values.
fn substitute(
    text: &str,
    toks: Vec<Token>,
    ends: &[usize],
    index: usize,
    st: &State,
) -> Result<Vec<Token>, Error> {
    let max_words = st
        .vars
        .keys()
        .map(|k| k.split(' ').count())
        .max()
        .unwrap_or(0);
    let mut out = Vec::with_capacity(toks.len());
    let mut i = 0;
    'next: while i < toks.len() {
        let start = toks[i].at;
        if let Tok::Word(w) = &toks[i].tok
            && w.eq_ignore_ascii_case("line")
            && lookup_variable(&st.vars, "line").is_none()
            && let Some(Token {
                tok: Tok::Num(n), ..
            }) = toks.get(i + 1)
            && n.is_integer()
            && !n.is_negative()
        {
            let wanted = usize::try_from(n.numer().clone()).unwrap_or(usize::MAX);
            let value = wanted
                .checked_sub(1)
                .filter(|n| *n < index)
                .and_then(|n| st.results[n].counted_value())
                .ok_or(Error::BadReference { line: wanted })?;
            out.push(Token {
                tok: Tok::Value(value),
                at: start,
            });
            i += 2;
            continue 'next;
        }
        // A lone `x` is the times sign between two operands, and a variable name anywhere else.
        let is_x = matches!(toks[i].tok, Tok::Star)
            && text[start..ends[i]].chars().all(char::is_alphabetic);
        let next_starts_operand = toks.get(i + 1).is_some_and(|n| {
            is_operand_start(&n.tok)
                || (matches!(n.tok, Tok::Star)
                    && text[n.at..ends[i + 1]].chars().all(char::is_alphabetic))
        });
        let between_operands =
            out.last().is_some_and(|p: &Token| is_operand_end(&p.tok)) && next_starts_operand;
        let times_sign_used_as_name = is_x && !between_operands;
        if matches!(toks[i].tok, Tok::Word(_) | Tok::Unit(_)) || times_sign_used_as_name {
            for span in (1..=max_words.min(toks.len() - i)).rev() {
                let name = normalise_name(&text[start..ends[i + span - 1]]);
                if let Some(v) = lookup_variable(&st.vars, &name) {
                    out.push(Token {
                        tok: Tok::Value(v.clone()),
                        at: start,
                    });
                    i += span;
                    continue 'next;
                }
            }
        }
        out.push(toks[i].clone());
        i += 1;
    }
    Ok(out)
}

/// Natural-language mode: drop parenthesised remarks, unknown words and stray keywords, and say what was dropped.
///
/// Dropping is only safe around a calculation. A dropped stretch that fills the place of a number
/// (`y + 1` with `y` undefined, `(y) + 1`) is refused, so a mistyped name never turns into a quiet wrong answer.
fn drop_unknown(
    text: &str,
    toks: Vec<Token>,
    ignored: &mut Vec<Ignored>,
    warnings: &mut Vec<Warning>,
) -> Result<Vec<Token>, Error> {
    // A function name is never prose: dropping `sqrt` from `sqrt(16)` would answer 16.
    let unknown = |t: &Token| matches!(&t.tok, Tok::Word(w) if !phrase::is_vocabulary_word(w) && !phrase::is_infix_word(w) && crate::functions::lookup(w).is_none());
    let mut drop = vec![false; toks.len()];

    // A parenthesised group holding a word is a remark: `$999 (for iPhone 16)`.
    let mut open: Vec<usize> = Vec::new();
    for (i, t) in toks.iter().enumerate() {
        match t.tok {
            Tok::LParen => open.push(i),
            Tok::RParen => {
                if let Some(from) = open.pop()
                    && toks[from..=i].iter().any(unknown)
                {
                    drop[from..=i].fill(true);
                }
            }
            _ => {}
        }
    }
    for (i, t) in toks.iter().enumerate() {
        // `as timespan` and `in laptime` name a layout, not a stray word.
        let layout_word = matches!(&t.tok, Tok::Word(w) if w.eq_ignore_ascii_case("timespan") || w.eq_ignore_ascii_case("laptime") || w.eq_ignore_ascii_case("fraction"))
            && i > 0
            && matches!(&toks[i - 1].tok, Tok::Word(p) if ["as", "in", "to", "into"].iter().any(|k| p.eq_ignore_ascii_case(k)));
        if unknown(t) && !layout_word {
            drop[i] = true;
        }
    }

    // Keywords that do not spell a phrase are prose (`Boing is $5`); `of`, `on`, `off` stay as operators.
    let kept_at: Vec<usize> = (0..toks.len()).filter(|i| !drop[*i]).collect();
    let kept: Vec<Token> = kept_at.iter().map(|i| toks[*i].clone()).collect();
    let signature = phrase::keyword_signature(&kept);
    if !signature.is_empty() && !phrase::keywords_form_a_phrase(&signature) {
        for (k, name) in &signature {
            let is_operator_word =
                matches!(&kept[*k].tok, Tok::Word(w) if phrase::is_infix_word(w));
            if *name != "%" && !is_operator_word {
                drop[kept_at[*k]] = true;
            }
        }
    }

    // An operator or conversion word with only prose after it is prose: the `on` of `$45 on clothes` and the
    // `in` of `40 lb in watermelons`.
    if let Some(last_kept) = (0..toks.len()).rev().find(|i| !drop[*i])
        && last_kept + 1 < toks.len()
        && matches!(&toks[last_kept].tok, Tok::Word(w) if phrase::is_infix_word(w) || phrase::is_conversion_word(w))
        && last_kept > 0
    {
        drop[last_kept] = true;
    }

    refuse_a_dropped_stretch_that_stands_in_for_a_value(&toks, &drop)?;

    for (t, d) in toks.iter().zip(&drop) {
        if !*d {
            continue;
        }
        match &t.tok {
            Tok::Word(w) => {
                ignored.push(Ignored {
                    text: w.clone(),
                    at: t.at,
                });
                if let Some(like) = typo_of_a_unit(w) {
                    warnings.push(Warning {
                        text: w.clone(),
                        at: t.at,
                        looks_like: like,
                    });
                }
            }
            _ => {
                // The token runs up to the next one that starts later (or the end of the line), less spaces.
                let next = toks
                    .iter()
                    .map(|n| n.at)
                    .filter(|at| *at > t.at)
                    .min()
                    .unwrap_or(text.len());
                let shown = text[t.at..next].trim_end();
                let shown = if shown.is_empty() {
                    &text[t.at..t.at + text[t.at..].chars().next().map_or(0, char::len_utf8)]
                } else {
                    shown
                };
                ignored.push(Ignored {
                    text: shown.to_string(),
                    at: t.at,
                });
            }
        }
    }
    ignored.sort_by_key(|i| i.at);
    Ok(toks
        .into_iter()
        .zip(drop)
        .filter(|(_, d)| !*d)
        .map(|(t, _)| t)
        .collect())
}

fn is_operator(t: &Token) -> bool {
    matches!(
        t.tok,
        Tok::Plus | Tok::Minus | Tok::Star | Tok::Slash | Tok::Caret
    ) || matches!(&t.tok, Tok::Word(w) if phrase::is_infix_word(w))
}

fn refuse_a_dropped_stretch_that_stands_in_for_a_value(
    toks: &[Token],
    drop: &[bool],
) -> Result<(), Error> {
    if drop.iter().all(|d| *d) {
        return Ok(());
    }
    let mut i = 0;
    while i < toks.len() {
        if !drop[i] {
            i += 1;
            continue;
        }
        let end = (i..toks.len()).find(|j| !drop[*j]).unwrap_or(toks.len());
        let before = (0..i).rev().find(|j| !drop[*j]).map(|j| &toks[j]);
        let after = toks.get(end);
        let wants_value = before.is_none_or(|b| is_operator(b) || matches!(b.tok, Tok::LParen));
        let value_missing = after.is_none_or(|a| is_operator(a) || matches!(a.tok, Tok::RParen));
        if wants_value && value_missing {
            let word = toks[i..end].iter().find_map(|t| match &t.tok {
                Tok::Word(w) => Some(w.clone()),
                _ => None,
            });
            return Err(Error::UnexpectedWord {
                word: word.unwrap_or_else(|| "(".into()),
                at: toks[i].at,
            });
        }
        i = end;
    }
    Ok(())
}

/// A unit name one edit away from `word`, which shares its first letter.
fn typo_of_a_unit(word: &str) -> Option<String> {
    if word.chars().count() < 3 {
        return None;
    }
    let first = word.chars().next()?;
    let mut hits: Vec<&String> = units::registry()
        .names()
        .map(|(name, _)| name)
        .filter(|name| {
            name.chars().count() >= 2 && name.starts_with(first) && within_one_edit(word, name)
        })
        .collect();
    hits.sort();
    hits.first().map(|s| (*s).clone())
}

fn within_one_edit(a: &str, b: &str) -> bool {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if a == b || a.len().abs_diff(b.len()) > 1 {
        return false;
    }
    if a.len() == b.len() {
        let diffs: Vec<usize> = (0..a.len()).filter(|i| a[*i] != b[*i]).collect();
        return diffs.len() == 1
            || (diffs.len() == 2
                && diffs[1] == diffs[0] + 1
                && a[diffs[0]] == b[diffs[1]]
                && a[diffs[1]] == b[diffs[0]]);
    }
    let (long, short) = if a.len() > b.len() {
        (&a, &b)
    } else {
        (&b, &a)
    };
    (0..long.len()).any(|skip| {
        long.iter()
            .enumerate()
            .filter(|(i, _)| *i != skip)
            .map(|(_, c)| c)
            .eq(short.iter())
    })
}
