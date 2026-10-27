//! Colors. One dark theme ships; Omarchy's `colors.toml` replaces its main colors when it can be read,
//! and the file is watched so a theme switch shows without restarting.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use gpui::{App, Global, Hsla, Rgba};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Colors {
    pub background: Hsla,
    pub sidebar: Hsla,
    pub answers: Hsla,
    pub raised: Hsla,
    pub line: Hsla,
    pub text: Hsla,
    pub muted: Hsla,
    pub gutter: Hsla,
    pub number: Hsla,
    pub unit: Hsla,
    pub currency: Hsla,
    pub chip_bg: Hsla,
    pub chip_text: Hsla,
    /// A name the sheet defined.
    pub variable: Hsla,
    pub comment: Hsla,
    pub ignored: Hsla,
    pub warn: Hsla,
    pub selection: Hsla,
    pub caret: Hsla,
}

impl Global for Colors {}

/// 0xRRGGBB.
fn hex(v: u32) -> Hsla {
    Rgba {
        r: ((v >> 16) & 0xff) as f32 / 255.,
        g: ((v >> 8) & 0xff) as f32 / 255.,
        b: (v & 0xff) as f32 / 255.,
        a: 1.,
    }
    .into()
}

fn rgb_of(v: u32) -> [f32; 3] {
    [
        ((v >> 16) & 0xff) as f32,
        ((v >> 8) & 0xff) as f32,
        (v & 0xff) as f32,
    ]
}

/// `a` moved `t` of the way to `b`, both 0xRRGGBB.
fn mix(a: u32, b: u32, t: f32) -> u32 {
    let (x, y) = (rgb_of(a), rgb_of(b));
    let c = |i: usize| (x[i] + (y[i] - x[i]) * t).round() as u32;
    (c(0) << 16) | (c(1) << 8) | c(2)
}

impl Colors {
    /// The built-in dark theme, picked from the target screenshot.
    pub fn dark() -> Colors {
        Colors::from_palette(&Palette::default())
    }

    /// These colors with the ones the settings change laid over them.
    pub fn with_overrides(mut self) -> Colors {
        use crate::appprefs::color;
        if let Some(c) = color("color_number") {
            self.number = hex(c);
        }
        if let Some(c) = color("color_unit") {
            self.unit = hex(c);
        }
        if let Some(c) = color("color_currency") {
            self.currency = hex(c);
        }
        if let Some(c) = color("color_variable") {
            self.variable = hex(c);
        }
        if let Some(c) = color("answer_background") {
            self.answers = hex(c);
        }
        self
    }

    fn from_palette(p: &Palette) -> Colors {
        let sidebar = mix(p.background, 0x000000, 0.06);
        let answers = mix(p.background, p.foreground, 0.035);
        Colors {
            background: hex(p.background),
            sidebar: hex(sidebar),
            answers: hex(answers),
            raised: hex(mix(p.background, p.foreground, 0.12)),
            line: hex(mix(p.background, p.foreground, 0.10)),
            text: hex(p.foreground),
            muted: hex(mix(p.background, p.foreground, 0.5)),
            gutter: hex(mix(p.background, p.foreground, 0.38)),
            number: hex(p.number),
            unit: hex(p.unit),
            currency: hex(p.currency),
            chip_bg: hex(p.chip),
            chip_text: hex(mix(p.background, 0x000000, 0.6)),
            variable: hex(0x62c25a),
            comment: hex(mix(p.background, p.foreground, 0.45)),
            ignored: hex(mix(p.background, p.foreground, 0.38)),
            warn: hex(p.warn),
            selection: hex(mix(p.background, p.accent, 0.4)),
            caret: hex(p.accent),
        }
    }
}

/// The few colors a theme file may set; everything else is derived from them.
#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    pub background: u32,
    pub foreground: u32,
    pub accent: u32,
    pub number: u32,
    pub unit: u32,
    pub currency: u32,
    pub chip: u32,
    pub warn: u32,
}

impl Default for Palette {
    fn default() -> Palette {
        Palette {
            background: 0x1f1f1f,
            foreground: 0xe6e6e6,
            accent: 0x7aa2f7,
            number: 0xb4ecf7,
            unit: 0x9fdcea,
            currency: 0xf0b4f0,
            chip: 0xcfe3ff,
            warn: 0xe0af68,
        }
    }
}

