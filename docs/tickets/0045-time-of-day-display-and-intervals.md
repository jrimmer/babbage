# 0045 Time-of-day display and midnight rules

Status: half closed (the reference app)
Milestone: v1
Area: time

Choices made without a real install to check:
- A time of day always shows on a 12-hour clock (`7:12 pm`), even when typed as `16:00`. A 24-hour option belongs with the region settings (ticket 0027).
- `7:30am to 8:45pm` rolls over midnight (`10pm to 6am` is 8 hours); `5pm - 9pm` is the size of the gap whichever way round, so both `5pm - 3pm` and `5pm - 9pm` are positive.
- `5pm` is a time of day. A number from 1 to 12 followed by `am` or `pm` is never attometres or picometres (neither symbol is a unit here).
- An ISO-shaped string (`2019-04-01`) is a date, and a malformed one (`2019-02-30`) is an error and not a subtraction.
- A Unix timestamp from 100,000,000,000 up is read as milliseconds.
Confirm each against a real install.

The reference app: a malformed ISO date is a subtraction (ticket 0071); times of day print upper case in the US region (ticket 0068).
