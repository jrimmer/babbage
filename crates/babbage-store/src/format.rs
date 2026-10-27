//! The text format of a sheet file.

/// The file's bytes for `lines`: each line followed by `\n`. A sheet with no lines is an empty file, and
/// a sheet of one empty line is a single `\n`, so the two stay different.
///
/// A line that itself holds `\n` would turn into two lines, so the editor never produces one; here the
/// extra break simply becomes a line boundary.
pub fn encode<S: AsRef<str>>(lines: &[S]) -> String {
    let mut out = String::new();
    for l in lines {
        out.push_str(l.as_ref());
        out.push('\n');
    }
    out
}

/// The lines of a file's text. A final line without `\n` still counts, and a `\r` before the `\n` (a file
/// edited on another system) is dropped.
pub fn decode(text: &str) -> Vec<String> {
    text.split_terminator('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l).to_owned())
        .collect()
}

/// A line that starts with this and ends a file's lines is not part of the sheet's text: the app keeps how each
/// answer is written in such trailer lines (`⟦bbg line 3 dp=4⟧`).
pub const TRAILER_START: &str = "⟦bbg ";

/// How many of `lines` are the sheet's text: the trailer lines at the end are left out.
pub fn text_len<S: AsRef<str>>(lines: &[S]) -> usize {
    lines
        .iter()
        .rposition(|l| !l.as_ref().starts_with(TRAILER_START))
        .map_or(0, |i| i + 1)
}

/// The first line that holds something besides white space, trimmed.
pub fn title_of<S: AsRef<str>>(lines: &[S]) -> Option<String> {
    lines[..text_len(lines)]
        .iter()
        .map(|l| l.as_ref().trim())
        .find(|l| !l.is_empty())
        .map(str::to_owned)
}

/// How many lines the sheet has, not counting blank lines at the end (a sheet of only blank lines has
/// none, which the sidebar shows as "No lines").
pub fn count_lines<S: AsRef<str>>(lines: &[S]) -> usize {
    lines[..text_len(lines)]
        .iter()
        .rposition(|l| !l.as_ref().trim().is_empty())
        .map_or(0, |i| i + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn an_empty_sheet_and_a_sheet_of_one_empty_line_differ() {
        assert_eq!(encode::<&str>(&[]), "");
        assert_eq!(encode(&[""]), "\n");
        assert_eq!(decode(""), Vec::<String>::new());
        assert_eq!(decode("\n"), vec![""]);
    }

    #[test]
    fn a_missing_final_newline_and_crlf_are_accepted() {
        assert_eq!(decode("a\r\nb"), vec!["a", "b"]);
        assert_eq!(decode("a\n\nb\n"), vec!["a", "", "b"]);
    }

    #[test]
    fn the_title_is_the_first_non_blank_line() {
        assert_eq!(title_of(&["", "  ", " 1 + 1 "]), Some("1 + 1".to_owned()));
        assert_eq!(title_of(&["", " "]), None);
    }

    #[test]
    fn trailer_lines_are_not_part_of_the_title_or_the_count() {
        let lines = ["", "1/3", "2/3", "⟦bbg line 2 dp=4⟧"];
        assert_eq!(text_len(&lines), 3);
        assert_eq!(count_lines(&lines), 3);
        assert_eq!(title_of(&["⟦bbg line 1 dp=2⟧"]), None);
        assert_eq!(count_lines(&["⟦bbg line 1 dp=2⟧"]), 0);
        assert_eq!(title_of(&lines), Some("1/3".to_owned()));
        // A line like a trailer in the middle of the text is text.
        assert_eq!(text_len(&["⟦bbg x⟧", "a"]), 2);
    }

    #[test]
    fn trailing_blank_lines_are_not_counted() {
        assert_eq!(count_lines(&["a", "", "b", "", " "]), 3);
        assert_eq!(count_lines(&["", ""]), 0);
        assert_eq!(count_lines::<&str>(&[]), 0);
    }

    proptest! {
        #[test]
        fn arbitrary_lines_round_trip(lines in proptest::collection::vec("[^\n\r]{0,40}", 0..20)) {
            prop_assert_eq!(decode(&encode(&lines)), lines);
        }

        #[test]
        fn arbitrary_unicode_lines_round_trip(lines in proptest::collection::vec(any::<String>(), 0..10)) {
            let lines: Vec<String> = lines.into_iter().flat_map(|l| {
                l.split(['\n', '\r']).map(str::to_owned).collect::<Vec<_>>()
            }).collect();
            prop_assert_eq!(decode(&encode(&lines)), lines);
        }
    }
}
