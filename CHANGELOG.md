# Changelog

## 1.0.0 (draft, not released)

Babbage is a notepad calculator for Linux. You write lines of plain language and numbers in a sheet and the answer
appears beside each line.

- **Calculating.** Arithmetic that stays exact until it cannot be, with the ≈ mark when an answer is rounded.
  Percentages in every common form, more than 200 units with exact conversions, money in many currencies, dates,
  times, durations, workdays and time zones, roots, logarithms, trigonometry, statistics, loans and growth.
- **Sheets.** Names for values, references to earlier lines, subtotals, tags and a running Total. Sheets are plain
  `.bbg` text files in your own folder, with a trash you can recover from (Ctrl+Shift+T).
- **Answers.** Each answer has a menu: decimal places, thousands separators, large-number notation, fixed money
  digits and repeating decimals (`0.(3)`), kept with the line in the sheet.
- **Words that were not understood** are underlined, and a word one letter from a unit shows what it may mean.
- **The command line.** `bbg 20% of 85` answers from a terminal, reads sheets from a file or a pipe, and prints
  JSON. It is the same program as the window, so the answers always agree.
- **Install.** One line installs the program under `~/.local`, checking every file against its published SHA-256.
- **Updates.** Babbage can look for a newer release once a week and says exactly what it asks; one setting or one
  environment variable turns it off.
- **A user guide** of fourteen chapters with pictures from the app.
