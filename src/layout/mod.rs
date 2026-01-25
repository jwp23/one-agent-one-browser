mod inline;
mod flex;
mod table;

use crate::image::Argb32Image;
use crate::resources::ResourceLoader;
use crate::dom::{Document, Element, Node};
use crate::geom::{Edges, Rect};
use crate::render::{
    DisplayCommand,
    DisplayList,
    DrawImage,
    DrawRect,
    DrawRoundedRect,
    DrawRoundedRectBorder,
    DrawSvg,
    LinkHitRegion,
    TextMeasurer,
    TextStyle,
    Viewport,
};
use crate::style::{AutoEdges, ComputedStyle, Display, Position, StyleComputer, TextAlign, Visibility};
use std::collections::HashMap;
use std::rc::Rc;

pub struct LayoutOutput {
    pub display_list: DisplayList,
    pub link_regions: Vec<LinkHitRegion>,
}

pub fn layout_document(
    document: &Document,
    styles: &StyleComputer,
    measurer: &dyn TextMeasurer,
    viewport: Viewport,
    resources: &dyn ResourceLoader,
) -> Result<LayoutOutput, String> {
    let mut engine = LayoutEngine {
        styles,
        measurer,
        viewport,
        resources,
        image_cache: HashMap::new(),
        list: DisplayList::default(),
        link_regions: Vec::new(),
        positioned_containing_blocks: Vec::new(),
    };
    engine.layout_document(document)?;
    Ok(LayoutOutput {
        display_list: engine.list,
        link_regions: engine.link_regions,
    })
}

struct LayoutEngine<'a> {
    styles: &'a StyleComputer,
    measurer: &'a dyn TextMeasurer,
    viewport: Viewport,
    resources: &'a dyn ResourceLoader,
    image_cache: HashMap<String, Rc<Argb32Image>>,
    list: DisplayList,
    link_regions: Vec<LinkHitRegion>,
    positioned_containing_blocks: Vec<Rect>,
}

