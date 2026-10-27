# 0002 Flagged approximate answer when an exact result exceeds the size cap

Status: closed (built)
Milestone: v1.1
Area: engine, precision

v1 refuses results larger than 2^20 bits (`Error::Overflow`) instead of approximating. Example: `(1+0.03/365)^36500` (100 years of daily compounding). The exact result is about 1.0M bits.

## Proposal
Retry the line in the 50-digit float path and show the answer with the approximate marker. Never do this silently: the marker is mandatory and the exact path stays the first attempt.

## Open question
Whether 50 digits is enough for compounding chains. Add a test that compares against Python Fraction to the shown digits.

Audit 2026-10-26: a power too large to hold exactly now falls back to about 115 digits and is marked approximate (`approx::pow_large`; `(1+0.03/365)^36500` is 20.0830609124, checked against a Python `Fraction` power to 50 digits in `tests/large_powers.rs`, mutation clean). An exponent past 400,000 and a result too large to write down are still refused. Sheet G asks what the reference app shows for the line.

Sheet G: The reference app prints `(1+0.03/365)^36500` as `20.0830609125`; the true value is 20.08306091241977..., so Babbage's `20.0830609124` is right and the difference is deliberate.
