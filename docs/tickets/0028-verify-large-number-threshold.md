# 0028 Verify when results abbreviate to k/M/G/T

Status: closed (settled by a maintainer check)
Milestone: v1
Area: display

The engine abbreviates from 100,000. Evidence: `300,000` shows `300k`, `16,384` and `12,288` do not, `64,000,000` shows `64M`. The exact threshold, and whether percentages ever abbreviate, are unverified.

Settled by a maintainer check: `1_000_000 + 2_000` is `1.002M`, `0xCAFE_F00D to decimal` is `3.405705229G` and `1,700,000` is `1.7M`, as the engine prints. The page's `1,002,000` and `3.406G` are not what the expected behaviour prints.