impl LayoutEngine<'_> {
    fn current_positioned_containing_block(&self) -> Rect {
        self.positioned_containing_blocks
            .last()
            .copied()
            .unwrap_or(Rect {
                x: 0,
                y: 0,
                width: self.viewport.width_px.max(0),
                height: self.viewport.height_px.max(0),
            })
    }

    fn push_positioned_containing_block(&mut self, border_box: Rect, border: Edges) {
        let height = if border_box.height > 0 {
            border_box.height
        } else {
            self.viewport.height_px.max(0)
        };
        let padding_box = Rect { height, ..border_box }.inset(border);
        self.positioned_containing_blocks.push(padding_box);
    }

    fn load_image(&mut self, src: &str) -> Result<Option<Rc<Argb32Image>>, String> {
        let src = src.trim();
        if src.is_empty() {
            return Ok(None);
        }
        if let Some(existing) = self.image_cache.get(src) {
            return Ok(Some(existing.clone()));
        }

        let Some(bytes) = self.resources.load_bytes(src)? else {
            return Ok(None);
        };
        let decoded = match crate::image::decode_image(&bytes) {
            Ok(image) => image,
            Err(_) => return Ok(None),
        };

        let image = Rc::new(decoded);
        self.image_cache.insert(src.to_owned(), image.clone());
        Ok(Some(image))
    }

    fn paint_replaced_content(&mut self, element: &Element, content_box: Rect) -> Result<(), String> {
        if content_box.width <= 0 || content_box.height <= 0 {
            return Ok(());
        }

        match element.name.as_str() {
            "img" => {
                if let Some(src) = element.attributes.get("src") {
                    if let Some(image) = self.load_image(src)? {
                        self.list.commands.push(DisplayCommand::Image(DrawImage {
                            x_px: content_box.x,
                            y_px: content_box.y,
                            width_px: content_box.width,
                            height_px: content_box.height,
                            opacity: 255,
                            image,
                        }));
                    }
                }
            }
            "svg" => {
                let xml = inline::serialize_element_xml(element);
                self.list.commands.push(DisplayCommand::Svg(DrawSvg {
                    x_px: content_box.x,
                    y_px: content_box.y,
                    width_px: content_box.width,
                    height_px: content_box.height,
                    opacity: 255,
                    svg_xml: Rc::from(xml),
                }));
            }
            _ => {}
        }

        Ok(())
    }

    fn layout_document(&mut self, document: &Document) -> Result<(), String> {
        let root = document.render_root();
        let root_style = ComputedStyle::root_defaults();
        let mut ancestors = Vec::new();

        let style = self.styles.compute_style_in_viewport(
            root,
            &root_style,
            &ancestors,
            self.viewport.width_px,
            self.viewport.height_px,
        );

        let rect = Rect {
            x: 0,
            y: 0,
            width: self.viewport.width_px.max(0),
            height: self.viewport.height_px.max(0),
        };
        self.positioned_containing_blocks.clear();
        self.positioned_containing_blocks.push(rect);
        if let Some(color) = resolve_canvas_background(
            document,
            self.styles,
            &root_style,
            &style,
            self.viewport.width_px,
            self.viewport.height_px,
        ) {
            self.list.commands.push(DisplayCommand::Rect(DrawRect {
                x_px: rect.x,
                y_px: rect.y,
                width_px: rect.width,
                height_px: rect.height,
                color,
            }));
        }
        let mut cursor_y = rect.y;
        self.layout_block_box(root, &style, &root_style, &mut ancestors, rect, &mut cursor_y, true)
    }

    fn layout_block_box<'doc>(
        &mut self,
        element: &'doc Element,
        style: &ComputedStyle,
        parent_style: &ComputedStyle,
        ancestors: &mut Vec<&'doc Element>,
        containing: Rect,
        cursor_y: &mut i32,
        paint: bool,
    ) -> Result<(), String> {
        if style.display == Display::None {
            return Ok(());
        }

        let mut paint = paint && style.visibility == Visibility::Visible;
        if paint && style.opacity == 0 {
            paint = false;
        }
        let opacity = style.opacity;
        let needs_opacity_group = paint && opacity < 255;
        if needs_opacity_group {
            self.list.commands.push(DisplayCommand::PushOpacity(opacity));
        }
        let margin = style.margin;
        let margin_auto = style.margin_auto;
        let border = style.border_width;
        let padding = style.padding;

        let replaced_size = if inline::is_replaced_element(element) {
            Some(inline::measure_replaced_element_outer_size(
                element,
                style,
                containing.width,
            )?)
        } else {
            None
        };

        let margin_left_px = if margin_auto.left { 0 } else { margin.left };
        let margin_right_px = if margin_auto.right { 0 } else { margin.right };

        let available_width = containing
            .width
            .saturating_sub(margin_left_px.saturating_add(margin_right_px))
            .max(0);
        let used_width = if let Some(size) = replaced_size {
            size.width
                .saturating_sub(margin.left.saturating_add(margin.right))
                .max(0)
        } else {
            let mut width = self.resolve_used_width(element, style, available_width);
            if let Some(min_width) = style.min_width_px {
                width = width.max(min_width);
            }
            if let Some(max_width) = style.max_width_px {
                width = width.min(max_width);
            }
            width.max(0)
        };

        let mut x = containing.x.saturating_add(margin_left_px);
        let y = cursor_y.saturating_add(margin.top);

        if margin_auto.left || margin_auto.right {
            x = apply_auto_margin_alignment(margin_auto, containing, x, used_width, margin);
        } else {
            x = apply_block_alignment(parent_style.text_align, containing, x, used_width, margin);
        }

        let border_box = Rect {
            x,
            y,
            width: used_width,
            height: 0,
        };
        let content_box = border_box.inset(add_edges(border, padding));

        let mut background_index = None;
        if paint {
            if let Some(color) = style.background_color {
                background_index = Some(self.list.commands.len());
                if style.border_radius_px > 0 {
                    self.list
                        .commands
                        .push(DisplayCommand::RoundedRect(DrawRoundedRect {
                            x_px: border_box.x,
                            y_px: border_box.y,
                            width_px: border_box.width,
                            height_px: 0,
                            radius_px: style.border_radius_px,
                            color,
                        }));
                } else {
                    self.list.commands.push(DisplayCommand::Rect(DrawRect {
                        x_px: border_box.x,
                        y_px: border_box.y,
                        width_px: border_box.width,
                        height_px: 0,
                        color,
                    }));
                }
            }
        }

        let content_height = if let Some(size) = replaced_size {
            let border_height = size
                .height
                .saturating_sub(margin.top.saturating_add(margin.bottom))
                .max(0);
            border_height
                .saturating_sub(
                    border
                        .top
                        .saturating_add(padding.top)
                        .saturating_add(padding.bottom)
                        .saturating_add(border.bottom),
                )
                .max(0)
        } else {
            let mut pushed_positioning = false;
            if style.position != Position::Static {
                self.push_positioned_containing_block(border_box, border);
                pushed_positioning = true;
            }
            ancestors.push(element);
            let content_height = match style.display {
                Display::Table => table::layout_table(self, element, style, ancestors, content_box, paint)?
                    .height,
                Display::Flex => flex::layout_flex_row(self, element, style, ancestors, content_box, paint)?,
                _ => self.layout_flow_children(&element.children, style, ancestors, content_box, paint)?,
            };
            ancestors.pop();
            if pushed_positioning {
                let _ = self.positioned_containing_blocks.pop();
            }
            content_height
        };

        let mut border_height = border
            .top
            .saturating_add(padding.top)
            .saturating_add(content_height)
            .saturating_add(padding.bottom)
            .saturating_add(border.bottom);
        if let Some(height) = style.height_px {
            border_height = border_height.max(height);
        }
        if let Some(min_height) = style.min_height_px {
            border_height = border_height.max(min_height);
        }

        if let Some(index) = background_index {
            if let Some(cmd) = self.list.commands.get_mut(index) {
                match cmd {
                    DisplayCommand::Rect(rect) => rect.height_px = border_height,
                    DisplayCommand::RoundedRect(rect) => rect.height_px = border_height,
                    _ => {}
                }
            }
        }

        if paint {
            self.paint_border(
                Rect {
                    x: border_box.x,
                    y: border_box.y,
                    width: border_box.width,
                    height: border_height,
                },
                style,
            );

            if replaced_size.is_some() {
                let content_box = Rect {
                    x: border_box.x,
                    y: border_box.y,
                    width: border_box.width,
                    height: border_height,
                }
                .inset(add_edges(border, padding));
                self.paint_replaced_content(element, content_box)?;
            }
        }

        if needs_opacity_group {
            self.list.commands.push(DisplayCommand::PopOpacity(opacity));
        }

        *cursor_y = y
            .saturating_add(border_height)
            .saturating_add(margin.bottom);

        Ok(())
    }

    fn layout_positioned_box<'doc>(
        &mut self,
        element: &'doc Element,
        style: &ComputedStyle,
        ancestors: &mut Vec<&'doc Element>,
        containing: Rect,
        paint: bool,
    ) -> Result<(), String> {
        if style.display == Display::None {
            return Ok(());
        }

        let mut paint = paint && style.visibility == Visibility::Visible;
        if paint && style.opacity == 0 {
            paint = false;
        }
        let opacity = style.opacity;
        let needs_opacity_group = paint && opacity < 255;
        if needs_opacity_group {
            self.list.commands.push(DisplayCommand::PushOpacity(opacity));
        }

        let containing = match style.position {
            Position::Fixed => Rect {
                x: 0,
                y: 0,
                width: self.viewport.width_px.max(0),
                height: self.viewport.height_px.max(0),
            },
            _ => containing,
        };

        let margin = style.margin;
        let margin_auto = style.margin_auto;
        let border = style.border_width;
        let padding = style.padding;

        let replaced_size = if inline::is_replaced_element(element) {
            Some(inline::measure_replaced_element_outer_size(
                element,
                style,
                containing.width,
            )?)
        } else {
            None
        };

        let mut used_width = if let Some(width) = style.width_px {
            width
        } else if let (Some(left), Some(right)) = (style.left_px, style.right_px) {
            containing.width.saturating_sub(left.saturating_add(right))
        } else if let Some(size) = replaced_size {
            size.width
                .saturating_sub(margin.left.saturating_add(margin.right))
                .max(0)
        } else {
            flex::measure_element_max_content_width(self, element, style, ancestors, containing.width)?
        };
        if let Some(min_width) = style.min_width_px {
            used_width = used_width.max(min_width);
        }
        if let Some(max_width) = style.max_width_px {
            used_width = used_width.min(max_width);
        }
        used_width = used_width.max(0);

        let mut x = if let Some(left) = style.left_px {
            containing.x.saturating_add(left)
        } else if let Some(right) = style.right_px {
            containing
                .right()
                .saturating_sub(used_width)
                .saturating_sub(right)
        } else {
            containing.x
        };
        let y = if let Some(top) = style.top_px {
            containing.y.saturating_add(top)
        } else {
            containing.y
        };

        if !margin_auto.left {
            x = x.saturating_add(margin.left);
        }
        let y = y.saturating_add(margin.top);

        let border_box = Rect {
            x,
            y,
            width: used_width,
            height: 0,
        };
        let content_box = border_box.inset(add_edges(border, padding));

        let mut background_index = None;
        if paint {
            if let Some(color) = style.background_color {
                background_index = Some(self.list.commands.len());
                if style.border_radius_px > 0 {
                    self.list
                        .commands
                        .push(DisplayCommand::RoundedRect(DrawRoundedRect {
                            x_px: border_box.x,
                            y_px: border_box.y,
                            width_px: border_box.width,
                            height_px: 0,
                            radius_px: style.border_radius_px,
                            color,
                        }));
                } else {
                    self.list.commands.push(DisplayCommand::Rect(DrawRect {
                        x_px: border_box.x,
                        y_px: border_box.y,
                        width_px: border_box.width,
                        height_px: 0,
                        color,
                    }));
                }
            }
        }

        let content_height = if let Some(size) = replaced_size {
            let border_height = size
                .height
                .saturating_sub(margin.top.saturating_add(margin.bottom))
                .max(0);
            border_height
                .saturating_sub(
                    border
                        .top
                        .saturating_add(padding.top)
                        .saturating_add(padding.bottom)
                        .saturating_add(border.bottom),
                )
                .max(0)
        } else {
            let mut pushed_positioning = false;
            if style.position != Position::Static {
                self.push_positioned_containing_block(border_box, border);
                pushed_positioning = true;
            }
            ancestors.push(element);
            let content_height = match style.display {
                Display::Table => table::layout_table(self, element, style, ancestors, content_box, paint)?
                    .height,
                Display::Flex => flex::layout_flex_row(self, element, style, ancestors, content_box, paint)?,
                _ => self.layout_flow_children(&element.children, style, ancestors, content_box, paint)?,
            };
            ancestors.pop();
            if pushed_positioning {
                let _ = self.positioned_containing_blocks.pop();
            }
            content_height
        };

        let mut border_height = border
            .top
            .saturating_add(padding.top)
            .saturating_add(content_height)
            .saturating_add(padding.bottom)
            .saturating_add(border.bottom);
        if let Some(height) = style.height_px {
            border_height = border_height.max(height);
        }
        if let Some(min_height) = style.min_height_px {
            border_height = border_height.max(min_height);
        }

        if let Some(index) = background_index {
            if let Some(cmd) = self.list.commands.get_mut(index) {
                match cmd {
                    DisplayCommand::Rect(rect) => rect.height_px = border_height,
                    DisplayCommand::RoundedRect(rect) => rect.height_px = border_height,
                    _ => {}
                }
            }
        }

        if paint {
            self.paint_border(
                Rect {
                    x: border_box.x,
                    y: border_box.y,
                    width: border_box.width,
                    height: border_height,
                },
                style,
            );

            if replaced_size.is_some() {
                let content_box = Rect {
                    x: border_box.x,
                    y: border_box.y,
                    width: border_box.width,
                    height: border_height,
                }
                .inset(add_edges(border, padding));
                self.paint_replaced_content(element, content_box)?;
            }
        }

        if needs_opacity_group {
            self.list.commands.push(DisplayCommand::PopOpacity(opacity));
        }

        Ok(())
    }

    fn layout_flow_children<'doc>(
        &mut self,
        children: &'doc [Node],
        parent_style: &ComputedStyle,
        ancestors: &mut Vec<&'doc Element>,
        content_box: Rect,
        paint: bool,
    ) -> Result<i32, String> {
        let mut cursor_y = content_box.y;
        let mut inline_nodes: Vec<&'doc Node> = Vec::new();

        for child in children {
            match child {
                Node::Text(_) => inline_nodes.push(child),
                Node::Element(el) => {
                    let style = self.styles.compute_style_in_viewport(
                        el,
                        parent_style,
                        ancestors,
                        self.viewport.width_px,
                        self.viewport.height_px,
                    );
                    if style.display == Display::None {
                        continue;
                    }

                    if matches!(style.position, Position::Absolute | Position::Fixed) {
                        if !inline_nodes.is_empty() {
                            let height = inline::layout_inline_nodes(
                                self,
                                &inline_nodes,
                                parent_style,
                                ancestors,
                                content_box,
                                cursor_y,
                                paint,
                            )?;
                            cursor_y = cursor_y.saturating_add(height);
                            inline_nodes.clear();
                        }

                        let containing = self.current_positioned_containing_block();
                        self.layout_positioned_box(
                            el,
                            &style,
                            ancestors,
                            containing,
                            paint,
                        )?;
                        continue;
                    }

                    if is_flow_block(&style, el) {
                        if !inline_nodes.is_empty() {
                            let height = inline::layout_inline_nodes(
                                self,
                                &inline_nodes,
                                parent_style,
                                ancestors,
                                content_box,
                                cursor_y,
                                paint,
                            )?;
                            cursor_y = cursor_y.saturating_add(height);
                            inline_nodes.clear();
                        }

                        let mut child_cursor_y = cursor_y;
                        self.layout_block_box(
                            el,
                            &style,
                            parent_style,
                            ancestors,
                            Rect {
                                x: content_box.x,
                                y: cursor_y,
                                width: content_box.width,
                                height: content_box.height,
                            },
                            &mut child_cursor_y,
                            paint,
                        )?;
                        cursor_y = child_cursor_y;
                    } else {
                        inline_nodes.push(child);
                    }
                }
            }

            if cursor_y >= self.viewport.height_px {
                break;
            }
        }

        if !inline_nodes.is_empty() && cursor_y < self.viewport.height_px {
                            let height = inline::layout_inline_nodes(
                                self,
                                &inline_nodes,
                                parent_style,
                                ancestors,
                                content_box,
                                cursor_y,
                                paint,
                            )?;
            cursor_y = cursor_y.saturating_add(height);
        }

        Ok(cursor_y.saturating_sub(content_box.y).max(0))
    }

    fn resolve_used_width(&self, element: &Element, style: &ComputedStyle, available_width: i32) -> i32 {
        if let Some(width) = style.width_px {
            return width;
        }

        if style.display == Display::Table {
            if let Some(percent) = element
                .attributes
                .get("width")
                .and_then(parse_percentage)
            {
                let pct_width = (available_width as f32 * (percent / 100.0)).round() as i32;
                return pct_width.max(0);
            }
        }

        available_width
    }

    fn text_style_for(&self, style: &ComputedStyle) -> TextStyle {
        TextStyle {
            color: style.color,
            bold: style.bold,
            underline: style.underline,
            font_family: style.font_family,
            font_size_px: style.font_size_px,
            letter_spacing_px: style.letter_spacing_px,
        }
    }

    fn paint_border(&mut self, border_box: Rect, style: &ComputedStyle) {
        if style.border_style != crate::style::BorderStyle::Solid {
            return;
        }

        let color = style.border_color;
        let border = style.border_width;
        if border.top <= 0 && border.right <= 0 && border.bottom <= 0 && border.left <= 0 {
            return;
        }

        if border.top == border.right
            && border.top == border.bottom
            && border.top == border.left
            && border.top > 0
        {
            self.list.commands.push(DisplayCommand::RoundedRectBorder(
                DrawRoundedRectBorder {
                    x_px: border_box.x,
                    y_px: border_box.y,
                    width_px: border_box.width,
                    height_px: border_box.height,
                    radius_px: style.border_radius_px,
                    border_width_px: border.top,
                    color,
                },
            ));
            return;
        }

        if border.top > 0 {
            self.list.commands.push(DisplayCommand::Rect(DrawRect {
                x_px: border_box.x,
                y_px: border_box.y,
                width_px: border_box.width,
                height_px: border.top,
                color,
            }));
        }
        if border.bottom > 0 {
            self.list.commands.push(DisplayCommand::Rect(DrawRect {
                x_px: border_box.x,
                y_px: border_box.bottom().saturating_sub(border.bottom),
                width_px: border_box.width,
                height_px: border.bottom,
                color,
            }));
        }

        let middle_height = border_box
            .height
            .saturating_sub(border.top.saturating_add(border.bottom))
            .max(0);
        if middle_height <= 0 {
            return;
        }

        if border.left > 0 {
            self.list.commands.push(DisplayCommand::Rect(DrawRect {
                x_px: border_box.x,
                y_px: border_box.y.saturating_add(border.top),
                width_px: border.left,
                height_px: middle_height,
                color,
            }));
        }
        if border.right > 0 {
            self.list.commands.push(DisplayCommand::Rect(DrawRect {
                x_px: border_box.right().saturating_sub(border.right),
                y_px: border_box.y.saturating_add(border.top),
                width_px: border.right,
                height_px: middle_height,
                color,
            }));
        }
    }
}

