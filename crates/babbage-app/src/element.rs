//! Paints the line-number gutter, the text, the selection, the caret and the answer column of the open
//! sheet in one pass, so each answer sits on the same row as its line by construction.

use gpui::{
    App, Bounds, Element, ElementId, ElementInputHandler, Entity, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, Pixels, ShapedLine, SharedString, Style, TextRun,
    Window, fill, point, px, quad, relative, size,
};

use crate::shown::{Shown, shown_for};
use crate::syntax::{Class, spans, spans_in};
use crate::theme::Colors;
use crate::view::{FONT_PX, GUTTER_W, Layout, ROW_H, SheetView, TOP_PAD, answer_min_w, runs_for};

pub struct SheetElement {
    pub view: Entity<SheetView>,
}

impl IntoElement for SheetElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

fn shape(window: &mut Window, text: &str, size: Pixels, runs: &[TextRun]) -> ShapedLine {
    window
        .text_system()
        .shape_line(SharedString::from(text.to_owned()), size, runs, None)
}

/// `text` cut and ended with an ellipsis until it is no wider than `max`.
fn fit(window: &mut Window, text: &str, size: Pixels, run: &TextRun, max: Pixels) -> ShapedLine {
    let one = |t: &str, window: &mut Window| {
        let mut r = run.clone();
        r.len = t.len();
        shape(window, t, size, &[r])
    };
    let full = one(text, window);
    if full.width <= max {
        return full;
    }
    let mut chars: Vec<char> = text.chars().collect();
    while chars.len() > 1 {
        chars.pop();
        let cut: String = chars.iter().collect::<String>() + "…";
        let s = one(&cut, window);
        if s.width <= max {
            return s;
        }
    }
    one("…", window)
}

impl Element for SheetElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut Window,
        _: &mut App,
    ) {
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let entity = self.view.clone();
        // Updating the view lends it and the app together, so painting can use both.
        entity.update(cx, |view, cx| Self::paint_view(view, bounds, window, cx));
    }
}

