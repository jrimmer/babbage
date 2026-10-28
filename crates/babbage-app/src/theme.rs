//! Colors. Omarchy's `colors.toml` sets the palette when it can be read, light or dark by its `mode` line, and the
//! file is watched so a theme switch shows without restarting. Without it a built-in dark or light theme is drawn,
//! following the system's light or dark setting (`BABBAGE_APPEARANCE=light` or `dark` overrides it).

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use gpui::{App, Global, Hsla, Rgba, WindowAppearance};

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
    /// The built-in theme for a system that prefers light or dark; the dark one was picked from the target screenshot.
    pub fn builtin(light: bool) -> Colors {
        Colors::from_palette(&if light {
            Palette::light()
        } else {
            Palette::default()
        })
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

    /// Every color from the palette's few. Colors that carry text are moved toward the foreground until they can be
    /// read on the background, so a light theme and a theme with a dull accent stay legible.
    fn from_palette(p: &Palette) -> Colors {
        let (bg, fg) = (p.background, p.foreground);
        let sidebar = mix(bg, 0x000000, 0.06);
        let answers = mix(bg, fg, 0.035);
        // Text sits on the page and on the answers column, which is a little different from it.
        let read =
            |c: u32, min: f32| ensure_contrast(ensure_contrast(c, bg, fg, min), answers, fg, min);
        let chip = p.chip;
        Colors {
            background: hex(bg),
            sidebar: hex(sidebar),
            answers: hex(answers),
            raised: hex(mix(bg, fg, 0.12)),
            line: hex(mix(bg, fg, 0.10)),
            text: hex(fg),
            muted: hex(read(mix(bg, fg, 0.5), DIM)),
            gutter: hex(read(mix(bg, fg, 0.38), FAINT)),
            number: hex(read(p.number, READABLE)),
            unit: hex(read(p.unit, READABLE)),
            currency: hex(read(p.currency, READABLE)),
            chip_bg: hex(chip),
            chip_text: hex(ensure_contrast(
                mix(bg, 0x000000, 0.6),
                chip,
                0x000000,
                READABLE,
            )),
            variable: hex(read(p.variable, READABLE)),
            comment: hex(read(mix(bg, fg, 0.45), DIM)),
            ignored: hex(read(mix(bg, fg, 0.38), FAINT)),
            warn: hex(read(p.warn, READABLE)),
            // The text must stay readable over a selection, so a strong accent is thinned toward the page.
            selection: hex(ensure_contrast(mix(bg, p.accent, 0.4), fg, bg, READABLE)),
            caret: hex(read(p.accent, DIM)),
        }
    }
}

/// The least contrast ratio for text that carries meaning, for text that is meant to be dim, and for the faintest
/// marks (line numbers, words the sheet ignores).
const READABLE: f32 = 4.5;
const DIM: f32 = 3.0;
const FAINT: f32 = 2.5;

/// WCAG relative luminance of 0xRRGGBB.
fn luminance(c: u32) -> f32 {
    let lin = |v: f32| {
        let v = v / 255.;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let [r, g, b] = rgb_of(c);
    0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

/// WCAG contrast ratio of two colors, 1 to 21.
fn contrast(a: u32, b: u32) -> f32 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// `c`, or the first color on the way from `c` to `toward` that reads on `bg` with ratio `min`. A color that
/// already reads is returned as it is.
fn ensure_contrast(c: u32, bg: u32, toward: u32, min: f32) -> u32 {
    (0..=20)
        .map(|step| mix(c, toward, step as f32 / 20.))
        .find(|m| contrast(*m, bg) >= min)
        .unwrap_or(toward)
}

/// The few colors a theme file may set; everything else is derived from them.
#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    /// A light theme: the file's `mode = "light"`, or a background that is light.
    pub light: bool,
    pub background: u32,
    pub foreground: u32,
    pub accent: u32,
    pub number: u32,
    pub unit: u32,
    pub currency: u32,
    pub chip: u32,
    /// A name the sheet defined.
    pub variable: u32,
    pub warn: u32,
}

impl Default for Palette {
    fn default() -> Palette {
        Palette {
            light: false,
            background: 0x1f1f1f,
            foreground: 0xe6e6e6,
            accent: 0x7aa2f7,
            number: 0xb4ecf7,
            unit: 0x9fdcea,
            currency: 0xf0b4f0,
            chip: 0xcfe3ff,
            variable: 0x62c25a,
            warn: 0xe0af68,
        }
    }
}

impl Palette {
    /// The built-in light theme.
    pub fn light() -> Palette {
        Palette {
            light: true,
            background: 0xf7f7f5,
            foreground: 0x2b2d33,
            accent: 0x2f6fdb,
            number: 0x0b5f73,
            unit: 0x127a8a,
            currency: 0x8e3a95,
            chip: 0xa9c8f5,
            variable: 0x2f7d2a,
            warn: 0x9a5b00,
        }
    }
}

/// The text after `key =` on the first line that sets it, without quotes.
fn value_of<'a>(src: &'a str, key: &str) -> Option<&'a str> {
    src.lines().find_map(|l| {
        let (k, v) = l.split_once('=')?;
        (k.trim() == key).then(|| v.trim().trim_matches(['"', '\'']))
    })
}

