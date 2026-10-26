use babbage_core::settings::{Settings, scoped};
use babbage_core::{FormatOptions, evaluate_with, format_value};
fn main() {
    let s = Settings::at(1_718_452_800);
    for line in std::env::args().skip(1) {
        let out = scoped(&s, || match evaluate_with(&line, &s) {
            Ok(v) => format_value(&v, &FormatOptions::default()).text,
            Err(e) => format!("ERROR {e}"),
        });
        println!("{line}  =>  {out}");
    }
}
