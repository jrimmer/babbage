use babbage_core::settings::{Settings, scoped};
use babbage_core::{FormatOptions, LineKind, SheetOptions, evaluate_sheet, format_value};
fn main() {
    let lines: Vec<String> = std::env::args().skip(1).collect();
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let s = Settings::at(1_718_452_800);
    let opts = SheetOptions {
        settings: s.clone(),
        ..SheetOptions::default()
    };
    scoped(&s, || {
        for (l, r) in lines.iter().zip(evaluate_sheet(&refs, &opts)) {
            let t = match &r.kind {
                LineKind::Value(v) => {
                    format_value(v, &r.display_options(&FormatOptions::default())).text
                }
                LineKind::Error(e) => format!("ERROR {e}"),
                o => format!("{o:?}"),
            };
            println!(
                "{l}  =>  {t}   ignored={:?}",
                r.ignored.iter().map(|i| i.text.clone()).collect::<Vec<_>>()
            );
        }
    });
}
