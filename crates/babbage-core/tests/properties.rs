use babbage_core::parser::{BinOp, Expr};
use babbage_core::{
    Error, FormatOptions, Number, RoundingMode, eval::eval, evaluate_plain as evaluate,
    format_decimal, parse_expression,
};
use num_bigint::BigInt;
use num_traits::{One, Signed};
use proptest::prelude::*;

fn cfg() -> ProptestConfig {
    ProptestConfig {
        cases: 3000,
        max_shrink_iters: 4096,
        ..ProptestConfig::default()
    }
}

fn number() -> impl Strategy<Value = Number> {
    (
        any::<i64>(),
        prop_oneof![1i64..=1_000_000_007, -1_000_000_007i64..=-1],
    )
        .prop_map(|(n, d)| Number::from_ratio(BigInt::from(n), BigInt::from(d)).unwrap())
}

fn small_number() -> impl Strategy<Value = Number> {
    (-1000i64..=1000, prop_oneof![1i64..=60, -60i64..=-1])
        .prop_map(|(n, d)| Number::from_ratio(BigInt::from(n), BigInt::from(d)).unwrap())
}

fn nonzero() -> impl Strategy<Value = Number> {
    number().prop_filter("nonzero", |n| !n.is_zero())
}

fn zero() -> Number {
    Number::from_i64(0)
}

fn one() -> Number {
    Number::from_i64(1)
}