fn add_edges(a: Edges, b: Edges) -> Edges {
    Edges {
        top: a.top.saturating_add(b.top),
        right: a.right.saturating_add(b.right),
        bottom: a.bottom.saturating_add(b.bottom),
        left: a.left.saturating_add(b.left),
    }
}

fn resolve_canvas_background(
    document: &Document,
    styles: &StyleComputer,
    root_style: &ComputedStyle,
    body_style: &ComputedStyle,
    viewport_width_px: i32,
    viewport_height_px: i32,
) -> Option<crate::geom::Color> {
    if let Some(html) = document.find_first_element_by_name("html") {
        let html_style = styles.compute_style_in_viewport(
            html,
            root_style,
            &[],
            viewport_width_px,
            viewport_height_px,
        );
        if html_style.background_color.is_some() {
            return html_style.background_color;
        }
    }
    body_style.background_color
}

fn is_flow_block(style: &ComputedStyle, element: &Element) -> bool {
    match style.display {
        Display::Block | Display::Flex | Display::Table => true,
        Display::TableRow | Display::TableCell => true,
        Display::Inline | Display::InlineBlock => {
            if matches!(element.name.as_str(), "div" | "p" | "table") {
                return true;
            }

            if element.name != "span" {
                return false;
            }

            element.children.iter().any(|child| {
                let Node::Element(el) = child else {
                    return false;
                };
                matches!(
                    el.name.as_str(),
                    "html"
                        | "body"
                        | "div"
                        | "p"
                        | "center"
                        | "header"
                        | "main"
                        | "footer"
                        | "nav"
                        | "ul"
                        | "ol"
                        | "li"
                        | "h1"
                        | "h2"
                        | "h3"
                        | "blockquote"
                        | "pre"
                        | "table"
                        | "tr"
                        | "td"
                )
            })
        }
        Display::None => false,
    }
}

