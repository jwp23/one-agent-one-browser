use crate::dom::{Element, Node};
use crate::geom::{Rect, Size};
use crate::render::{DisplayCommand, DrawRect, DrawText, FontMetricsPx, LinkHitRegion, TextStyle};
use crate::style::{ComputedStyle, Display, TextAlign, Visibility};
use std::rc::Rc;

use super::LayoutEngine;

#[derive(Clone, Debug)]
enum InlineToken {
    Word(String, TextStyle, bool, Option<Rc<str>>),
    Space(TextStyle, bool, Option<Rc<str>>),
    Newline,
    Box(Size, bool),
}

pub(super) fn layout_inline_nodes<'doc>(
    engine: &mut LayoutEngine<'_>,
    nodes: &[&'doc Node],
    parent_style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    content_box: Rect,
    start_y: i32,
    paint: bool,
) -> Result<i32, String> {
    layout_inline_nodes_with_link(
        engine,
        nodes,
        parent_style,
        ancestors,
        content_box,
        start_y,
        paint,
        None,
    )
}

pub(super) fn layout_inline_nodes_with_link<'doc>(
    engine: &mut LayoutEngine<'_>,
    nodes: &[&'doc Node],
    parent_style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    content_box: Rect,
    start_y: i32,
    paint: bool,
    link_href: Option<Rc<str>>,
) -> Result<i32, String> {
    let mut tokens = Vec::new();
    let mut cursor = InlineCursor::default();

    for &node in nodes {
        collect_tokens(
            engine,
            node,
            parent_style,
            ancestors,
            paint,
            link_href.clone(),
            &mut cursor,
            &mut tokens,
        );
    }

    layout_tokens(engine, &tokens, parent_style, content_box, start_y, paint)
}

pub(super) fn measure_inline_nodes<'doc>(
    engine: &LayoutEngine<'_>,
    nodes: &[&'doc Node],
    parent_style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    max_width: i32,
) -> Result<Size, String> {
    let mut tokens = Vec::new();
    let mut cursor = InlineCursor::default();

    for &node in nodes {
        collect_tokens(
            engine,
            node,
            parent_style,
            ancestors,
            false,
            None,
            &mut cursor,
            &mut tokens,
        );
    }

    measure_tokens(engine, &tokens, parent_style, max_width)
}

#[derive(Default)]
struct InlineCursor {
    pending_space: Option<PendingSpace>,
}

#[derive(Clone, Debug)]
struct PendingSpace {
    style: TextStyle,
    visible: bool,
    link_href: Option<Rc<str>>,
}

impl InlineCursor {
    fn mark_pending_space(&mut self, style: TextStyle, visible: bool, link_href: Option<Rc<str>>) {
        self.pending_space = Some(PendingSpace {
            style,
            visible,
            link_href,
        });
    }

    fn clear_pending_space(&mut self) {
        self.pending_space = None;
    }

    fn flush_pending_space(&mut self, out: &mut Vec<InlineToken>) {
        let Some(space) = self.pending_space.take() else {
            return;
        };
        if matches!(out.last(), Some(InlineToken::Newline) | None) {
            return;
        }
        out.push(InlineToken::Space(space.style, space.visible, space.link_href));
    }
}

