//! Paints the line-number gutter, the text, the selection, the caret and the answer column of the open
//! sheet in one pass, so each answer sits on the same row as its line by construction.

use gpui::{
    App, Bounds, Element, ElementId, ElementInputHandler, Entity, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, Pixels, ShapedLine, SharedString, Style, TextRun,
    Window, fill, point, px, quad, relative, size,
};

use crate::syntax::{Class, spans};
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
        let (sel_a, sel_b) = view.ed.selection();
        let has_sel = view.ed.has_selection();
        let head = view.ed.head();
        let marked = view.ed.marked;

        let first = ((bounds.top() - top).max(px(0.)) / row_h).floor() as usize;
        let last = (((bounds.bottom() - top) / row_h).ceil().max(0.) as usize).min(n);

        window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
            for i in first..last {
                let y = top + row_h * i as f32;
                let line = view.ed.line(i);
                let result = view.computed.results.get(i);

                // Line number.
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
                let ny = y;
                let _ = num_line.paint(
                    point(bounds.left() + px(GUTTER_W - 16.) - num_line.width, ny),
                    row_h,
                    window,
                    cx,
                );

                // Text.
                let sp = spans(line, result);
                let line_marked = marked.filter(|m| m.line == i).map(|m| (m.start, m.end));
                let text_line = if line.is_empty() {
                    None
                } else {
                    let runs = runs_for(line, &sp, line_marked, &font, &c);
                    Some(shape(window, line, font_px, &runs))
                };
                let x_of = |col: usize| match &text_line {
                    Some(l) if col > 0 => l.x_for_index(col.min(line.len())),
                    _ => px(0.),
                };

                // Selection.
                if has_sel && i >= sel_a.line && i <= sel_b.line {
                    let from = if i == sel_a.line {
                        x_of(sel_a.col)
                    } else {
                        px(0.)
                    };
                    let to = if i == sel_b.line {
                        x_of(sel_b.col)
                    } else {
                        x_of(line.len()) + px(7.)
                    };
                    if to > from {
                        window.paint_quad(fill(
                            Bounds::new(point(text_x + from, y), size(to - from, row_h)),
                            c.selection,
                        ));
                    }
                }

                // Line references are drawn as chips behind their text.
                if let Some(l) = &text_line {
                    for s in sp.iter().filter(|s| s.class == Class::LineRef) {
                        let (x0, x1) = (l.x_for_index(s.range.start), l.x_for_index(s.range.end));
                        window.paint_quad(quad(
                            Bounds::new(
                                point(text_x + x0 - px(4.), y + px(3.)),
                                size(x1 - x0 + px(8.), row_h - px(6.)),
                            ),
                            px(6.),
                            c.chip_bg,
                            0.,
                            c.chip_bg,
                            gpui::BorderStyle::default(),
                        ));
                    }
                    let lh = row_h;
                    let ty = y;
                    let _ = l.paint(point(text_x, ty), lh, window, cx);
                }

                // Caret.
                if focused && head.line == i {
                    window.paint_quad(fill(
                        Bounds::new(
                            point(text_x + x_of(head.col) - px(1.), y + px(4.)),
                            size(px(2.), row_h - px(8.)),
                        ),
                        c.caret,
                    ));
                }

                // Answer.
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
                    if let Some(t) = &a.text {
                        let r = TextRun {
                            len: t.len(),
                            font: font.clone(),
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
                shaped[i] = text_line;
            }
        });

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
        });
    }
}
