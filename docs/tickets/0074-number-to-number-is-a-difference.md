# 0074 `1978 to 2021` is 43

Status: closed
Milestone: v1
Area: dates, numbers

The expected answer is `1978 to 2021` as `43`, a plain number with no unit. The engine refuses `to` between two plain numbers. The maintainer's page prints `42 years` for the same line; a maintainer check does not. Whether `to` between any two numbers is their difference (`10 to 25` is `15`) is not known.

Fixed for two four-digit years: `1978 to 2021` is `43`. Other `N to M` forms are untouched because the maintainer has not shown them.
