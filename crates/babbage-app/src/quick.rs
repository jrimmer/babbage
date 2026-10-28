//! `babbage --quick`: a small window for one calculation, started from a key. When a Babbage window is already open
//! on Hyprland the key brings that one forward instead of opening a second, so pressing it twice does not stack windows.

use std::process::Command;

/// The title of the quick window, which the window rule `babbage setup omarchy` writes matches.
pub const TITLE: &str = "Babbage Quick";

/// The size of the quick window, in points.
pub const WIDTH: f32 = 760.;
pub const HEIGHT: f32 = 480.;

/// The address of the first window of `class` in the output of `hyprctl -j clients`.
pub fn find_window(clients: &str, class: &str) -> Option<String> {
    let clients: serde_json::Value = serde_json::from_str(clients).ok()?;
    clients
        .as_array()?
        .iter()
        .find(|c| c["class"] == class && c["mapped"].as_bool().unwrap_or(true))
        .and_then(|c| c["address"].as_str())
        .map(str::to_owned)
}

/// Whether what `hyprctl dispatch` printed says the dispatch worked: Hyprland says `ok`, or nothing, and a warning or
/// an error otherwise (and exits 0 for a warning).
pub fn dispatch_worked(printed: &str) -> bool {
    let low = printed.to_ascii_lowercase();
    !["error", "not found", "invalid", "unknown", "warning"]
        .iter()
        .any(|bad| low.contains(bad))
}

/// Asks Hyprland to focus the window at `address`. Hyprland 0.55 and later take Lua in a dispatch, older ones take the
/// classic form, so the Lua one is tried first and the classic one when it does not work.
fn focus_address(address: &str) -> bool {
    let tries = [
        vec![
            "dispatch".to_owned(),
            format!("hl.dsp.focus({{ window = \"address:{address}\" }})"),
        ],
        vec![
            "dispatch".to_owned(),
            "focuswindow".to_owned(),
            format!("address:{address}"),
        ],
    ];
    tries.iter().any(|args| {
        Command::new("hyprctl").args(args).output().is_ok_and(|o| {
            o.status.success()
                && dispatch_worked(&String::from_utf8_lossy(&o.stdout))
                && dispatch_worked(&String::from_utf8_lossy(&o.stderr))
        })
    })
}

/// Brings an open window of `class` forward, and says whether there was one. Only Hyprland is asked; anywhere else
/// (or when it cannot be asked) the answer is no, and the caller opens a window.
pub fn focus_running(class: &str) -> bool {
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none_or(|v| v.is_empty()) {
        return false;
    }
    let Ok(out) = Command::new("hyprctl").args(["-j", "clients"]).output() else {
        return false;
    };
    let Some(address) = find_window(&String::from_utf8_lossy(&out.stdout), class) else {
        return false;
    };
    focus_address(&address)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_open_window_of_the_class_is_found() {
        let clients = r#"[{"class":"foot","address":"0x1","mapped":true},
            {"class":"babbage","address":"0x2","mapped":false},
            {"class":"babbage","address":"0x3","mapped":true},
            {"class":"babbage","address":"0x4"}]"#;
        assert_eq!(find_window(clients, "babbage").as_deref(), Some("0x3"));
        assert_eq!(find_window(clients, "foot").as_deref(), Some("0x1"));
        assert_eq!(find_window(clients, "firefox"), None);
        assert_eq!(find_window("[]", "babbage"), None);
        assert_eq!(find_window("not json", "babbage"), None);
        assert_eq!(find_window("{}", "babbage"), None);
    }

    #[test]
    fn a_dispatch_worked_when_hyprland_says_ok_or_nothing() {
        assert!(dispatch_worked("ok"));
        assert!(dispatch_worked("ok\n"));
        assert!(dispatch_worked(""));
        assert!(!dispatch_worked(
            "warning: =[C]:-1: hl.focus: window not found"
        ));
        assert!(!dispatch_worked(
            "error: [string \"return hl.dispatch(focuswindow\"]:1: ')' expected"
        ));
        assert!(!dispatch_worked("Invalid dispatcher"));
        assert!(!dispatch_worked("Unknown"));
    }

    #[test]
    fn a_window_without_an_address_does_not_count() {
        assert_eq!(find_window(r#"[{"class":"babbage"}]"#, "babbage"), None);
    }
}
