# 0050 What the floating Total shows for dollars, plain numbers and volumes together

Status: open
Milestone: v1
Area: sheets

The reference sheet is 78 lines long, so `Total 95,3...` includes lines that are not visible, and the 19 visible answers (plain numbers, dollars, volumes) cannot be checked against it. The engine adds the lines it can and refuses a sheet that mixes kinds (`these units measure different things`), and the app shows that message in the chip. The reference app may add the bare numbers of mixed lines, or only some kinds. Check a real install with a small mixed sheet before changing the rule.
