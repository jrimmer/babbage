use babbage_core::Error;

#[test]
fn every_error_has_a_distinct_human_readable_message() {
    let all = [
        (Error::Empty, "empty expression"),
        (Error::InputTooLong, "input too long"),
        (Error::UnexpectedChar { ch: '@', at: 3 }, "unexpected character '@' at byte 3"),
        (Error::InvalidNumber { text: "1.2.3".into(), at: 4 }, "invalid number \"1.2.3\" at byte 4"),
        (Error::UnexpectedToken { found: "')'".into(), at: 7 }, "unexpected ')' at byte 7"),
        (Error::UnexpectedEnd, "unexpected end of expression"),
        (Error::UnexpectedWord { word: "abc".into(), at: 2 }, "unexpected word \"abc\" at byte 2"),
        (Error::UnsupportedOperation("no way"), "unsupported: no way"),
        (Error::NotAPlainNumber, "result is not a plain number"),
        (Error::DivisionByZero, "division by zero"),
        (Error::ZeroToTheZero, "0 to the power 0 is undefined"),
        (Error::NonIntegerExponent, "exponent must be a whole number"),
        (Error::Overflow, "number too large to represent"),
        (Error::InvalidPrecision, "decimal places out of range"),
    ];
    for (err, text) in &all {
        assert_eq!(err.to_string(), *text);
    }
    let boxed: Box<dyn std::error::Error> = Box::new(Error::Overflow);
    assert_eq!(boxed.to_string(), "number too large to represent");
}