impl SheetElement {
    fn paint_view(
        view: &mut SheetView,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut gpui::Context<SheetView>,
    ) {
        let c = *cx.global::<Colors>();
        let style = window.text_style();
        let font = style.font();
        let font_px = px(FONT_PX);
        let small = px(13.);
        let row_h = px(ROW_H);
        let focus = view.focus_handle_ref().clone();
        let focused = focus.is_focused(window);

        // Columns: gutter, text, answers.
        let answer_w = px(answer_min_w().max(f32::from(bounds.size.width) * 0.3));
        let answer_x = bounds.right() - answer_w;
        let text_x = bounds.left() + px(GUTTER_W);
        let text_w = (answer_x - text_x - px(28.)).max(px(crate::wrap::MIN_WIDTH));

        // Where each line breaks into rows at this width, then the scroll that fits the new height.
        {
            let (ed, computed, wrap) = (&view.ed, &view.computed, &mut view.wrap);
            wrap.refresh(
                ed.len(),
                f32::from(text_w),
                |i| shown_for(ed.line(i), computed.results.get(i), i, &computed.answers).text,
                |i, text, width| {
                    let sp = spans(text, computed.results.get(i));
                    let runs = runs_for(text, &sp, None, &font, &c);
                    let l = shape(window, text, font_px, &runs);
                    crate::wrap::breaks(text, |b| f32::from(l.x_for_index(b)), width)
                },
            );
        }
        view.settle_scroll();
        let top = bounds.top() + px(TOP_PAD) - px(view.scroll());

        window.paint_quad(fill(
            Bounds::new(
                point(answer_x, bounds.top()),
                size(answer_w, bounds.size.height),
            ),
            c.answers,
        ));
        window.paint_quad(fill(
            Bounds::new(
                point(answer_x, bounds.top()),
                size(px(1.), bounds.size.height),
            ),
            c.line,
        ));

        let n = view.ed.len();
        let mut shaped: Vec<Option<ShapedLine>> = vec![None; n];
        let mut drawn: Vec<(usize, Option<ShapedLine>)> = Vec::new();
        let (sel_a, sel_b) = view.ed.selection();
        let has_sel = view.ed.has_selection();
        let head = view.ed.head();
        let marked = view.ed.marked;

        let first_row = ((bounds.top() - top).max(px(0.)) / row_h).floor() as usize;
        let first = view.wrap.locate(first_row, n).0;

        window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
            for i in first..n {
                let y = top + row_h * view.wrap.top_row(i) as f32;
                if y > bounds.bottom() {
                    break;
                }
                let line = view.ed.line(i);
                let result = view.computed.results.get(i);
                let rows = view.wrap.rows_of(i);

                // Line number, on the first row.
                let num = (i + 1).to_string();
                let num_run = TextRun {
                    len: num.len(),
                    font: font.clone(),
                    color: c.gutter,
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                let num_line = shape(window, &num, small, &[num_run]);
                let _ = num_line.paint(
                    point(bounds.left() + px(GUTTER_W - 16.) - num_line.width, y),
                    row_h,
                    window,
                    cx,
                );

                // Text, as shown: a reference to another line is that line's answer in a chip.
                let names: Vec<String> = view
                    .computed
                    .names
                    .get(i)
                    .map(|v| v.iter().map(|(n, _)| n.clone()).collect())
                    .unwrap_or_default();
                let typed_spans = spans_in(line, result, &names);
                let sh: Shown = Shown::build(line, &typed_spans, i, &view.computed.answers);
                let sp = sh.spans(&typed_spans);
                let shown_text = sh.text.as_str();
                let line_marked = marked
                    .filter(|m| m.line == i)
                    .map(|m| (sh.to_shown(m.start), sh.to_shown(m.end)));
                let text_line = if shown_text.is_empty() {
                    None
                } else {
                    let runs = runs_for(shown_text, &sp, line_marked, &font, &c);
                    Some(shape(window, shown_text, font_px, &runs))
                };
                // `x_of` takes a column of the shown text.
                let x_of = |col: usize| match &text_line {
                    Some(l) if col > 0 => l.x_for_index(col.min(shown_text.len())),
                    _ => px(0.),
                };
                let (sel_from, sel_to) = (sh.to_shown(sel_a.col), sh.to_shown(sel_b.col));
                let caret_col = sh.to_shown(head.col);

                for k in 0..rows {
                    let row_y = y + row_h * k as f32;
                    let (start, end) = view.wrap.row_span(i, k, shown_text.len());
                    let x0 = x_of(start);
                    let last_row = k + 1 == rows;

                    // Selection.
                    if has_sel && i >= sel_a.line && i <= sel_b.line {
                        let from_col = if i == sel_a.line { sel_from } else { 0 };
                        let to_col = if i == sel_b.line {
                            sel_to
                        } else {
                            shown_text.len()
                        };
                        let (a, b) = (from_col.max(start), to_col.min(end));
                        let carries_on = last_row && i < sel_b.line;
                        let from = if a >= b && !carries_on {
                            px(0.)
                        } else {
                            x_of(a) - x0
                        };
                        let to = x_of(b.max(a)) - x0 + if carries_on { px(7.) } else { px(0.) };
                        if to > from {
                            window.paint_quad(fill(
                                Bounds::new(point(text_x + from, row_y), size(to - from, row_h)),
                                c.selection,
                            ));
                        }
                    }

                    // Line references are drawn as chips behind their text.
                    if let Some(l) = &text_line {
                        for s in sp.iter().filter(|s| s.class == Class::LineRef) {
                            let (a, b) = (s.range.start.max(start), s.range.end.min(end));
                            if a >= b {
                                continue;
                            }
                            let (xa, xb) = (l.x_for_index(a) - x0, l.x_for_index(b) - x0);
                            window.paint_quad(quad(
                                Bounds::new(
                                    point(text_x + xa - px(4.), row_y + px(3.)),
                                    size(xb - xa + px(8.), row_h - px(6.)),
                                ),
                                px(6.),
                                c.chip_bg,
                                0.,
                                c.chip_bg,
                                gpui::BorderStyle::default(),
                            ));
                        }
                        // The whole styled line is painted once per row, clipped to that row.
                        let clip = Bounds::new(
                            point(text_x - px(6.), row_y),
                            size(text_w + px(12.), row_h),
                        );
                        window.with_content_mask(
                            Some(gpui::ContentMask { bounds: clip }),
                            |window| {
                                let _ = l.paint(point(text_x - x0, row_y), row_h, window, cx);
                            },
                        );
                    }

                    // Caret: on the row that holds it, and at the end of the line on the last row.
                    let holds_caret = head.line == i
                        && (if last_row {
                            caret_col >= start
                        } else {
                            caret_col >= start && caret_col < end
                        });
                    if focused && holds_caret {
                        window.paint_quad(fill(
                            Bounds::new(
                                point(text_x + x_of(caret_col) - x0 - px(1.), row_y + px(4.)),
                                size(px(2.), row_h - px(8.)),
                            ),
                            c.caret,
                        ));
                    }
                }

                // The `⋯` button on the line the pointer is over (or whose menu is open).
                if (view.hover_line == Some(i) || view.answer_menu == Some(i))
                    && view.answer_for(i).is_some_and(|a| a.text.is_some())
                {
                    let r = px(crate::answer_menu::MORE_RADIUS);
                    let cx0 = bounds.right() - px(crate::answer_menu::MORE_FROM_RIGHT);
                    let cy0 = y + row_h / 2.;
                    window.paint_quad(quad(
                        Bounds::new(point(cx0 - r, cy0 - r), size(r * 2., r * 2.)),
                        r,
                        c.raised,
                        1.,
                        c.line,
                        gpui::BorderStyle::default(),
                    ));
                    for dx in [-6., 0., 6.] {
                        window.paint_quad(fill(
                            Bounds::new(
                                point(cx0 + px(dx) - px(1.5), cy0 - px(1.5)),
                                size(px(3.), px(3.)),
                            ),
                            c.muted,
                        ));
                    }
                }

                // Answer, on the first row.
                if let Some(a) = view.answer_for(i) {
                    let ax = answer_x + px(30.);
                    let max = bounds.right() - ax - px(16.);
                    if view.answer_focus == Some(i) {
                        window.paint_quad(quad(
                            Bounds::new(
                                point(answer_x + px(8.), y + px(2.)),
                                size(answer_w - px(16.), row_h - px(4.)),
                            ),
                            px(6.),
                            c.selection,
                            1.,
                            c.caret,
                            gpui::BorderStyle::default(),
                        ));
                    }
                    let lh = row_h;
                    let ty = y;
                    // A subtotal is bold, under a rule that runs across the answer column.
                    let subtotal = result.is_some_and(|r| r.subtotal);
                    if subtotal {
                        window.paint_quad(fill(
                            Bounds::new(
                                point(answer_x + px(12.), y + px(1.)),
                                size(answer_w - px(24.), px(1.)),
                            ),
                            c.gutter,
                        ));
                    }
                    if let Some(t) = &a.text {
                        let mut bold = font.clone();
                        if subtotal {
                            bold.weight = gpui::FontWeight::BOLD;
                        }
                        let r = TextRun {
                            len: t.len(),
                            font: bold,
                            color: c.text,
                            background_color: None,
                            underline: None,
                            strikethrough: None,
                        };
                        let l = fit(window, t, font_px, &r, max);
                        let _ = l.paint(point(ax, ty), lh, window, cx);
                        if !a.exact {
                            let r = TextRun {
                                len: "≈".len(),
                                color: c.muted,
                                ..r
                            };
                            let l = shape(window, "≈", small, &[r]);
                            let _ = l.paint(point(answer_x + px(12.), ty), lh, window, cx);
                        }
                    } else if let Some(e) = &a.error {
                        let r = TextRun {
                            len: e.len(),
                            font: font.clone(),
                            color: c.warn,
                            background_color: None,
                            underline: None,
                            strikethrough: None,
                        };
                        let l = fit(window, e, px(14.), &r, max);
                        let _ = l.paint(point(ax, ty), lh, window, cx);
                    }
                }
                drawn.push((i, text_line));
            }
        });

        for (i, l) in drawn {
            shaped[i] = l;
        }
        window.handle_input(&focus, ElementInputHandler::new(bounds, cx.entity()), cx);
        let scale = window.scale_factor();
        view.set_layout(Layout {
            bounds,
            text_x,
            answer_x,
            answer_w,
            top,
            shaped,
            scale,
            text_w,
        });
    }
}
