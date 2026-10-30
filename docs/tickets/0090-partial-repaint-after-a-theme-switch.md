# 0090 Window repaints only part of itself after an Omarchy theme switch

Status: open
Milestone: v1
Area: app

Found by the maintainer on the real desktop (test sheet, tests 4 to 6): after `omarchy theme set "Catppuccin Latte"` the open window turns light and paints correctly for a few seconds, then only its top-left tenth is painted (once only the "Babbage + Sheet" strip of the sidebar). Moving the window repaints it fully. Switching back to a dark theme needed the same push. Foot did almost the same thing; Ghostty and another app did not.

Not reproduced in the nested Hyprland (`tools/nested-hyprland.sh`): five swaps of the theme file and a `hyprctl reload` leave every screenshot fully painted, and the `light-theme` acceptance check passes. So it needs something the nested compositor lacks: the real GPU, Omarchy's own theme switch (which also reloads Hyprland, restarts the bar and rewrites terminal configs), or Hyprland's animations and blur.

What the app does on a switch: a task polls the theme file's modified time once a second and, when the colors differ, sets the global and calls `refresh_windows`. Nothing else repaints.

To find out: does it also happen with the plain binary (not the AppImage), with `BABBAGE_THEME_FILE` pointing at a file that is rewritten by hand (no Omarchy switch), and with Hyprland's animations off (`hyprctl keyword animations:enabled 0`)? The answer separates a Babbage drawing bug from an effect of the switch itself.
