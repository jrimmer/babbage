//! How the sidebar prints a sheet's modified time: "8:37 PM" today, "Oct 20 at 10:22 AM" otherwise.

use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Local {
    pub year: i32,
    pub month0: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
}

/// The local calendar time of `t`, through the C library so the user's time zone applies.
pub fn local(t: SystemTime) -> Option<Local> {
    let secs = t.duration_since(UNIX_EPOCH).ok()?.as_secs() as libc::time_t;
    // SAFETY: `tm` is plain data that localtime_r fills in; both pointers are valid for the call.
    let tm = unsafe {
        let mut tm: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&secs, &mut tm).is_null() {
            return None;
        }
        tm
    };
    Some(Local {
        year: tm.tm_year + 1900,
        month0: tm.tm_mon as u32,
        day: tm.tm_mday as u32,
        hour: tm.tm_hour as u32,
        minute: tm.tm_min as u32,
    })
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn clock(l: &Local) -> String {
    let h12 = match l.hour % 12 {
        0 => 12,
        h => h,
    };
    let ampm = if l.hour < 12 { "AM" } else { "PM" };
    format!("{h12}:{:02} {ampm}", l.minute)
}

pub fn describe(then: Local, now: Local) -> String {
    if then.year == now.year && then.month0 == now.month0 && then.day == now.day {
        clock(&then)
    } else {
        format!(
            "{} {} at {}",
            MONTHS[then.month0 as usize % 12],
            then.day,
            clock(&then)
        )
    }
}

pub fn describe_time(t: SystemTime) -> String {
    match (local(t), local(SystemTime::now())) {
        (Some(then), Some(now)) => describe(then, now),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(month0: u32, day: u32, hour: u32, minute: u32) -> Local {
        Local {
            year: 2026,
            month0,
            day,
            hour,
            minute,
        }
    }

    #[test]
    fn today_shows_only_the_clock() {
        assert_eq!(describe(at(9, 25, 20, 37), at(9, 25, 22, 0)), "8:37 PM");
        assert_eq!(describe(at(9, 25, 0, 5), at(9, 25, 22, 0)), "12:05 AM");
        assert_eq!(describe(at(9, 25, 12, 0), at(9, 25, 22, 0)), "12:00 PM");
    }

    #[test]
    fn another_day_shows_month_and_day() {
        assert_eq!(
            describe(at(9, 20, 10, 22), at(9, 25, 22, 0)),
            "Oct 20 at 10:22 AM"
        );
    }

    #[test]
    fn the_same_day_of_another_year_is_not_today() {
        let mut then = at(9, 25, 8, 0);
        then.year = 2025;
        assert!(describe(then, at(9, 25, 9, 0)).starts_with("Oct 25 at"));
    }

    #[test]
    fn the_local_time_of_now_is_plausible() {
        let l = local(SystemTime::now()).unwrap();
        assert!(l.year >= 2025 && l.month0 < 12 && l.hour < 24 && l.minute < 60);
    }
}