proptest! {
    #![proptest_config(cfg())]

    #[test]
    fn addition_is_commutative_and_associative(a in number(), b in number(), c in number()) {
        prop_assert_eq!(a.add(&b)?, b.add(&a)?);
        prop_assert_eq!(a.add(&b)?.add(&c)?, a.add(&b.add(&c)?)?);
    }

    #[test]
    fn multiplication_is_commutative_and_associative(a in number(), b in number(), c in number()) {
        prop_assert_eq!(a.mul(&b)?, b.mul(&a)?);
        prop_assert_eq!(a.mul(&b)?.mul(&c)?, a.mul(&b.mul(&c)?)?);
    }

    #[test]
    fn multiplication_distributes_over_addition_exactly(a in number(), b in number(), c in number()) {
        prop_assert_eq!(a.mul(&b.add(&c)?)?, a.mul(&b)?.add(&a.mul(&c)?)?);
    }

    #[test]
    fn identities_and_inverses(a in number()) {
        prop_assert_eq!(a.add(&zero())?, a.clone());
        prop_assert_eq!(a.mul(&one())?, a.clone());
        prop_assert_eq!(a.mul(&zero())?, zero());
        prop_assert_eq!(a.sub(&a)?, zero());
        prop_assert_eq!(a.add(&a.neg())?, zero());
        prop_assert_eq!(a.neg().neg(), a.clone());
        prop_assert_eq!(a.abs().is_negative(), false);
        prop_assert_eq!(a.sub(&zero())?, a.clone());
        prop_assert_eq!(zero().sub(&a)?, a.neg());
    }

    #[test]
    fn division_inverts_multiplication_exactly(a in number(), b in nonzero()) {
        prop_assert_eq!(a.div(&b)?.mul(&b)?, a.clone());
        prop_assert_eq!(a.mul(&b)?.div(&b)?, a.clone());
        prop_assert_eq!(b.div(&b)?, one());
        prop_assert_eq!(a.div(&b)?.mul(&b.div(&a).unwrap_or_else(|_| zero()))?.is_zero(), a.is_zero());
    }

    #[test]
    fn division_by_zero_is_always_an_error(a in number()) {
        prop_assert_eq!(a.div(&zero()), Err(Error::DivisionByZero));
    }

    #[test]
    fn ordering_is_translation_invariant(a in number(), b in number(), c in number()) {
        prop_assert_eq!(a.cmp(&b), a.add(&c)?.cmp(&b.add(&c)?));
        let pos = c.abs().add(&one())?;
        prop_assert_eq!(a.cmp(&b), a.mul(&pos)?.cmp(&b.mul(&pos)?));
        prop_assert_eq!(a.cmp(&b), b.neg().cmp(&a.neg()));
    }

    #[test]
    fn denominators_are_always_positive_and_reduced(n in any::<i64>(), d in prop_oneof![1i64..=i64::MAX, i64::MIN..=-1]) {
        let x = Number::from_ratio(BigInt::from(n), BigInt::from(d)).unwrap();
        prop_assert!(x.denom().is_positive());
        let g = num_integer::Integer::gcd(x.numer(), x.denom());
        prop_assert!(g.is_one());
        let y = Number::from_ratio(BigInt::from(-n), BigInt::from(-d)).unwrap();
        prop_assert_eq!(x, y);
    }

    #[test]
    fn integer_powers_obey_the_laws(a in small_number().prop_filter("nonzero", |n| !n.is_zero()), m in -8i64..=8, n in -8i64..=8) {
        let (pm, pn) = (Number::from_i64(m), Number::from_i64(n));
        let sum = Number::from_i64(m + n);
        prop_assert_eq!(a.pow(&pm)?.mul(&a.pow(&pn)?)?, a.pow(&sum)?);
        prop_assert_eq!(a.pow(&pm)?.pow(&pn)?, a.pow(&Number::from_i64(m * n))?);
        prop_assert_eq!(a.pow(&pm.neg())?, one().div(&a.pow(&pm)?)?);
        prop_assert_eq!(a.pow(&one())?, a.clone());
        prop_assert_eq!(a.pow(&zero())?, one());
    }

    #[test]
    fn repeated_multiplication_equals_pow(a in small_number(), n in 1u32..=12) {
        let mut acc = one();
        for _ in 0..n { acc = acc.mul(&a)?; }
        prop_assert_eq!(a.pow(&Number::from_i64(i64::from(n)))?, acc);
    }

    #[test]
    fn decimal_literals_parse_to_exactly_their_digits(int in "[0-9]{1,30}", frac in "[0-9]{1,30}") {
        let text = format!("{int}.{frac}");
        let n = evaluate(&text)?;
        let scaled = BigInt::from(10).pow(frac.len() as u32);
        let digits: BigInt = format!("{int}{frac}").parse().unwrap();
        prop_assert_eq!(n.numer() * &scaled, &digits * n.denom());
        let exact_text = n.to_exact_decimal().expect("decimal literals always terminate");
        prop_assert_eq!(evaluate(&exact_text)?, n);
    }

    #[test]
    fn scientific_notation_equals_scaling_by_a_power_of_ten(m in "[1-9][0-9]{0,15}", frac in "[0-9]{0,10}", e in -40i32..=40) {
        let mantissa = if frac.is_empty() { m.clone() } else { format!("{m}.{frac}") };
        let sci = evaluate(&format!("{mantissa}e{e}"))?;
        let scaled = evaluate(&format!("{mantissa} * 10^({e})"))?;
        prop_assert_eq!(sci, scaled);
    }

    #[test]
    fn grouped_and_plain_integers_agree(v in 0u64..=u64::MAX) {
        let plain = v.to_string();
        let mut grouped = String::new();
        for (i, c) in plain.chars().enumerate() {
            if i > 0 && (plain.len() - i) % 3 == 0 { grouped.push(','); }
            grouped.push(c);
        }
        prop_assert_eq!(evaluate(&grouped)?, evaluate(&plain)?);
    }
}

fn leaf() -> impl Strategy<Value = Expr> {
    prop_oneof![
        (0u32..=100_000).prop_map(|v| Expr::num(Number::from_i64(i64::from(v)))),
        ("[0-9]{1,3}", "[0-9]{1,6}").prop_map(|(i, f)| {
            let n = evaluate(&format!("{i}.{f}")).unwrap();
            Expr::num(n)
        }),
    ]
}

fn expr() -> impl Strategy<Value = Expr> {
    leaf().prop_recursive(6, 64, 2, |inner| {
        prop_oneof![
            inner.clone().prop_map(Expr::negated),
            (
                prop_oneof![
                    Just(BinOp::Add),
                    Just(BinOp::Sub),
                    Just(BinOp::Mul),
                    Just(BinOp::Div)
                ],
                inner.clone(),
                inner.clone()
            )
                .prop_map(|(op, l, r)| Expr::bin(op, l, r)),
            (inner.clone(), 0u32..=5).prop_map(|(b, e)| Expr::bin(
                BinOp::Pow,
                b,
                Expr::num(Number::from_i64(i64::from(e)))
            )),
            (inner.clone(), 1u32..=4).prop_map(|(b, e)| Expr::bin(
                BinOp::Pow,
                b,
                Expr::num(Number::from_i64(i64::from(e))).negated()
            )),
        ]
    })
}