/// `key = "#rrggbb"` lines of an Omarchy `colors.toml`. Keys this does not know are ignored, and a
/// value that is not a color keeps the default, so a half-written file never breaks the app.
pub fn parse_palette(src: &str) -> Palette {
    let mut p = Palette::default();
    let get = |key: &str| -> Option<u32> {
        src.lines().find_map(|l| {
            let (k, v) = l.split_once('=')?;
            if k.trim() != key {
                return None;
            }
            let v = v.trim().trim_matches(['"', '\'']);
            let v = v.strip_prefix('#').or_else(|| v.strip_prefix("0x"))?;
            (v.len() == 6)
                .then(|| u32::from_str_radix(v, 16).ok())
                .flatten()
        })
    };
    if let Some(c) = get("background") {
        p.background = c;
    }
    if let Some(c) = get("foreground") {
        p.foreground = c;
    }
    if let Some(c) = get("accent").or_else(|| get("color4")) {
        p.accent = c;
    }
    if let Some(c) = get("cyan").or_else(|| get("color6")) {
        p.number = mix(c, p.foreground, 0.35);
        p.unit = c;
    }
    if let Some(c) = get("magenta").or_else(|| get("color5")) {
        p.currency = mix(c, p.foreground, 0.3);
    }
    if let Some(c) = get("blue").or_else(|| get("color4")) {
        p.chip = mix(c, 0xffffff, 0.65);
    }
    if let Some(c) = get("yellow").or_else(|| get("color3")) {
        p.warn = c;
    }
    p
}

pub fn theme_file() -> Option<PathBuf> {
    if let Some(f) = std::env::var_os("BABBAGE_THEME_FILE").filter(|f| !f.is_empty()) {
        return Some(PathBuf::from(f));
    }
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".local/state/omarchy/current/theme/colors.toml"))
}

fn load(file: &Option<PathBuf>) -> (Colors, Option<SystemTime>) {
    let Some(f) = file else {
        return (Colors::dark(), None);
    };
    let modified = std::fs::metadata(f).and_then(|m| m.modified()).ok();
    match std::fs::read_to_string(f) {
        Ok(src) => (Colors::from_palette(&parse_palette(&src)), modified),
        Err(_) => (Colors::dark(), modified),
    }
}

/// Reads the theme file again and lays the settings' colors over it: after a color setting changed.
pub fn reload(cx: &mut App) {
    let (colors, _) = load(&theme_file());
    let colors = colors.with_overrides();
    if *cx.global::<Colors>() != colors {
        cx.set_global(colors);
        cx.refresh_windows();
    }
}

/// Sets the colors global and follows the theme file for changes (checked once a second: a theme
/// switch replaces the file, and polling its time needs no file-watching dependency).
pub fn install(cx: &mut App) {
    let file = theme_file();
    let (colors, mut seen) = load(&file);
    cx.set_global(colors.with_overrides());
    cx.spawn(async move |cx| {
        loop {
            cx.background_executor().timer(Duration::from_secs(1)).await;
            let modified = file
                .as_ref()
                .and_then(|f| std::fs::metadata(f).and_then(|m| m.modified()).ok());
            if modified == seen {
                continue;
            }
            seen = modified;
            let (colors, _) = load(&file);
            let colors = colors.with_overrides();
            if cx
                .update(|cx| {
                    if *cx.global::<Colors>() != colors {
                        cx.set_global(colors);
                        cx.refresh_windows();
                    }
                })
                .is_err()
            {
                break;
            }
        }
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_theme_file_sets_the_main_colours() {
        let p = parse_palette(
            "background = \"#101820\"\nforeground = \"#ddeedd\"\naccent = \"#ff8800\"\n",
        );
        assert_eq!(
            (p.background, p.foreground, p.accent),
            (0x101820, 0xddeedd, 0xff8800)
        );
    }

    #[test]
    fn bad_values_and_unknown_keys_keep_the_defaults() {
        let d = Palette::default();
        let p = parse_palette("background = \"red\"\nfoo = \"#123456\"\nforeground = \"#12\"\n");
        assert_eq!(p, d);
    }

    #[test]
    fn terminal_colour_numbers_are_accepted_for_older_themes() {
        let p = parse_palette("color6 = \"#00ffff\"\n");
        assert_eq!(p.unit, 0x00ffff);
    }

    #[test]
    fn mixing_endpoints_return_the_inputs() {
        assert_eq!(mix(0x102030, 0xffffff, 0.0), 0x102030);
        assert_eq!(mix(0x102030, 0xffffff, 1.0), 0xffffff);
    }
}
