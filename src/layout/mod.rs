mod flex;
mod floats;
mod helpers;
mod inline;
mod replaced;
mod svg_xml;
mod table;

use crate::dom::{Document, Element, Node};
use crate::geom::{Edges, Rect};
use crate::image::Argb32Image;
use crate::render::{
    DisplayCommand, DisplayList, DrawLinearGradientRect, DrawRect, DrawRoundedRect,
    DrawRoundedRectBorder, LinkHitRegion, TextMeasurer, TextStyle, Viewport,
};
use crate::resources::ResourceLoader;
use crate::style::{
    ComputedStyle, Display, Float, Position, StyleComputer, Visibility,
};
use std::collections::HashMap;
use std::rc::Rc;

use helpers::*;

pub struct LayoutOutput {
    pub display_list: DisplayList,
    pub link_regions: Vec<LinkHitRegion>,
    pub document_height_px: i32,
    pub canvas_background_color: Option<crate::geom::Color>,
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
        fixed_depth: 0,
        canvas_background_color: None,
    };
    let document_height_px = engine.layout_document(document)?;
    Ok(LayoutOutput {
        display_list: engine.list,
        link_regions: engine.link_regions,
        document_height_px,
        canvas_background_color: engine.canvas_background_color,
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
    fixed_depth: usize,
    canvas_background_color: Option<crate::geom::Color>,
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
        let padding_box = Rect {
            height,
            ..border_box
        }
        .inset(border);
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
        let decoded = match crate::image::decode_image(bytes.as_ref()) {
            Ok(image) => image,
            Err(_) => return Ok(None),
        };

        let image = Rc::new(decoded);
        self.image_cache.insert(src.to_owned(), image.clone());
        Ok(Some(image))
    }

    fn layout_document(&mut self, document: &Document) -> Result<i32, String> {
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
        let body_style = if root.name == "html" {
            document
                .find_first_element_by_name("body")
                .map(|body| {
                    let body_ancestors = vec![root];
                    self.styles.compute_style_in_viewport(
                        body,
                        &style,
                        &body_ancestors,
                        self.viewport.width_px,
                        self.viewport.height_px,
                    )
                })
                .unwrap_or_else(|| style.clone())
        } else {
            style.clone()
        };

        let rect = Rect {
            x: 0,
            y: 0,
            width: self.viewport.width_px.max(0),
            height: self.viewport.height_px.max(0),
        };
        self.positioned_containing_blocks.clear();
        self.positioned_containing_blocks.push(rect);
        self.canvas_background_color = resolve_canvas_background(
            document,
            self.styles,
            &root_style,
            &body_style,
            self.viewport.width_px,
            self.viewport.height_px,
        );
        let mut cursor_y = rect.y;
        self.layout_block_box(
            root,
            &style,
            &root_style,
            &mut ancestors,
            rect,
            &mut cursor_y,
            true,
            None,
        )?;
        Ok(cursor_y.max(self.viewport.height_px).max(0))
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
        flow_override: Option<Rect>,
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
            self.list
                .commands
                .push(DisplayCommand::PushOpacity(opacity));
        }
        let margin = style.margin;
        let margin_auto = style.margin_auto;
        let border = style.border_width;
        let padding = style.padding.resolve_px(containing.width);

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
            if let Some(min_width) = style
                .min_width_px
                .map(|width| width.resolve_px(available_width))
            {
                width = width.max(min_width);
            }
            if let Some(max_width) = style
                .max_width_px
                .map(|width| width.resolve_px(available_width))
            {
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
        let child_content_box = flow_override
            .map(|flow| constrain_flow_content_box(content_box, flow))
            .unwrap_or(content_box);

        let background_index = if paint {
            self.push_background(border_box, style, 0)
        } else {
            None
        };

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
                Display::Table => {
                    table::layout_table(self, element, style, ancestors, content_box, paint)?.height
                }
                Display::Flex => {
                    flex::layout_flex_row(self, element, style, ancestors, content_box, paint)?
                }
                _ => self.layout_flow_children(
                    &element.children,
                    style,
                    ancestors,
                    child_content_box,
                    paint,
                )?,
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
            self.set_background_height(index, border_height);
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
                self.paint_replaced_content(element, style, content_box)?;
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
        let is_fixed = paint && style.position == Position::Fixed;
        if is_fixed {
            self.fixed_depth = self.fixed_depth.saturating_add(1);
            self.list.commands.push(DisplayCommand::PushFixed);
        }

        let opacity = style.opacity;
        let needs_opacity_group = paint && opacity < 255;
        if needs_opacity_group {
            self.list
                .commands
                .push(DisplayCommand::PushOpacity(opacity));
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
        let padding = style.padding.resolve_px(containing.width);

        let replaced_size = if inline::is_replaced_element(element) {
            Some(inline::measure_replaced_element_outer_size(
                element,
                style,
                containing.width,
            )?)
        } else {
            None
        };

        let mut used_width = if let Some(width) = style
            .width_px
            .map(|width| width.resolve_px(containing.width))
        {
            width
        } else if let (Some(left), Some(right)) = (style.left_px, style.right_px) {
            let left = left.resolve_px(containing.width);
            let right = right.resolve_px(containing.width);
            containing.width.saturating_sub(left.saturating_add(right))
        } else if let Some(size) = replaced_size {
            size.width
                .saturating_sub(margin.left.saturating_add(margin.right))
                .max(0)
        } else {
            flex::measure_element_max_content_width(
                self,
                element,
                style,
                ancestors,
                containing.width,
            )?
        };
        if let Some(min_width) = style
            .min_width_px
            .map(|width| width.resolve_px(containing.width))
        {
            used_width = used_width.max(min_width);
        }
        if let Some(max_width) = style
            .max_width_px
            .map(|width| width.resolve_px(containing.width))
        {
            used_width = used_width.min(max_width);
        }
        used_width = used_width.max(0);

        let mut x = if let Some(left) = style.left_px {
            containing
                .x
                .saturating_add(left.resolve_px(containing.width))
        } else if let Some(right) = style.right_px {
            containing
                .right()
                .saturating_sub(used_width)
                .saturating_sub(right.resolve_px(containing.width))
        } else {
            containing.x
        };
        let y = if let Some(top) = style.top_px {
            containing
                .y
                .saturating_add(top.resolve_px(containing.height))
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

        let background_index = if paint {
            self.push_background(border_box, style, 0)
        } else {
            None
        };

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
                Display::Table => {
                    table::layout_table(self, element, style, ancestors, content_box, paint)?.height
                }
                Display::Flex => {
                    flex::layout_flex_row(self, element, style, ancestors, content_box, paint)?
                }
                _ => self.layout_flow_children(
                    &element.children,
                    style,
                    ancestors,
                    content_box,
                    paint,
                )?,
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
            self.set_background_height(index, border_height);
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
                self.paint_replaced_content(element, style, content_box)?;
            }
        }

        if needs_opacity_group {
            self.list.commands.push(DisplayCommand::PopOpacity(opacity));
        }

        if is_fixed {
            self.list.commands.push(DisplayCommand::PopFixed);
            self.fixed_depth = self.fixed_depth.saturating_sub(1);
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
        struct DeferredFloatPaint {
            commands: Vec<DisplayCommand>,
            links: Vec<LinkHitRegion>,
        }

        let inherited_link_href = ancestors.iter().rev().find_map(|ancestor| {
            if ancestor.name != "a" {
                return None;
            }
            let href = ancestor.attributes.get("href")?.trim();
            if href.is_empty() {
                return None;
            }
            Some(Rc::<str>::from(href))
        });

        let mut cursor_y = content_box.y;
        let mut inline_nodes: Vec<&'doc Node> = Vec::new();
        let mut floats: Vec<floats::FloatPlacement> = Vec::new();
        let mut max_float_bottom = cursor_y;
        let mut deferred_floats: Vec<DeferredFloatPaint> = Vec::new();

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

                    if matches!(style.float, Float::Left | Float::Right)
                        && !matches!(style.position, Position::Absolute | Position::Fixed)
                    {
                        if !inline_nodes.is_empty() {
                            let (flow_box, new_y) =
                                floats::flow_area_at_y(&floats, content_box, cursor_y);
                            cursor_y = new_y;
                            let height = inline::layout_inline_nodes_with_link(
                                self,
                                &inline_nodes,
                                parent_style,
                                ancestors,
                                flow_box,
                                cursor_y,
                                paint,
                                inherited_link_href.clone(),
                            )?;
                            cursor_y = cursor_y.saturating_add(height);
                            inline_nodes.clear();
                        }

                        let mut saved_commands = Vec::new();
                        let mut saved_links = Vec::new();
                        std::mem::swap(&mut self.list.commands, &mut saved_commands);
                        std::mem::swap(&mut self.link_regions, &mut saved_links);

                        let placement = floats::layout_float(
                            self,
                            el,
                            &style,
                            parent_style,
                            ancestors,
                            content_box,
                            cursor_y,
                            &floats,
                            paint,
                        )?;
                        deferred_floats.push(DeferredFloatPaint {
                            commands: std::mem::take(&mut self.list.commands),
                            links: std::mem::take(&mut self.link_regions),
                        });

                        std::mem::swap(&mut self.list.commands, &mut saved_commands);
                        std::mem::swap(&mut self.link_regions, &mut saved_links);
                        max_float_bottom = max_float_bottom.max(placement.rect.bottom());
                        floats.push(placement);
                        continue;
                    }

                    if matches!(style.position, Position::Absolute | Position::Fixed) {
                        if !inline_nodes.is_empty() {
                            let (flow_box, new_y) =
                                floats::flow_area_at_y(&floats, content_box, cursor_y);
                            cursor_y = new_y;
                            let height = inline::layout_inline_nodes_with_link(
                                self,
                                &inline_nodes,
                                parent_style,
                                ancestors,
                                flow_box,
                                cursor_y,
                                paint,
                                inherited_link_href.clone(),
                            )?;
                            cursor_y = cursor_y.saturating_add(height);
                            inline_nodes.clear();
                        }

                        let containing = self.current_positioned_containing_block();
                        self.layout_positioned_box(el, &style, ancestors, containing, paint)?;
                        continue;
                    }

                    if is_flow_block(&style, el) {
                        if !inline_nodes.is_empty() {
                            let (flow_box, new_y) =
                                floats::flow_area_at_y(&floats, content_box, cursor_y);
                            cursor_y = new_y;
                            let height = inline::layout_inline_nodes_with_link(
                                self,
                                &inline_nodes,
                                parent_style,
                                ancestors,
                                flow_box,
                                cursor_y,
                                paint,
                                inherited_link_href.clone(),
                            )?;
                            cursor_y = cursor_y.saturating_add(height);
                            inline_nodes.clear();
                        }

                        let establishes_bfc = establishes_block_formatting_context(&style);
                        if establishes_bfc {
                            let required_outer_width = required_outer_width_for_float_clearance(
                                &style,
                                content_box.width,
                            );
                            let (flow_box, new_y) = floats::flow_area_for_width(
                                &floats,
                                content_box,
                                cursor_y,
                                required_outer_width,
                            );
                            cursor_y = new_y;
                            let mut child_cursor_y = cursor_y;
                            self.layout_block_box(
                                el,
                                &style,
                                parent_style,
                                ancestors,
                                Rect {
                                    x: flow_box.x,
                                    y: cursor_y,
                                    width: flow_box.width,
                                    height: content_box.height,
                                },
                                &mut child_cursor_y,
                                paint,
                                None,
                            )?;
                            cursor_y = child_cursor_y;
                        } else {
                            let flow_box = floats::flow_area_at_exact_y(&floats, content_box, cursor_y);
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
                                Some(flow_box),
                            )?;
                            cursor_y = child_cursor_y;
                        }
                    } else {
                        inline_nodes.push(child);
                    }
                }
            }

        }

        if !inline_nodes.is_empty() {
            let (flow_box, new_y) = floats::flow_area_at_y(&floats, content_box, cursor_y);
            cursor_y = new_y;
            let height = inline::layout_inline_nodes_with_link(
                self,
                &inline_nodes,
                parent_style,
                ancestors,
                flow_box,
                cursor_y,
                paint,
                inherited_link_href,
            )?;
            cursor_y = cursor_y.saturating_add(height);
        }

        for deferred in deferred_floats {
            self.list.commands.extend(deferred.commands);
            self.link_regions.extend(deferred.links);
        }

        Ok(cursor_y.max(max_float_bottom).saturating_sub(content_box.y).max(0))
    }

    fn resolve_used_width(
        &self,
        element: &Element,
        style: &ComputedStyle,
        available_width: i32,
    ) -> i32 {
        if let Some(width) = style.width_px {
            return width.resolve_px(available_width).max(0);
        }

        if style.display == Display::Table {
            if let Some(percent) = element.attributes.get("width").and_then(parse_percentage) {
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
            self.list
                .commands
                .push(DisplayCommand::RoundedRectBorder(DrawRoundedRectBorder {
                    x_px: border_box.x,
                    y_px: border_box.y,
                    width_px: border_box.width,
                    height_px: border_box.height,
                    radius_px: style.border_radius_px,
                    border_width_px: border.top,
                    color,
                }));
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

    fn push_background(
        &mut self,
        border_box: Rect,
        style: &ComputedStyle,
        height_px: i32,
    ) -> Option<usize> {
        if border_box.width <= 0 {
            return None;
        }

        if let Some(gradient) = style.background_gradient {
            let index = self.list.commands.len();
            self.list
                .commands
                .push(DisplayCommand::LinearGradientRect(DrawLinearGradientRect {
                    x_px: border_box.x,
                    y_px: border_box.y,
                    width_px: border_box.width,
                    height_px,
                    direction: gradient.direction,
                    start_color: gradient.start,
                    end_color: gradient.end,
                }));
            return Some(index);
        }

        let Some(color) = style.background_color else {
            return None;
        };

        let index = self.list.commands.len();
        if style.border_radius_px > 0 {
            self.list
                .commands
                .push(DisplayCommand::RoundedRect(DrawRoundedRect {
                    x_px: border_box.x,
                    y_px: border_box.y,
                    width_px: border_box.width,
                    height_px,
                    radius_px: style.border_radius_px,
                    color,
                }));
        } else {
            self.list.commands.push(DisplayCommand::Rect(DrawRect {
                x_px: border_box.x,
                y_px: border_box.y,
                width_px: border_box.width,
                height_px,
                color,
            }));
        }
        Some(index)
    }

    fn set_background_height(&mut self, index: usize, height_px: i32) {
        let Some(cmd) = self.list.commands.get_mut(index) else {
            return;
        };

        match cmd {
            DisplayCommand::Rect(rect) => rect.height_px = height_px,
            DisplayCommand::RoundedRect(rect) => rect.height_px = height_px,
            DisplayCommand::LinearGradientRect(rect) => rect.height_px = height_px,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests;