proptest! {
    #![proptest_config(cfg())]

    #[test]
    fn printing_then_parsing_reproduces_the_tree(e in expr()) {
        let text = e.to_string();
        prop_assert_eq!(parse_expression(&text), Ok(e));
    }

    #[test]
    fn printing_then_evaluating_equals_evaluating_the_tree(e in expr()) {
        prop_assert_eq!(evaluate(&e.to_string()), eval(&e).and_then(|v| v.into_plain()));
    }

    #[test]
    fn spaces_never_change_the_meaning_of_a_printed_expression(e in expr()) {
        let text = e.to_string();
        prop_assert_eq!(evaluate(&text.replace(' ', "")), evaluate(&text));
        prop_assert_eq!(evaluate(&text.replace(' ', "   ")), evaluate(&text));
        prop_assert_eq!(evaluate(&format!("  {text}\t ")), evaluate(&text));
    }

    #[test]
    fn extra_parentheses_never_change_the_value(e in expr()) {
        let text = e.to_string();
        prop_assert_eq!(evaluate(&format!("(({text}))")), evaluate(&text));
    }

    #[test]
    fn negating_twice_is_the_identity_on_the_tree_value(e in expr()) {
        let text = e.to_string();
        prop_assert_eq!(evaluate(&format!("--{text}")), evaluate(&text));
        let cancelled = evaluate(&format!("-({text}) + {text}"));
        match evaluate(&text) {
            Ok(_) => prop_assert_eq!(cancelled, Ok(zero())),
            Err(e) => prop_assert_eq!(cancelled, Err(e)),
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 20000, ..ProptestConfig::default() })]

    #[test]
    fn arbitrary_text_never_panics(s in "\\PC{0,120}") {
        let _ = evaluate(&s);
    }

    #[test]
    fn token_soup_never_panics(s in "[0-9.,+*/^()xXeE×÷–−·⋅∕ \t-]{0,80}") {
        let _ = evaluate(&s);
    }

    #[test]
    fn errors_always_name_a_position_inside_the_input(s in "[0-9.,+*/^()xXeE×÷–−· -]{0,60}") {
        if let Err(Error::UnexpectedChar { at, .. } | Error::UnexpectedToken { at, .. } | Error::InvalidNumber { at, .. } | Error::UnexpectedWord { at, .. }) = evaluate(&s) {
            prop_assert!(at < s.len());
            prop_assert!(s.is_char_boundary(at));
        }
    }
}

fn opts(dp: u32, mode: RoundingMode) -> FormatOptions {
    let mut o = FormatOptions::new(dp).unwrap();
    o.rounding = mode;
    o.group_thousands = false;
    o.trim_trailing_zeros = false;
    o
}

fn unit(dp: u32) -> Number {
    one().div(&evaluate(&format!("10^{dp}")).unwrap()).unwrap()
}

