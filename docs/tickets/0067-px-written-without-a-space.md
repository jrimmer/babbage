# 0067 `px` is written with no space after the number

Status: closed
Milestone: v1
Area: display

The expected answer is `1 cm in px at 326 ppi` as `128.3464566929px` (checklist line A6), with no space, where the engine prints `128.3464566929 px`. Only this one line was checked; whether `px` always attaches, or other units do too (`%`, `°`), is not known. See the decisions log.

Fixed: `px` is written with no space (`128.3464566929px`). Only `px` was observed; other units are unchanged.