fn collect_tokens<'doc>(
    engine: &LayoutEngine<'_>,
    node: &'doc Node,
    parent_style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    paint: bool,
    link_href: Option<Rc<str>>,
    cursor: &mut InlineCursor,
    out: &mut Vec<InlineToken>,
) {
    match node {
        Node::Text(text) => {
            let visible = paint && parent_style.visibility == Visibility::Visible;
            push_text(
                text,
                engine.text_style_for(parent_style),
                visible,
                link_href,
                cursor,
                out,
            )
        }
        Node::Element(el) => {
            let style = engine.styles.compute_style(el, parent_style, ancestors);
            if style.display == Display::None {
                return;
            }

            if el.name == "br" {
                out.push(InlineToken::Newline);
                cursor.clear_pending_space();
                return;
            }

            let link_href = anchor_href(el).or(link_href);
            let paint = paint && style.visibility == Visibility::Visible;
            if is_replaced_element(el) {
                cursor.flush_pending_space(out);
                let size = inline_box_size(&style);
                out.push(InlineToken::Box(size, paint));
                return;
            }
            let display = style.display;
            ancestors.push(el);
            match display {
                Display::Inline => {
                    push_inline_spacing(out, style.margin.left.saturating_add(style.padding.left));
                    for child in &el.children {
                        collect_tokens(
                            engine,
                            child,
                            &style,
                            ancestors,
                            paint,
                            link_href.clone(),
                            cursor,
                            out,
                        );
                    }
                    push_inline_spacing(out, style.margin.right.saturating_add(style.padding.right));
                }
                _ => {
                    cursor.flush_pending_space(out);
                    let size = inline_box_size(&style);
                    out.push(InlineToken::Box(size, paint));
                }
            }
            ancestors.pop();
        }
    }
}

fn anchor_href(element: &Element) -> Option<Rc<str>> {
    if element.name != "a" {
        return None;
    }
    let href = element.attributes.get("href")?.trim();
    if href.is_empty() {
        return None;
    }
    Some(Rc::from(href))
}

fn is_replaced_element(element: &Element) -> bool {
    matches!(element.name.as_str(), "img" | "input")
}

fn push_inline_spacing(out: &mut Vec<InlineToken>, width: i32) {
    let width = width.max(0);
    if width == 0 {
        return;
    }
    out.push(InlineToken::Box(
        Size { width, height: 0 },
        false,
    ));
}

fn inline_box_size(style: &ComputedStyle) -> Size {
    let width = style
        .width_px
        .unwrap_or(0)
        .saturating_add(style.margin.left)
        .saturating_add(style.margin.right)
        .saturating_add(style.padding.left)
        .saturating_add(style.padding.right);
    let height = style
        .height_px
        .unwrap_or(0)
        .saturating_add(style.margin.top)
        .saturating_add(style.margin.bottom)
        .saturating_add(style.padding.top)
        .saturating_add(style.padding.bottom);
    Size { width, height }
}

fn push_text(
    text: &str,
    style: TextStyle,
    visible: bool,
    link_href: Option<Rc<str>>,
    cursor: &mut InlineCursor,
    out: &mut Vec<InlineToken>,
) {
    let mut iter = text.chars().peekable();
    while let Some(ch) = iter.next() {
        if ch.is_whitespace() {
            cursor.mark_pending_space(style, visible, link_href.clone());
            continue;
        }

        cursor.flush_pending_space(out);

        let mut word = String::new();
        word.push(ch);
        while let Some(&next) = iter.peek() {
            if next.is_whitespace() {
                break;
            }
            word.push(next);
            iter.next();
        }
        out.push(InlineToken::Word(word, style, visible, link_href.clone()));
    }
}

