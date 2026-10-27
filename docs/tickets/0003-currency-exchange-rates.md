# 0003 Currency conversion with live exchange rates

Status: open (post-v1: needs exchange rates, which Babbage would have to fetch; it is offline apart from the update check, see docs/updates.md)
Milestone: post-v1 (needs off-box data)
`350 EUR in USD`, `10 AUD + 20 CAD in GBP`. Needs a rates source, caching, offline behavior and a stale-rate indicator. Currency formatting itself (symbols, 2 dp) is v1 and needs no data.
