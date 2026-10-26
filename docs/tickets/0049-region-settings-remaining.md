# 0049 Region settings still to build

Status: half closed
Milestone: v1 (row 22)
Area: settings

Built: number format (three regions), underscore separators, the user's time zone, the work-hours setting, the region dollar (USD, AUD, CAD, NZD, SGD, HKD; `$` shows for the region's own dollar and `US$` for US dollars elsewhere) and US customary or imperial gallons, quarts, pints and fluid ounces. Still open: a metric volume system (its pint, quart and gallon values are not in the material available), and the rest of the dollar family (MXN, ARS and others). Also not covered: a leading decimal comma (`,5`) in Western Europe, Indian lakh and crore grouping, and a settings screen in the app (today a settings file is read).

Built (owner chose "formal imperial", 2026-10-26): with `volume_system = imperial` the tablespoon is 5/8 of an imperial fluid ounce (17.7581640625 mL) and the teaspoon a third of that (5.9193880208 mL); the cup stays US customary. Pinned in `tests/region_settings.rs`.

To confirm against a real reference app (owner offered): set its volume units to imperial (the wording of the setting may differ), then type these lines and send what it shows, or tell me which differ:

```
1 tbsp in mL        (expected 17.7581640625 mL)
1 tsp in mL         (expected 5.9193880208 mL)
1 fl oz in tbsp     (expected 1.6 tbsp)
1 cup in mL         (expected 236.5882365 mL; if it shows 284.1306 mL the cup changes too)
1 pint in mL        (expected 568.26125 mL)
```

If the reference app differs, the constants live in `factor_now` in `crates/babbage-core/src/units.rs`.
