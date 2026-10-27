# Dates and times

Type a date, a time of day or a length of time the way you would say it, and Babbage does the calendar arithmetic: days between two dates, a date plus three weeks, the weekday of Easter, nine o'clock in London as seen from Tokyo.

![Easter and workday arithmetic, a gap between two dates, and two time zone conversions](img/dates-holidays.webp)

Dates and times follow your machine's clock and time zone. The examples in this chapter use explicit dates so they come out the same whenever you read them. Where a date is in the current year the answer leaves the year off (`June 17`), and in any other year it shows it (`June 17, 2030`).

## Writing a date

All of these are the same day:

| You type | Notes |
|---|---|
| `June 17, 2030` | The usual form. |
| `Jun 17 2030` | Three-letter month, comma optional. |
| `June 17th, 2030` | `st`, `nd`, `rd`, `th` are accepted. |
| `17 June 2030` | Day first. |
| `2030-06-17` | Year, month, day. |
| `6/17/2030` | Month first by default. |

A date with a slash is read month first (`12/02/1988` is December 2). Turn on "Day first in 12/02/1988" in Settings, Dates & Times and it is read as 12 February. See [Settings](12-settings.html#dates-times).

A date with no year (`June 17`) is this year's. Words work as dates too:

| Word | Means |
|---|---|
| `today`, `tomorrow`, `yesterday` | Relative to the clock. |
| `Monday` | The next Monday after today. |
| `next Friday`, `last Friday`, `this Friday` | That weekday, forwards, backwards, or the coming one. |
| `Tuesday next week` | Also `last week` and `this week`; weeks start on Sunday. |
| `now` | The time of day, to the minute. |

A date followed by a time makes one moment: `June 17, 2030 at 3:30pm` and `June 17, 2030 3:30pm` both answer `June 17, 2030 at 3:30 PM`. Answers that fall on the day before or after today read `Yesterday at` or `Tomorrow at`.

## Holidays

Name a holiday and Babbage finds the day. Add a year for a particular one, or leave it off for the nearest this year.

| You type | Answer |
|---|---|
| `Easter 2030` | `April 21, 2030` |
| `Good Friday 2030` | `April 19, 2030` |
| `Easter Monday 2030` | `April 22, 2030` |
| `Orthodox Easter 2030` | `April 28, 2030` |
| `Christmas 2030` | `December 25, 2030` |
| `Boxing Day 2030` | `December 26, 2030` |
| `Thanksgiving 2030` | `November 28, 2030` |
| `Black Friday 2030` | `November 29, 2030` |
| `Halloween 2030` | `October 31, 2030` |
| `Valentine's Day 2030` | `February 14, 2030` |
| `New Year's Eve 2030` | `December 31, 2030` |
| `Chinese New Year 2030` | `February 3, 2030` |
| `Ramadan 2030` | `January 5, 2030` |
| `Hanukkah 2030` | `December 20, 2030` |

Also understood: `Christmas Eve`, `New Year's Day`, `Holy Saturday`, `Easter Sunday`, `Orthodox Christmas`, `Orthodox Good Friday`, `End of Ramadan`, `End of Hanukkah` and `Chinese New Year's Eve`. A holiday is an ordinary date, so you can add to it (`Easter 2030 + 1 week` is `April 28, 2030`) or ask its weekday (`weekday of Easter 2030` is `Sunday`).

## Date plus or minus a span

Add or subtract days, weeks, months or years:

```
June 17, 2030 + 30 days
June 17, 2030 - 2 weeks
June 17, 2030 + 1 year 2 months
2 weeks from June 17, 2030
10 days after January 1, 2030
2 days before Christmas 2030
```

The first gives `July 17, 2030`, and the next `June 3, 2030`, `August 17, 2031`, `July 1, 2030`, `January 11, 2030` and `December 23, 2030`.

Days, weeks and seconds are exact. Months and years follow the calendar and the day clamps at the end of a month: `Jan 31, 2030 + 1 month` is `February 28, 2030`. Several spans in a row apply in turn (`2 months 3 days after June 5, 2030`).

Going back in time works with `ago`: `5 days ago` and `3 months ago` give a date and time.

## The span between dates

Subtract two dates, or join them with `to`, to get the gap as weeks and days:

```
June 17, 2030 - June 1, 2030
June 1, 2030 to June 17, 2030
```

Both give `2 weeks 2 days`. Reverse them with `-` and the answer is negative: `June 1, 2030 - June 17, 2030` is `-2 weeks 2 days`. Ask for one unit by adding `in days`:

| You type | Answer |
|---|---|
| `January 1, 2030 to December 31, 2030` | `11 months 4 weeks 2 days` |
| `January 1, 2030 to December 31, 2030 in days` | `364 days` |
| `days between June 1, 2030 and July 4, 2030` | `33 days` |
| `April 1, 2030 through April 30, 2030` | `30 days` (both ends counted) |
| `time until December 25, 2030` | The calendar gap from now. |
| `time since January 1, 2030` | The calendar gap since then. |

Two dates with no year, written alone on a line, subtract to the span forward from the first. Dates are also counted from the clock: `days until Christmas`, `days since January 1`, `days until June 15, 2030` and `days left in 2030` count from today.

### Parts of a date

| You type | Answer |
|---|---|
| `day of the week on March 12, 2023` | `Sunday` |
| `what day is June 17, 2030` | `Monday` |
| `week number of June 17, 2024` | `25` |
| `day number on June 17, 2024` | `169` (the day of the year) |
| `days in February 2024` | `29 days` |
| `days in Q3 2024` | `92 days` |
| `days in 2024` | `366 days` |
| `year percentage` | How much of this year has gone. Also `month`, `week` and `day`. |

## Workdays

A workday is Monday to Friday. A span of workdays counts the first day and not the last.

| You type | Answer |
|---|---|
| `June 17, 2030 + 10 workdays` | `July 1, 2030` |
| `workdays between June 1, 2024 and July 4, 2024` | `23 workdays` |
| `work days from June 3, 2024 to June 28, 2024` | `19 workdays` |
| `workdays in June 2024` | `20 workdays` |
| `workdays in 3 weeks` | `15 workdays` |
| `workdays until Christmas` | Workdays from today. |
| `work hours between June 3, 2024 and June 7, 2024` | `32 hours` |
| `55 hours in workdays` | `6.875 workdays` |

Once a workday is in the line, a week counts as five workdays. Work hours multiply workdays by the length of your working day, 8 hours unless you change "Hours in a working day" in Settings, Dates & Times.

Public holidays are not guessed. List the days you do not work in Settings, Dates & Times, under "Public holidays": `2026-12-25` for a single date, `12-25` for the same day every year, separated with `;`. Workday arithmetic then skips them.

## Times of day

| You type | Answer |
|---|---|
| `3:30 pm` | `3:30 PM` |
| `15:45` | `3:45 PM` |
| `noon`, `midnight` | `12:00 PM`, `12:00 AM` |
| `9:15am + 45 min` | `10:00 AM` |
| `10pm - 3 hours` | `7:00 PM` |
| `noon + 14 hours` | `Tomorrow at 2:00 AM` |
| `7:30am to 8:45pm` | `13 hours 15 min` |
| `10pm to 6am` | `8 hours` |
| `5pm - 9am` | `8 hours` |

A time of day on its own never shows seconds, so `3:45:30 pm` reads `3:45 PM`. A date with a time does: `December 10, 2030 at 9:31:23 AM`. A gap between two times that crosses midnight rolls over, as `10pm to 6am` shows.

## Lengths of time

Write a length as parts or as one number with a unit:

```
3 hours 20 minutes
7 hours 45 min + 2 hours 30 min
90 minutes in hours
2 days 3 hours in hours
8 hours / 3
```

These answer `3 hours 20 min`, `10 hours 15 min`, `1.5 hours`, `51 hours` and `2.6666666667 hours`. Add `as timespan` to spell a decimal out in parts (`1.5 hours as timespan` is `1 hour 30 min`) and `in minutes` or any other unit to convert.

Three colons write a lap time, hours first: `00:01:30` is a minute and a half, and two lap times add (`03:04:05 + 00:00:55` is `03:05:00`). Speed changes a length: `1 hour 30 minutes at 1.5x` is `1 hour`, and `time saved 5 min at 1.5x` is `1 min 40 s`.

## Time zones

Say a time, the place it is in, and where you want it:

```
9am London in Tokyo
6pm Sydney in Chicago
8:00 Los Angeles to London
9am in London
```

The first answers `5:00 PM`. Using `to` instead of `in` is the same. Leave the first place off and the time is read in your own zone. Add a date and the date comes along: `June 17, 2030 3:30pm New York in Tokyo` is `June 18, 2030 at 4:30 AM`.

Other forms:

| You type | Gives |
|---|---|
| `time in Tokyo` or `Tokyo time` | The time there now. |
| `date in Paris` | The date there now. |
| `time in Tokyo when it is 9am in London` | `5:00 PM` |
| `time difference between Sydney and Chicago` | How far the second place's clock is ahead of the first: `-15 hours`. |
| `now in Tokyo` | The current time there. |

Places Babbage knows:

- Cities (`Tokyo`, `New York`, `Los Angeles`).
- Countries (`Japan`, `Germany`), meaning the capital's zone.
- Regions: `Eastern time`, `Central time`, `Mountain time`, `Pacific time`, `Alaska time`, `Hawaii time`, `Atlantic time`.
- Airport codes such as `LAX`.
- Abbreviations. The US ones (`EST`, `PST`, `CST` and the daylight forms) mean the whole region with its summer time, so `8am PST in Berlin` follows the season. Others are fixed offsets: `CET`, `JST`, `IST` (India), `AEST` and so on.
- An offset: `GMT+8`, `UTC-5:30`.
- Your own names, from Settings, Calculator, "Your own names for places": `HQ = Europe/Berlin; Lab = Asia/Tokyo`.

An answer in UTC or GMT shows on the 24-hour clock (`3pm EST to GMT` is `19:00` in summer). A zone-aware time is a whole-line operation: you cannot add to it or compare it afterwards.

## Formats

End a date with `as` and a pattern to choose how it prints:

| You type | Answer |
|---|---|
| `June 17, 2030 as EEEE, MMM d, yyyy` | `Monday, Jun 17, 2030` |
| `June 17, 2030 as dd/MM/yyyy` | `17/06/2030` |
| `June 17, 2030 3:30pm as yyyy-MM-dd HH:mm` | `2030-06-17 15:30` |
| `June 17, 2030 3:30pm as h:mm a` | `3:30 PM` |
| `June 17, 2030 as EEE` | `Mon` |
| `June 17, 2030 as iso8601` | `2030-06-17T00:00:00Z` |

## Timestamps and timecodes

| You type | Answer |
|---|---|
| `1718600000 to date` | `June 17, 2024 at 4:53:20 AM UTC` |
| `1718600000000 to date` | The same (a number of 100,000,000,000 or more is read as milliseconds). |
| `June 17, 2024 to timestamp` | `1718625600` (a date alone is taken at noon). |
| `current timestamp` | Seconds since 1970 now. |
| `90 seconds as timecode` | `00:01:30:00` |
| `1500 frames to timecode` | `00:01:02:12` (24 frames a second) |
| `1500 frames to timecode at 30 fps` | `00:00:50:00` |
| `1 hour in frames at 24 fps` | `86,400 frames` |

A timecode has three colons: hours, minutes, seconds and frames.
