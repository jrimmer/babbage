# 0009 Income tax brackets

Status: open (post-v1: needs income tax tables, which Babbage would have to fetch; it is offline apart from the update check, see docs/updates.md)
Milestone: post-v1 (needs off-box data)
Bracket tables change every year and differ per jurisdiction. Shipping them bundled goes stale, so this needs a data update path. A user-defined bracket function (pure math) can ship in v1 without any tables.
