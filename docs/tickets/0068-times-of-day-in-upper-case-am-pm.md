# 0068 Times of day print as `7:29 PM`

Status: closed
Milestone: v1
Area: display, region

The expected behaviour (region United States) prints `time in Paris` as `7:29 PM`, `6:00 PM` and `11:30 PM`, with upper-case AM and PM. The reference notes (Australian region) print `7:12 pm`. The engine prints lower case. The style follows the region, so US should be upper case; how another region would look is not known. Typed times (`7pm`, `3:30pm`) are inputs and are not affected.

Fixed: a time of day prints `7:29 PM` (upper case). Pinned by the owner replay and the regenerated oracles. A region setting for other styles is not built.