proptest! {
    #![proptest_config(cfg())]

    #[test]
    fn rounded_text_is_within_the_mode_bound_and_the_exact_flag_is_honest(n in number(), dp in 0u32..=15) {
        for mode in [RoundingMode::HalfAwayFromZero, RoundingMode::HalfEven, RoundingMode::TowardZero, RoundingMode::AwayFromZero] {
            let f = format_decimal(&n, &opts(dp, mode));
            let shown = evaluate(&f.text)?;
            let err = shown.sub(&n)?.abs();
            let ulp = unit(dp);
            prop_assert_eq!(f.exact, shown == n, "{:?} {:?}", mode, f.text);
            match mode {
                RoundingMode::HalfAwayFromZero | RoundingMode::HalfEven => {
                    prop_assert!(err.mul(&Number::from_i64(2))? <= ulp, "{:?}", f.text);
                }
                RoundingMode::TowardZero => {
                    prop_assert!(shown.abs() <= n.abs() && err < ulp, "{:?}", f.text);
                }
                RoundingMode::AwayFromZero => {
                    prop_assert!(shown.abs() >= n.abs() && err < ulp, "{:?}", f.text);
                }
            }
            let digits_after_point = f.text.split('.').nth(1).map_or(0, str::len);
            prop_assert_eq!(digits_after_point as u32, dp);
        }
    }

    #[test]
    fn ties_break_by_mode(k in -1_000_000i64..=1_000_000, dp in 0u32..=8) {
        let tie = Number::from_i64(2 * k + 1).div(&Number::from_i64(2))?.mul(&unit(dp))?;
        let away = format_decimal(&tie, &opts(dp, RoundingMode::HalfAwayFromZero));
        let even = format_decimal(&tie, &opts(dp, RoundingMode::HalfEven));
        let (a, e) = (evaluate(&away.text)?, evaluate(&even.text)?);
        prop_assert!(a.abs() > tie.abs());
        let last = even.text.chars().rev().find(char::is_ascii_digit).unwrap();
        prop_assert!(last.to_digit(10).unwrap() % 2 == 0, "{}", even.text);
        prop_assert!(e.sub(&tie)?.abs().mul(&Number::from_i64(2))? == unit(dp));
        prop_assert!(!away.exact && !even.exact);
    }

    #[test]
    fn formatting_is_sign_symmetric(n in number(), dp in 0u32..=12) {
        for mode in [RoundingMode::HalfAwayFromZero, RoundingMode::HalfEven, RoundingMode::TowardZero, RoundingMode::AwayFromZero] {
            let p = format_decimal(&n.abs(), &opts(dp, mode));
            let m = format_decimal(&n.abs().neg(), &opts(dp, mode));
            prop_assert_eq!(m.exact, p.exact);
            let zero_text = evaluate(&p.text)?.is_zero();
            if zero_text { prop_assert_eq!(m.text, p.text); } else { prop_assert_eq!(m.text, format!("-{}", p.text)); }
        }
    }

    #[test]
    fn formatting_is_monotone(a in number(), b in number(), dp in 0u32..=12) {
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        for mode in [RoundingMode::HalfAwayFromZero, RoundingMode::HalfEven, RoundingMode::TowardZero, RoundingMode::AwayFromZero] {
            let (x, y) = (evaluate(&format_decimal(&lo, &opts(dp, mode)).text)?, evaluate(&format_decimal(&hi, &opts(dp, mode)).text)?);
            prop_assert!(x <= y, "{:?}", mode);
        }
    }

    #[test]
    fn formatting_is_idempotent_and_terminating_values_round_trip(n in number(), dp in 0u32..=12) {
        let f = format_decimal(&n, &opts(dp, RoundingMode::HalfAwayFromZero));
        let again = format_decimal(&evaluate(&f.text)?, &opts(dp, RoundingMode::HalfAwayFromZero));
        prop_assert_eq!(&again.text, &f.text);
        prop_assert!(again.exact);
        if n.terminating_places().is_some_and(|places| places <= dp) {
            prop_assert!(f.exact);
            prop_assert_eq!(evaluate(&f.text)?, n);
        }
    }

    #[test]
    fn grouping_only_inserts_commas_between_integer_digits(n in number(), dp in 0u32..=10) {
        let mut g = opts(dp, RoundingMode::HalfAwayFromZero);
        let plain = format_decimal(&n, &g).text;
        g.group_thousands = true;
        let grouped = format_decimal(&n, &g).text;
        prop_assert_eq!(grouped.replace(',', ""), plain.clone());
        let int_part = grouped.trim_start_matches('-').split('.').next().unwrap();
        for (i, group) in int_part.split(',').enumerate() {
            prop_assert!(group.len() == 3 || (i == 0 && (1..=3).contains(&group.len())), "{grouped}");
        }
    }

    #[test]
    fn trimming_only_removes_trailing_fraction_zeros(n in number(), dp in 0u32..=10) {
        let full = opts(dp, RoundingMode::HalfAwayFromZero);
        let mut trimmed = full.clone();
        trimmed.trim_trailing_zeros = true;
        let (f, t) = (format_decimal(&n, &full).text, format_decimal(&n, &trimmed).text);
        prop_assert_eq!(evaluate(&f)?, evaluate(&t)?);
        prop_assert!(!t.contains('.') || !t.ends_with('0'));
        prop_assert!(!t.ends_with('.'));
        prop_assert!(f.starts_with(t.trim_end_matches('.')) || t == "0");
    }
}
