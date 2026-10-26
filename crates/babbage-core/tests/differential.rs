use babbage_core::{Error, FormatOptions, evaluate_plain as evaluate, format_decimal};

const CORPUS: &str = include_str!("corpus/arith.tsv");

fn fixed10() -> FormatOptions {
    let mut o = FormatOptions::new(10).unwrap();
    o.group_thousands = false;
    o.trim_trailing_zeros = false;
    o
}

#[test]
fn agrees_with_python_fraction_and_decimal_on_every_corpus_line() {
    let mut checked = 0;
    let mut failures = Vec::new();
    for (lineno, line) in CORPUS.lines().enumerate() {
        let cols: Vec<&str> = line.split('\t').collect();
        assert_eq!(cols.len(), 3, "corpus line {} malformed", lineno + 1);
        let (expr, exact, fixed) = (cols[0], cols[1], cols[2]);
        let got = evaluate(expr);
        match exact {
            "ERR:div0" => {
                if got != Err(Error::DivisionByZero) {
                    failures.push(format!("{expr:?}: want div0, got {got:?}"));
                }
            }
            "ERR:zero0" => {
                if got != Err(Error::ZeroToTheZero) {
                    failures.push(format!("{expr:?}: want 0^0, got {got:?}"));
                }
            }
            _ => match got {
                Err(e) => failures.push(format!("{expr:?}: want {exact}, got error {e}")),
                Ok(n) => {
                    let ours = format!("{}/{}", n.numer(), n.denom());
                    let text = format_decimal(&n, &fixed10()).text;
                    if ours != exact || text != fixed {
                        failures.push(format!(
                            "{expr:?}: want {exact} / {fixed}, got {ours} / {text}"
                        ));
                    }
                }
            },
        }
        checked += 1;
    }
    assert!(checked >= 13_000, "corpus unexpectedly small: {checked}");
    assert!(
        failures.is_empty(),
        "{} of {checked} corpus lines disagree, first 15:\n{}",
        failures.len(),
        failures
            .iter()
            .take(15)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
