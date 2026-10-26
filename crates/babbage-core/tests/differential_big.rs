use babbage_core::{Error, FormatOptions, evaluate, format_decimal};

const CORPUS: &str = include_str!("corpus/big.tsv");

#[test]
fn large_exact_results_match_python_bit_for_bit() {
    let mut opts = FormatOptions::new(10).unwrap();
    opts.group_thousands = false;
    opts.trim_trailing_zeros = false;
    let mut cases = 0;
    for line in CORPUS.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        let got = evaluate(cols[0]);
        if cols[1] == "ERR:overflow" {
            assert_eq!(got, Err(Error::Overflow), "{}", cols[0]);
        } else {
            let n = got.unwrap_or_else(|e| panic!("{}: {e}", cols[0]));
            assert_eq!(n.numer().magnitude().bits().to_string(), cols[1], "numer bits {}", cols[0]);
            assert_eq!(n.denom().magnitude().bits().to_string(), cols[2], "denom bits {}", cols[0]);
            assert_eq!(format_decimal(&n, &opts).text, cols[3], "{}", cols[0]);
        }
        cases += 1;
    }
    assert_eq!(cases, 11);
}
