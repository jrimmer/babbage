# 0001 Symbolic results where possible

Status: open
Milestone: post-v1
Area: engine, precision

Keep irrational quantities symbolic so identities hold exactly: `sqrt 2 × sqrt 2 = 2`, `π / π = 1`, `e^ln 5 = 5`, `sin(π/6) = 1/2`.

## Why
v1 computes irrationals to 50 digits and flags the answer as approximate (see decisions.md). That is correct but loses identities. A symbolic layer would return exact answers for the cases a person can check by hand and fall back to the 50-digit path otherwise.

## Scope notes
- Value domain grows from `Rational` to `Rational + radicals + named constants (π, e)` with a canonical form so equality is decidable.
- Start with square roots and π multiples, then trig at rational multiples of π.
- Needs a normalizer property test: `simplify(x) == simplify(simplify(x))` and numeric agreement with the 50-digit path to 40 digits.

## Depends on
v1 arbitrary-precision path (astro-float) as the reference oracle.
