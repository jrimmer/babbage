//! `babbage-accept [check ...]`: runs the acceptance checks against `target/release/babbage` in the nested
//! headless Hyprland. With no names, every check runs. Exit code 1 if any failed.

mod checks;
mod harness;

use std::path::PathBuf;
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    let wanted: Vec<String> = std::env::args().skip(1).collect();
    if wanted.iter().any(|a| a == "--list") {
        for (name, _) in checks::CHECKS {
            println!("{name}");
        }
        return Ok(());
    }
    for w in &wanted {
        anyhow::ensure!(
            checks::CHECKS.iter().any(|(n, _)| n == w),
            "no check named {w}; try --list"
        );
    }
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let mut h = harness::Harness::new(repo)?;
    let mut failed = Vec::new();
    for (name, check) in checks::CHECKS {
        if !wanted.is_empty() && !wanted.iter().any(|w| w == name) {
            continue;
        }
        let start = Instant::now();
        match check(&mut h) {
            Ok(()) => println!("PASS {name} ({:.1}s)", start.elapsed().as_secs_f32()),
            Err(e) => {
                println!(
                    "FAIL {name} ({:.1}s)\n  {e:#}",
                    start.elapsed().as_secs_f32()
                );
                failed.push(*name);
            }
        }
    }
    println!("logs: {}", h.logs.display());
    if failed.is_empty() {
        println!("bin/accept passed");
        Ok(())
    } else {
        println!("failed: {}", failed.join(", "));
        std::process::exit(1);
    }
}
