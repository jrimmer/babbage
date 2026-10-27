# 0032 The constant e, and `2pi`

Status: closed (settled by a maintainer check)
Milestone: v1
Area: syntax

The trigonometry notes lists pi, tau, phi but no `e`. `2pi` (implicit multiplication) is not documented either. Both are refused today.

Sheet G (the expected behaviour, 2026-10-26): `2pi` and `2 pi` are `6.2831853072`; built (also `2π`, `3 tau`, `5 phi`). `e` is not a constant in the expected behaviour (`e` and `e^2` show nothing), so Babbage has none either. Also from a side agent's note: `sin(2pi)` and the other trig of a multiple of the approximate pi now show `0`, not `-3.8e-116`.
