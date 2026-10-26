use babbage_core::settings::{Settings, scoped};
use babbage_core::zones::Zone;
use babbage_core::{FormatOptions, LineKind, SheetOptions, evaluate_sheet, format_value};
fn main() {
    let lines: Vec<String> = std::env::args().skip(1).collect();
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let now = std::env::var("PROBE_NOW")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1_718_452_800);
    let s = Settings {
        local_zone: std::env::var("PROBE_ZONE")
            .map_or_else(|_| Settings::at(now).local_zone, |z| Zone::named(&z)),
        ..Settings::at(now)
    };
    let opts = SheetOptions {
        settings: s.clone(),
        ..SheetOptions::default()
    };
    scoped(&s, || {
        let results = evaluate_sheet(&refs, &opts);
        if std::env::var("PROBE_TOTAL").is_ok() {
            let total = babbage_core::sheet::floating_total(&results, &opts);
            let text = match total {
                Some(Ok(v)) => format_value(&v, &FormatOptions::default()).text,
                Some(Err(e)) => format!("ERROR {e}"),
                None => "(none)".into(),
            };
            println!("TOTAL => {text}");
            return;
        }
        for (l, r) in lines.iter().zip(results) {
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
