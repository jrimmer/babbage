# 0002 Flagged approximate answer when an exact result exceeds the size cap

Status: open
Milestone: v1.1
Area: engine, precision

v1 refuses results larger than 2^20 bits (`Error::Overflow`) instead of approximating. Example: `(1+0.03/365)^36500` (100 years of daily compounding). The exact result is about 1.0M bits.

## Proposal
Retry the line in the 50-digit float path and show the answer with the approximate marker. Never do this silently: the marker is mandatory and the exact path stays the first attempt.

## Open question
Whether 50 digits is enough for compounding chains. Add a test that compares against Python Fraction to the shown digits.