fn apply_block_alignment(align: TextAlign, containing: Rect, default_x: i32, width: i32, margin: Edges) -> i32 {
    if width <= 0 {
        return default_x;
    }
    let available = containing.width.saturating_sub(margin.left.saturating_add(margin.right));
    if available <= width {
        return default_x;
    }
    match align {
        TextAlign::Center => containing.x.saturating_add((available - width) / 2),
        TextAlign::Right => containing
            .x
            .saturating_add(available.saturating_sub(width))
            .saturating_add(margin.left),
        TextAlign::Left => default_x,
    }
}

fn apply_auto_margin_alignment(
    auto: AutoEdges,
    containing: Rect,
    default_x: i32,
    width: i32,
    margin: Edges,
) -> i32 {
    let left_px = if auto.left { 0 } else { margin.left };
    let right_px = if auto.right { 0 } else { margin.right };
    let available = containing
        .width
        .saturating_sub(left_px.saturating_add(right_px))
        .max(0);

    if available <= width {
        return default_x;
    }

    let remaining = available.saturating_sub(width).max(0);
    if auto.left && auto.right {
        containing.x.saturating_add(left_px).saturating_add(remaining / 2)
    } else if auto.left {
        containing.x.saturating_add(left_px).saturating_add(remaining)
    } else {
        default_x
    }
}

