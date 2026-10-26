use std::path::PathBuf;

use babbage_core::settings::{Settings, scoped};
use babbage_core::zones::{BUNDLED_RELEASE, Zone, ZoneSource, current_release, release};
use babbage_core::{FormatOptions, evaluate_with, format_value};

const CORPUS: &str = include_str!("corpus/systemzones.tsv");

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/zoneinfo")
}

fn system(now: i64, local: Option<&str>) -> Settings {
    let mut s = Settings {
        zone_source: ZoneSource::System { dir: fixtures() },
        ..Settings::at(now)
    };
    s.local_zone = local.and_then(|name| scoped(&s, || Zone::named(name)));
    s
}

fn run(s: &Settings, line: &str) -> String {
    scoped(s, || match evaluate_with(line, s) {
        Ok(v) => format_value(&v, &FormatOptions::default()).text,
        Err(e) => format!("ERROR {e}"),
    })
}

#[test]
fn agrees_with_python_reading_the_same_zoneinfo_files() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for line in CORPUS.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        let s = system(
            cols[1].parse().unwrap(),
            (cols[0] != "-").then_some(cols[0]),
        );
        let got = run(&s, cols[2]);
        if got != cols[3] {
            failures.push(format!(
                "{} {}: want {}, got {got}",
                cols[0], cols[2], cols[3]
            ));
        }
        checked += 1;
    }
    assert!(checked > 6_000, "{checked}");
    assert!(
        failures.is_empty(),
        "{} of {checked} differ, first 15:\n{}",
        failures.len(),
        failures
            .iter()
            .take(15)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn the_machines_newer_rules_replace_the_bundled_ones() {
    // 2026-12-01 12:00 UTC. In 2025b Vancouver is on UTC-8 in December; the 2026e files differ from 2026-11-04.
    let now = 1_795_003_200 + 13 * 86_400; // 2026-12-01 12:00 UTC
    let bundled = Settings::at(now);
    let files = system(now, None);
    let line = "now in Vancouver";
    let (old, new) = (run(&bundled, line), run(&files, line));
    assert_ne!(old, new, "the rule change should be visible: {old}");
    assert_eq!(new, "December 1 at 5:00 am", "from the 2026e files");
    // Casablanca: 2026-09-23 onward.
    assert_ne!(
        run(&bundled, "now in Casablanca"),
        run(&files, "now in Casablanca")
    );
}

#[test]
fn a_zone_the_folder_lacks_falls_back_to_the_bundled_database() {
    let files = system(1_718_452_800, None);
    // Paris is not in the fixtures; its bundled rules still apply.
    assert_eq!(run(&files, "time in Paris"), "2:00 pm");
    let missing = Settings {
        zone_source: ZoneSource::System {
            dir: fixtures().join("nowhere"),
        },
        ..Settings::at(1_718_452_800)
    };
    assert_eq!(run(&missing, "time in Paris"), "2:00 pm");
    assert_eq!(run(&missing, "time in Tokyo"), "9:00 pm");
}

#[test]
fn a_name_that_could_leave_the_folder_is_never_read() {
    let files = system(1_718_452_800, None);
    scoped(&files, || {
        for name in [
            "../zoneinfo/Asia/Tokyo",
            "/etc/passwd",
            "Asia//Tokyo",
            "Asia/./Tokyo",
            "",
            "Asia/Tokyo\0",
            "..",
            "America/../Asia/Tokyo",
        ] {
            let zone = Zone::named(name);
            assert!(
                !matches!(zone, Some(Zone::Loaded(_))),
                "{name:?} must not load from the folder"
            );
        }
        assert!(matches!(Zone::named("Asia/Tokyo"), Some(Zone::Loaded(_))));
    });
}

#[test]
fn the_release_in_use_is_read_from_the_folder() {
    assert_eq!(release(&fixtures()).as_deref(), Some("2026e"));
    assert_eq!(release(&fixtures().join("nowhere")), None);
    let files = system(0, None);
    assert_eq!(scoped(&files, current_release), "2026e");
    assert_eq!(
        scoped(&Settings::default(), current_release),
        BUNDLED_RELEASE
    );
}