/// `key = "#rrggbb"` lines of an Omarchy `colors.toml`. Keys this does not know are ignored, and a
/// value that is not a color keeps the default, so a half-written file never breaks the app. The `mode` line says
/// whether the theme is light; without one the background decides.
pub fn parse_palette(src: &str) -> Palette {
    let get = |key: &str| -> Option<u32> {
        let v = value_of(src, key)?;
        let v = v.strip_prefix('#').or_else(|| v.strip_prefix("0x"))?;
        (v.len() == 6)
            .then(|| u32::from_str_radix(v, 16).ok())
            .flatten()
    };
    let background = get("background");
    let light = match value_of(src, "mode")
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("light") => true,
        Some("dark") => false,
        _ => background.is_some_and(|b| luminance(b) > 0.5),
    };
    let mut p = if light {
        Palette::light()
    } else {
        Palette::default()
    };
    if let Some(c) = background {
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
        // A pale chip carries dark text on a dark theme; on a light one it must still stand out from the page.
        p.chip = mix(c, 0xffffff, if light { 0.35 } else { 0.65 });
    }
    if let Some(c) = get("green").or_else(|| get("color2")) {
        p.variable = c;
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

/// Whether the built-in theme should be light: `BABBAGE_APPEARANCE` when it says `light` or `dark`, else the
/// system's setting.
fn system_light(cx: &App) -> bool {
    match std::env::var("BABBAGE_APPEARANCE")
        .map(|v| v.to_ascii_lowercase())
        .as_deref()
    {
        Ok("light") => true,
        Ok("dark") => false,
        _ => matches!(
            cx.window_appearance(),
            WindowAppearance::Light | WindowAppearance::VibrantLight
        ),
    }
}

fn load(file: &Option<PathBuf>, light: bool) -> (Colors, Option<SystemTime>) {
    let Some(f) = file else {
        return (Colors::builtin(light), None);
    };
    let modified = std::fs::metadata(f).and_then(|m| m.modified()).ok();
    match std::fs::read_to_string(f) {
        Ok(src) => (Colors::from_palette(&parse_palette(&src)), modified),
        Err(_) => (Colors::builtin(light), modified),
    }
}

/// Reads the theme file again and lays the settings' colors over it: after a color setting changed.
pub fn reload(cx: &mut App) {
    let (colors, _) = load(&theme_file(), system_light(cx));
    let colors = colors.with_overrides();
    if *cx.global::<Colors>() != colors {
        cx.set_global(colors);
        cx.refresh_windows();
    }
}

/// Sets the colors global and follows the theme file and the system's light or dark setting for changes (checked
/// once a second: a theme switch replaces the file, and polling its time needs no file-watching dependency).
pub fn install(cx: &mut App) {
    let file = theme_file();
    let mut light = system_light(cx);
    let (colors, mut seen) = load(&file, light);
    cx.set_global(colors.with_overrides());
    cx.spawn(async move |cx| {
        loop {
            cx.background_executor().timer(Duration::from_secs(1)).await;
            let modified = file
                .as_ref()
                .and_then(|f| std::fs::metadata(f).and_then(|m| m.modified()).ok());
            let Ok(now_light) = cx.update(|cx| system_light(cx)) else {
                break;
            };
            if modified == seen && now_light == light {
                continue;
            }
            seen = modified;
            light = now_light;
            let (colors, _) = load(&file, light);
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

    const LATTE: &str = "mode = \"light\"\naccent = \"#1e66f5\"\nbackground = \"#eff1f5\"\nforeground = \"#4c4f69\"\nyellow = \"#df8e1d\"\ngreen = \"#40a02b\"\ncyan = \"#179299\"\nblue = \"#1e66f5\"\nmagenta = \"#ea76cb\"\n";
    const WHITE: &str = "mode = \"light\"\naccent = \"#6e6e6e\"\nbackground = \"#ffffff\"\nforeground = \"#1a1a1a\"\nyellow = \"#f5c542\"\ngreen = \"#8fd18f\"\ncyan = \"#9ad5e0\"\nblue = \"#a9c1e8\"\nmagenta = \"#e6b3e6\"\n";
    const FLEXOKI_LIGHT: &str = "mode = \"light\"\naccent = \"#205EA6\"\nbackground = \"#FFFCF0\"\nforeground = \"#100F0F\"\nyellow = \"#AD8301\"\ngreen = \"#66800B\"\ncyan = \"#24837B\"\nblue = \"#205EA6\"\nmagenta = \"#A02F6F\"\n";
    const LIFTOFF: &str = "mode = \"dark\"\naccent = \"#F3B77B\"\nbackground = \"#151E26\"\nforeground = \"#ECE5D8\"\nyellow = \"#EAD093\"\ngreen = \"#A9C59A\"\ncyan = \"#8BC8D0\"\nblue = \"#7BBCE0\"\nmagenta = \"#C3ABCC\"\n";

    fn rgb(c: Hsla) -> u32 {
        let r: Rgba = c.into();
        let byte = |v: f32| (v * 255.).round() as u32;
        (byte(r.r) << 16) | (byte(r.g) << 8) | byte(r.b)
    }

    /// Every pair of colors the interface draws one on the other, with the least ratio each must keep.
    fn pairs(c: &Colors) -> Vec<(&'static str, u32, u32, f32)> {
        let (bg, answers) = (rgb(c.background), rgb(c.answers));
        let mut v = vec![("text", rgb(c.text), bg, 7.0)];
        for (name, color) in [
            ("number", c.number),
            ("unit", c.unit),
            ("currency", c.currency),
            ("variable", c.variable),
            ("warn", c.warn),
        ] {
            v.push((name, rgb(color), bg, READABLE));
            v.push((name, rgb(color), answers, READABLE));
        }
        for (name, color) in [
            ("muted", c.muted),
            ("comment", c.comment),
            ("caret", c.caret),
        ] {
            v.push((name, rgb(color), bg, DIM));
        }
        for (name, color) in [("gutter", c.gutter), ("ignored", c.ignored)] {
            v.push((name, rgb(color), bg, FAINT));
            v.push((name, rgb(color), answers, FAINT));
        }
        v.push(("chip", rgb(c.chip_text), rgb(c.chip_bg), READABLE));
        v.push(("selected text", rgb(c.text), rgb(c.selection), READABLE));
        v.push(("raised text", rgb(c.text), rgb(c.raised), READABLE));
        v
    }

    #[test]
    fn every_theme_keeps_its_text_readable_light_or_dark() {
        let mut themes: Vec<(&str, Colors)> = vec![
            ("built-in dark", Colors::builtin(false)),
            ("built-in light", Colors::builtin(true)),
        ];
        for (name, src) in [
            ("latte", LATTE),
            ("white", WHITE),
            ("flexoki-light", FLEXOKI_LIGHT),
            ("liftoff", LIFTOFF),
        ] {
            themes.push((name, Colors::from_palette(&parse_palette(src))));
        }
        for (name, colors) in &themes {
            for (what, fg, bg, min) in pairs(colors) {
                let got = contrast(fg, bg);
                assert!(
                    got >= min,
                    "{name}: {what} {fg:06x} on {bg:06x} has contrast {got:.2}, needs {min}"
                );
            }
        }
    }

    #[test]
    fn the_mode_line_says_light_and_without_one_the_background_does() {
        assert!(parse_palette(LATTE).light);
        assert!(!parse_palette(LIFTOFF).light);
        assert!(parse_palette("background = \"#fafafa\"\n").light);
        assert!(!parse_palette("background = \"#202020\"\n").light);
        assert!(!parse_palette("foreground = \"#202020\"\n").light);
        // The mode line wins over the background, whichever way they disagree.
        assert!(parse_palette("mode = \"light\"\nbackground = \"#202020\"\n").light);
        assert!(!parse_palette("mode = \"dark\"\nbackground = \"#fafafa\"\n").light);
        assert!(parse_palette("mode = \"LIGHT\"\n").light);
    }

    #[test]
    fn a_light_theme_reads_its_colors_and_is_a_light_page() {
        let p = parse_palette(LATTE);
        assert_eq!(
            (p.background, p.foreground, p.accent),
            (0xeff1f5, 0x4c4f69, 0x1e66f5)
        );
        let c = Colors::from_palette(&p);
        assert!(luminance(rgb(c.background)) > luminance(rgb(c.text)));
        assert_eq!(rgb(c.background), 0xeff1f5);
        assert_eq!(rgb(c.text), 0x4c4f69);
        // The accent is the caret, kept as it is when it already reads.
        assert_eq!(rgb(c.caret), 0x1e66f5);
    }

    #[test]
    fn the_built_in_themes_follow_the_system_and_dark_is_unchanged() {
        let dark = Colors::builtin(false);
        let (bg, fg) = (0x1f1f1f, 0xe6e6e6);
        assert_eq!(dark.muted, hex(mix(bg, fg, 0.5)));
        assert_eq!(dark.gutter, hex(mix(bg, fg, 0.38)));
        assert_eq!(dark.comment, hex(mix(bg, fg, 0.45)));
        assert_eq!(dark.number, hex(0xb4ecf7));
        assert_eq!(dark.variable, hex(0x62c25a));
        assert_eq!(dark.chip_text, hex(mix(bg, 0, 0.6)));
        let light = Colors::builtin(true);
        assert_ne!(light, dark);
        assert!(luminance(rgb(light.background)) > 0.8);
    }

    #[test]
    fn a_dull_color_is_moved_toward_the_text_until_it_reads() {
        // Pale yellow on white does not read; it darkens, and stops at the first color that does.
        let fixed = ensure_contrast(0xffee88, 0xffffff, 0x000000, READABLE);
        assert!(contrast(fixed, 0xffffff) >= READABLE);
        assert!(contrast(mix(0xffee88, 0, 0.0), 0xffffff) < READABLE);
        // One that reads is left as it is, and when nothing on the way reads the target is returned.
        assert_eq!(ensure_contrast(0x202020, 0xffffff, 0, READABLE), 0x202020);
        assert_eq!(
            ensure_contrast(0x777777, 0x777777, 0xffffff, 21.0),
            0xffffff
        );
    }

    #[test]
    fn the_first_color_on_the_way_that_reads_is_the_one_returned() {
        // Pale yellow on white darkens in twentieths and stops at the first that reaches 4.5 (the tenth, 4.53).
        assert_eq!(ensure_contrast(0xffee88, 0xffffff, 0, READABLE), 0x807744);
    }

    #[test]
    fn dark_channels_use_the_linear_part_of_the_curve() {
        assert!((luminance(0x080808) - 0.0024282).abs() < 1e-6);
        assert!((contrast(0x000000, 0x080808) - 1.04856).abs() < 1e-4);
        assert!((contrast(0x090909, 0x000000) - 1.05463).abs() < 1e-4);
    }

    #[test]
    fn contrast_is_the_standard_ratio() {
        assert!((contrast(0x000000, 0xffffff) - 21.0).abs() < 0.01);
        assert!((contrast(0xffffff, 0xffffff) - 1.0).abs() < 1e-6);
        assert!((contrast(0x777777, 0xffffff) - 4.48).abs() < 0.02);
    }

    #[test]
    fn mixing_endpoints_return_the_inputs() {
        assert_eq!(mix(0x102030, 0xffffff, 0.0), 0x102030);
        assert_eq!(mix(0x102030, 0xffffff, 1.0), 0xffffff);
    }
}
