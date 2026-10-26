use babbage_core::units::registry;

const GOLDEN: &str = include_str!("corpus/unit-names.txt");

fn snapshot() -> String {
    let mut lines: Vec<String> = registry()
        .names()
        .map(|(name, unit)| format!("{name}\t{}", unit.symbol()))
        .collect();
    lines.sort();
    lines.join("\n") + "\n"
}

/// Pins every spelling the engine accepts. A change here is a deliberate edit of the unit table: review the
/// diff, then regenerate with BABBAGE_UPDATE_GOLDEN=1 cargo test -p babbage-core --test unit_names.
#[test]
fn the_accepted_spellings_are_exactly_the_reviewed_set() {
    if std::env::var_os("BABBAGE_UPDATE_GOLDEN").is_some() {
        std::fs::write(
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/corpus/unit-names.txt"),
            snapshot(),
        )
        .unwrap();
        return;
    }
    let (now, then) = (snapshot(), GOLDEN.to_string());
    if now != then {
        let added: Vec<&str> = now
            .lines()
            .filter(|l| !then.lines().any(|t| t == *l))
            .take(10)
            .collect();
        let removed: Vec<&str> = then
            .lines()
            .filter(|l| !now.lines().any(|t| t == *l))
            .take(10)
            .collect();
        panic!("unit spellings changed. added: {added:?} removed: {removed:?}");
    }
}
