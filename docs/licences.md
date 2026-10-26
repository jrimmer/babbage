# Dependency licences

Direct dependencies of the desktop crates, from
`cargo tree --locked -e normal,dev --depth 1 -f '{p}|{l}'`. gpui brings about 680 crates of its own;
`bin/check` fails if any crate in the app's tree is copyleft-only. The only GPL mention is `self_cell`,
which is `Apache-2.0 OR GPL-2.0-only` and is used under Apache-2.0. No Zed editor crate is used: the
`zed-*` crates in the tree are published forks of font, HTTP, tar and XIM libraries (MIT or Apache-2.0)
that gpui itself depends on.

| crate | version | licence | used by |
|---|---|---|---|
| gpui | 0.2.2 | Apache-2.0 | app (pinned exactly, as the owner's other app does) |
| futures | 0.3.34 | MIT OR Apache-2.0 | app (channel between the control socket's threads and the window; already a dependency of gpui) |
| libc | 0.2.186 | MIT OR Apache-2.0 | app (`localtime_r` for the sidebar's times; pinned to 0.2.186 because gpui's xattr dependency does not build with a newer one) |
| serde_json | 1.0.154 | MIT OR Apache-2.0 | app (the control socket's replies), accept (reading them) |
| unicode-segmentation | 1.13.3 | MIT OR Apache-2.0 | app (grapheme and word steps in the editor) |
| anyhow | 1.0.104 | MIT OR Apache-2.0 | app (`main` only), accept |
| thiserror | 2.0.21 | MIT OR Apache-2.0 | store |
| tempfile | 3.27.0 | MIT OR Apache-2.0 | store (tests), app (tests), accept (the per-run work folder) |
| proptest | 1.12.0 | MIT OR Apache-2.0 | store (tests) |

The engine's own dependencies (num-bigint, num-integer, num-rational, num-traits, astro-float, and
proptest for its tests) are listed by the engine's owner.

Runtime tools of the acceptance checks, run as separate programs and never linked: Hyprland, `hyprctl`,
`wtype`, `grim`.