fn parse_percentage(value: &str) -> Option<f32> {
    let value = value.trim();
    let number = value.strip_suffix('%')?;
    let number: f32 = number.trim().parse().ok()?;
    Some(number)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedMeasurer;

    impl TextMeasurer for FixedMeasurer {
        fn font_metrics_px(&self, _style: TextStyle) -> crate::render::FontMetricsPx {
            crate::render::FontMetricsPx {
                ascent_px: 8,
                descent_px: 2,
            }
        }

        fn text_width_px(&self, text: &str, _style: TextStyle) -> Result<i32, String> {
            Ok(text.len() as i32)
        }
    }

    #[test]
    fn wraps_words_when_exceeding_width() {
        let doc = crate::html::parse_document("<p>Hello World</p>");
        let viewport = Viewport {
            width_px: 5,
            height_px: 200,
        };
        let styles = crate::style::StyleComputer::from_document(&doc);
        let output =
            layout_document(&doc, &styles, &FixedMeasurer, viewport, &crate::resources::NoResources)
                .unwrap();
        assert!(output
            .display_list
            .commands
            .iter()
            .any(|cmd| matches!(cmd, DisplayCommand::Text(_))));
    }

    #[test]
    fn records_link_hit_regions_for_anchor_text() {
        let doc = crate::html::parse_document(r#"<p><a href="https://example.com">Hello</a></p>"#);
        let viewport = Viewport {
            width_px: 200,
            height_px: 200,
        };
        let styles = crate::style::StyleComputer::from_document(&doc);
        let output =
            layout_document(&doc, &styles, &FixedMeasurer, viewport, &crate::resources::NoResources)
                .unwrap();
        assert!(output
            .link_regions
            .iter()
            .any(|region| region.href.as_ref() == "https://example.com"));
    }

    #[test]
    fn records_link_hit_regions_for_flex_item_anchor() {
        let doc = crate::html::parse_document(
            r#"<style>header { display: flex; }</style><header><a href="/posts/">Posts</a></header>"#,
        );
        let viewport = Viewport {
            width_px: 200,
            height_px: 200,
        };
        let styles = crate::style::StyleComputer::from_document(&doc);
        let output =
            layout_document(&doc, &styles, &FixedMeasurer, viewport, &crate::resources::NoResources)
                .unwrap();
        assert!(output
            .link_regions
            .iter()
            .any(|region| region.href.as_ref() == "/posts/"));
    }
}
