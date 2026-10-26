# 0047 Public holidays in workday calculations

Status: open
Milestone: post-v1 (region data)
Area: dates

The workdays notes excludes public holidays for the user's region: `December 24 + 2 workdays` gives `December 30`, because December 25 and 26 are holidays there. The engine counts Monday to Friday only. Holiday lists differ by country and state and change every year, so they are a shipped dataset (the same kind of data as tickets 0035 to 0038) and are not built yet. `December 24 + 2 workdays` gives `December 26` until then.