fn layout_tokens(
    engine: &mut LayoutEngine<'_>,
    tokens: &[InlineToken],
    parent_style: &ComputedStyle,
    content_box: Rect,
    start_y: i32,
    paint: bool,
) -> Result<i32, String> {
    let mut lines: Vec<Line> = Vec::new();
    let base_style = engine.text_style_for(parent_style);
    let base_metrics = engine.measurer.font_metrics_px(base_style);
    let mut line = Line::new(parent_style.line_height_px, base_metrics);
    let mut x_px = 0i32;

    for token in tokens {
        match token {
            InlineToken::Newline => {
                lines.push(std::mem::replace(
                    &mut line,
                    Line::new(parent_style.line_height_px, base_metrics),
                ));
                x_px = 0;
            }
            InlineToken::Space(style, visible, link_href) => {
                if x_px == 0 {
                    continue;
                }
                let space_width_px = engine.measurer.text_width_px(" ", *style)?;
                if x_px.saturating_add(space_width_px) > content_box.width {
                    continue;
                }
                let metrics = engine.measurer.font_metrics_px(*style);
                line.push(Fragment::Text(
                    " ".to_owned(),
                    *style,
                    space_width_px,
                    metrics,
                    *visible,
                    link_href.clone(),
                ));
                x_px = x_px.saturating_add(space_width_px);
            }
            InlineToken::Word(text, style, visible, link_href) => {
                if text.is_empty() {
                    continue;
                }
                let word_width_px = engine.measurer.text_width_px(text, *style)?;
                if x_px != 0 && x_px.saturating_add(word_width_px) > content_box.width {
                    lines.push(std::mem::replace(
                        &mut line,
                        Line::new(parent_style.line_height_px, base_metrics),
                    ));
                    x_px = 0;
                }

                let metrics = engine.measurer.font_metrics_px(*style);
                line.push(Fragment::Text(
                    text.clone(),
                    *style,
                    word_width_px,
                    metrics,
                    *visible,
                    link_href.clone(),
                ));
                x_px = x_px.saturating_add(word_width_px);
            }
            InlineToken::Box(size, visible) => {
                if x_px != 0 && x_px.saturating_add(size.width) > content_box.width {
                    lines.push(std::mem::replace(
                        &mut line,
                        Line::new(parent_style.line_height_px, base_metrics),
                    ));
                    x_px = 0;
                }
                line.push(Fragment::Box(*size, *visible));
                x_px = x_px.saturating_add(size.width);
            }
        }
    }

    if !line.fragments.is_empty() {
        lines.push(line);
    }

    let mut y_px = start_y;
    for line in lines {
        if y_px >= engine.viewport.height_px {
            break;
        }
        let line_width = line.width_px;
        let align = parent_style.text_align;
        let x_offset = match align {
            TextAlign::Left => 0,
            TextAlign::Center => ((content_box.width - line_width) / 2).max(0),
            TextAlign::Right => (content_box.width - line_width).max(0),
        };

        let baseline_y = y_px.saturating_add(line.baseline_offset_px());
        let mut x_px = content_box.x.saturating_add(x_offset);
        for frag in line.fragments {
            match frag {
                Fragment::Text(text, style, width, _metrics, visible, link_href) => {
                    if paint && visible {
                        engine.list.commands.push(DisplayCommand::Text(DrawText {
                            x_px,
                            y_px: baseline_y,
                            text,
                            style,
                        }));
                        if let Some(href) = link_href {
                            engine.link_regions.push(LinkHitRegion {
                                href,
                                x_px,
                                y_px,
                                width_px: width,
                                height_px: line.height_px,
                            });
                        }
                    }
                    x_px = x_px.saturating_add(width);
                }
                Fragment::Box(size, visible) => {
                    if paint && visible {
                        // draw backgrounds for inline boxes if they have one.
                        if let Some(color) = parent_style.background_color {
                            engine.list.commands.push(DisplayCommand::Rect(DrawRect {
                                x_px,
                                y_px,
                                width_px: size.width,
                                height_px: size.height,
                                color,
                            }));
                        }
                    }
                    x_px = x_px.saturating_add(size.width);
                }
            }
        }

        y_px = y_px.saturating_add(line.height_px);
    }

    Ok(y_px.saturating_sub(start_y).max(0))
}

