//! Number scrubbing: hold Alt and drag across a number in the sheet, and it counts up or down by its last digit, so
//! `12.5` goes to `12.6`, `12.7`, and `9` to `10`.

/// A number the way it is typed: digits with an optional decimal part, and thousands commas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scrubbable {
    /// The value in units of the last digit: `12.50` is 1250 with 2 decimals.
    value: i128,
    decimals: usize,
    grouped: bool,
}

/// Pixels the pointer moves for each step.
pub const PIXELS_PER_STEP: f32 = 6.;

/// `1,234,567`: one to three digits, then groups of exactly three.
fn properly_grouped(whole: &str) -> bool {
    let mut parts = whole.split(',');
    let first = parts.next().unwrap_or("");
    (1..=3).contains(&first.len())
        && parts.all(|g| g.len() == 3 && g.chars().all(|c| c.is_ascii_digit()))
}

impl Scrubbable {
    /// Reads `text` if it is a plain number (no sign, exponent or suffix).
    pub fn parse(text: &str) -> Option<Scrubbable> {
        let (whole, frac) = match text.split_once('.') {
            Some((w, f)) => (w, f),
            None => (text, ""),
        };
        let grouped = whole.contains(',');
        let digits: String = whole.chars().filter(|c| *c != ',').collect();
        let plain = !digits.is_empty()
            && digits.chars().all(|c| c.is_ascii_digit())
            && frac.chars().all(|c| c.is_ascii_digit())
            && (!grouped || properly_grouped(whole))
            && digits.len() + frac.len() <= 30
            && (text.contains('.') == !frac.is_empty());
        if !plain {
            return None;
        }
        Some(Scrubbable {
            value: format!("{digits}{frac}").parse().ok()?,
            decimals: frac.len(),
            grouped,
        })
    }

    /// The number `steps` last-digits away (never below zero), written the way this one was.
    pub fn shifted(&self, steps: i64) -> String {
        let value = (self.value + i128::from(steps)).max(0);
        let digits = format!("{value:0>width$}", width = self.decimals + 1);
        let (whole, frac) = digits.split_at(digits.len() - self.decimals);
        let whole = if self.grouped {
            let mut out = String::new();
            for (i, c) in whole.chars().enumerate() {
                if i > 0 && (whole.len() - i) % 3 == 0 {
                    out.push(',');
                }
                out.push(c);
            }
            out
        } else {
            whole.to_string()
        };
        if self.decimals == 0 {
            whole
        } else {
            format!("{whole}.{frac}")
        }
    }
}

/// How many steps a pointer move of `dx` pixels makes (towards the right is up).
pub fn steps_for(dx: f32) -> i64 {
    (dx / PIXELS_PER_STEP).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shifted(text: &str, steps: i64) -> Option<String> {
        Scrubbable::parse(text).map(|s| s.shifted(steps))
    }

    #[test]
    fn a_number_counts_by_its_last_digit() {
        assert_eq!(shifted("12", 1).as_deref(), Some("13"));
        assert_eq!(shifted("12.5", 1).as_deref(), Some("12.6"));
        assert_eq!(shifted("12.5", -3).as_deref(), Some("12.2"));
        assert_eq!(shifted("0.05", 7).as_deref(), Some("0.12"));
        assert_eq!(shifted("9", 1).as_deref(), Some("10"));
        assert_eq!(shifted("12.50", 0).as_deref(), Some("12.50"));
    }

    #[test]
    fn it_never_goes_below_zero_and_keeps_its_places() {
        assert_eq!(shifted("3", -10).as_deref(), Some("0"));
        assert_eq!(shifted("0.5", -9).as_deref(), Some("0.0"));
        assert_eq!(shifted("1.05", -200).as_deref(), Some("0.00"));
    }

    #[test]
    fn thousands_commas_are_kept() {
        assert_eq!(shifted("1,234", 1).as_deref(), Some("1,235"));
        assert_eq!(shifted("999", 1).as_deref(), Some("1000"));
        assert_eq!(shifted("999,999", 1).as_deref(), Some("1,000,000"));
        assert_eq!(shifted("1,000", -1).as_deref(), Some("999"));
        assert_eq!(shifted("1,000", -1).as_deref(), Some("999"));
    }

    #[test]
    fn only_a_plain_number_is_scrubbed() {
        for text in [
            "", "abc", "1e3", "5k", "$5", ".5", "5.", "1,,2", ",5", "5,", "1.2.3", "-3", "0x10",
        ] {
            assert_eq!(Scrubbable::parse(text), None, "{text:?}");
        }
    }

    #[test]
    fn a_pointer_move_is_a_step_every_few_pixels() {
        assert_eq!(steps_for(0.), 0);
        assert_eq!(steps_for(2.9), 0);
        assert_eq!(steps_for(3.1), 1);
        assert_eq!(steps_for(60.), 10);
        assert_eq!(steps_for(-12.), -2);
    }
}
