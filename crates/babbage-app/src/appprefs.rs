//! The settings of the app itself (as against the engine's): how sheets are ordered, what happens at launch, how
//! typing and renaming behave, and how the window looks. They are lines of the same settings file, read through
//! `prefs::raw`.

use crate::prefs::raw;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SheetOrder {
    Modified,
    Created,
    Title,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenamePolicy {
    Always,
    Ask,
    Never,
}

fn flag(key: &str, default: bool) -> bool {
    match raw(key).as_deref() {
        Some("on" | "yes" | "true") => true,
        Some("off" | "no" | "false") => false,
        _ => default,
    }
}

/// How the sidebar lists sheets.
pub fn sheet_order() -> SheetOrder {
    match raw("sheet_order").as_deref() {
        Some("created") => SheetOrder::Created,
        Some("title") => SheetOrder::Title,
        _ => SheetOrder::Modified,
    }
}

/// Whether the app starts on a new, empty sheet and not the one last used.
pub fn new_sheet_on_launch() -> bool {
    raw("on_launch").as_deref() == Some("new")
}

/// Whether typing digits puts in the thousands separators as you go.
pub fn group_as_you_type() -> bool {
    flag("group_as_you_type", false)
}

/// Whether numbers pasted in another region's format are turned into this one's.
pub fn convert_pasted_numbers() -> bool {
    flag("paste_convert", false)
}

/// What renaming a variable does to the lines that use it.
pub fn rename_policy() -> RenamePolicy {
    match raw("rename_variables").as_deref() {
        Some("always") => RenamePolicy::Always,
        Some("never") => RenamePolicy::Never,
        _ => RenamePolicy::Ask,
    }
}

/// Whether Tab (and not Escape) finishes the name of a variable.
pub fn autocomplete_on_tab() -> bool {
    raw("autocomplete_key").as_deref() == Some("tab")
}

/// Whether a divider line starts a new set of variables.
pub fn dividers_reset_variables() -> bool {
    flag("dividers_reset", true)
}

/// The global variables, each as a line like `rent = $900`.
pub fn global_variables() -> Vec<String> {
    raw("globals")
        .unwrap_or_default()
        .split(';')
        .map(str::trim)
        .filter(|l| l.contains('='))
        .map(str::to_string)
        .collect()
}

/// A colour the settings file holds, `#rrggbb`; None for `default` or anything else.
pub fn color(key: &str) -> Option<u32> {
    let v = raw(key)?;
    let hex = v.trim().strip_prefix('#')?;
    (hex.len() == 6)
        .then(|| u32::from_str_radix(hex, 16).ok())
        .flatten()
}

/// The font the sheet is written in.
pub fn font_family() -> String {
    raw("font")
        .filter(|f| FONTS.contains(&f.as_str()))
        .unwrap_or_else(|| FONTS[0].to_string())
}

/// The fonts the Settings window offers.
pub const FONTS: &[&str] = &[
    "Noto Sans",
    "JetBrainsMono Nerd Font",
    "Liberation Sans",
    "Liberation Serif",
];

/// The size of the sheet's text, in points.
pub fn font_size() -> f32 {
    raw("font_size")
        .and_then(|v| v.parse::<f32>().ok())
        .filter(|v| (14.0..=26.0).contains(v))
        .unwrap_or(21.)
}

/// How big a heading reads against ordinary text, in percent.
pub fn heading_percent() -> u32 {
    raw("heading_size")
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|v| (100..=160).contains(v))
        .unwrap_or(100)
}

/// Whether answers sit against the right edge of their column.
pub fn answers_right() -> bool {
    raw("answer_align").as_deref() == Some("right")
}

/// Whether digits all have the same width (tabular figures).
pub fn mono_digits() -> bool {
    flag("mono_digits", false)
}