fn measure_tokens(
    engine: &LayoutEngine<'_>,
    tokens: &[InlineToken],
    parent_style: &ComputedStyle,
    max_width: i32,
) -> Result<Size, String> {
    let max_width = max_width.max(0);
    let mut lines: Vec<Line> = Vec::new();
    let base_style = engine.text_style_for(parent_style);
    let base_metrics = engine.measurer.font_metrics_px(base_style);
    let mut line = Line::new(parent_style.line_height_px, base_metrics);
    let mut x_px = 0i32;

    for token in tokens {
        match token {
            InlineToken::Newline => {
                lines.push(std::mem::replace(
                    &mut line,
                    Line::new(parent_style.line_height_px, base_metrics),
                ));
                x_px = 0;
            }
            InlineToken::Space(style, _visible, _link_href) => {
                if x_px == 0 {
                    continue;
                }
                let space_width_px = engine.measurer.text_width_px(" ", *style)?;
                if x_px.saturating_add(space_width_px) > max_width {
                    continue;
                }
                let metrics = engine.measurer.font_metrics_px(*style);
                line.push(Fragment::Text(
                    " ".to_owned(),
                    *style,
                    space_width_px,
                    metrics,
                    false,
                    None,
                ));
                x_px = x_px.saturating_add(space_width_px);
            }
            InlineToken::Word(text, style, _visible, _link_href) => {
                if text.is_empty() {
                    continue;
                }
                let word_width_px = engine.measurer.text_width_px(text, *style)?;
                if x_px != 0 && x_px.saturating_add(word_width_px) > max_width {
                    lines.push(std::mem::replace(
                        &mut line,
                        Line::new(parent_style.line_height_px, base_metrics),
                    ));
                    x_px = 0;
                }

                let metrics = engine.measurer.font_metrics_px(*style);
                line.push(Fragment::Text(
                    text.clone(),
                    *style,
                    word_width_px,
                    metrics,
                    false,
                    None,
                ));
                x_px = x_px.saturating_add(word_width_px);
            }
            InlineToken::Box(size, visible) => {
                if x_px != 0 && x_px.saturating_add(size.width) > max_width {
                    lines.push(std::mem::replace(
                        &mut line,
                        Line::new(parent_style.line_height_px, base_metrics),
                    ));
                    x_px = 0;
                }
                line.push(Fragment::Box(*size, *visible));
                x_px = x_px.saturating_add(size.width);
            }
        }
    }

    if !line.fragments.is_empty() {
        lines.push(line);
    }

    let mut width_px = 0i32;
    let mut height_px = 0i32;
    for line in lines {
        width_px = width_px.max(line.width_px);
        height_px = height_px.saturating_add(line.height_px);
    }

    Ok(Size {
        width: width_px.max(0),
        height: height_px.max(0),
    })
}

#[derive(Clone, Debug)]
enum Fragment {
    Text(String, TextStyle, i32, FontMetricsPx, bool, Option<Rc<str>>),
    Box(Size, bool),
}

struct Line {
    fragments: Vec<Fragment>,
    width_px: i32,
    ascent_px: i32,
    descent_px: i32,
    height_px: i32,
    explicit_line_height_px: Option<i32>,
}

impl Line {
    fn new(explicit_line_height_px: Option<i32>, base_metrics: FontMetricsPx) -> Line {
        let ascent_px = base_metrics.ascent_px.max(1);
        let descent_px = base_metrics.descent_px.max(0);
        let text_height_px = ascent_px.saturating_add(descent_px).max(1);
        let height_px = explicit_line_height_px
            .unwrap_or(text_height_px)
            .max(text_height_px)
            .max(1);
        Line {
            fragments: Vec::new(),
            width_px: 0,
            ascent_px,
            descent_px,
            height_px,
            explicit_line_height_px,
        }
    }

    fn push(&mut self, fragment: Fragment) {
        match &fragment {
            Fragment::Text(_, _, width, metrics, _, _) => {
                self.width_px = self.width_px.saturating_add(*width);
                self.ascent_px = self.ascent_px.max(metrics.ascent_px.max(1));
                self.descent_px = self.descent_px.max(metrics.descent_px.max(0));
            }
            Fragment::Box(size, _) => {
                self.width_px = self.width_px.saturating_add(size.width);
                self.height_px = self.height_px.max(size.height.max(1));
            }
        }
        self.recompute_height();
        self.fragments.push(fragment);
    }

    fn recompute_height(&mut self) {
        let text_height_px = self.ascent_px.saturating_add(self.descent_px).max(1);
        self.height_px = self.height_px.max(text_height_px);
        if let Some(explicit) = self.explicit_line_height_px {
            self.height_px = self.height_px.max(explicit.max(1));
        }
    }

    fn baseline_offset_px(&self) -> i32 {
        let text_height_px = self.ascent_px.saturating_add(self.descent_px).max(1);
        let extra = self.height_px.saturating_sub(text_height_px).max(0);
        self.ascent_px.saturating_add(extra / 2)
    }
}
