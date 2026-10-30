# 0091 The sidebar and the answers column cannot be resized

Status: open
Milestone: 1.1
Area: app

Found by the maintainer on 2026-10-30: neither separator can be dragged. The sidebar is a fixed 236 pt (`SIDEBAR_W` in `view.rs`), and the answers column is 30% of the pane with a 300 pt minimum.

Sizing, from reading the layout code: the sidebar is one constant used in one place, so about an hour (a drag handle, a clamp of roughly 160 to 400 pt, a saved width). The answers column is the same rule in three places that must agree, the drawing code (`element.rs`), the toolbar strip (`chrome.rs`) and the Total chip's width cap (`view.rs`), so they move to one function first; a drag also changes where every answer is drawn and where lines wrap. About two hours. Each separator needs a real-pointer drag check in the nested compositor, and the defaults must not move, because the existing window checks aim at fixed positions.
