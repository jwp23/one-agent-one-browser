use super::CustomProperties;
use super::parse::{parse_css_color, parse_css_length_px_with_viewport, parse_html_length_px};
use super::{
    AutoEdges, BorderStyle, ComputedStyle, CssEdges, CssLength, Display, FlexAlignItems,
    FlexDirection, FlexJustifyContent, FlexWrap, Float, FontFamily, LineHeight, LinearGradient,
    Position, TextAlign, TextTransform, Visibility, custom_properties, declarations, length,
};
use crate::css::{Rule, Specificity};
use crate::dom::Element;
use crate::geom::{Color, Edges};
use std::collections::HashMap;

pub(super) struct MatchedRule<'a> {
    pub(super) rule: &'a Rule,
    pub(super) specificity: Specificity,
    pub(super) order: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct CascadePriority {
    pub(super) specificity: CascadeSpecificity,
    pub(super) order: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct CascadeSpecificity {
    pub(super) inline: u16,
    pub(super) ids: u16,
    pub(super) classes: u16,
    pub(super) tags: u16,
}

impl CascadeSpecificity {
    pub(super) fn from_selector(specificity: Specificity) -> CascadeSpecificity {
        CascadeSpecificity {
            inline: 0,
            ids: specificity.ids,
            classes: specificity.classes,
            tags: specificity.tags,
        }
    }
}

pub(super) struct Cascaded<T> {
    pub(super) value: T,
    pub(super) priority: CascadePriority,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum LetterSpacing {
    Normal,
    Px(i32),
    Em(f32),
}

impl LetterSpacing {
    fn resolve_px(self, font_size_px: i32) -> i32 {
        let font_size_px = font_size_px.max(0);
        match self {
            LetterSpacing::Normal => 0,
            LetterSpacing::Px(px) => px,
            LetterSpacing::Em(factor) => (factor * (font_size_px as f32)).round() as i32,
        }
    }
}

pub(super) struct StyleBuilder {
    base: ComputedStyle,
    viewport: Option<(i32, i32)>,
    custom_properties_declared: HashMap<String, Cascaded<String>>,
    custom_properties: CustomProperties,
    display: Option<Cascaded<Display>>,
    visibility: Option<Cascaded<Visibility>>,
    position: Option<Cascaded<Position>>,
    float: Option<Cascaded<Float>>,
    top_px: Option<Cascaded<Option<CssLength>>>,
    right_px: Option<Cascaded<Option<CssLength>>>,
    bottom_px: Option<Cascaded<Option<CssLength>>>,
    left_px: Option<Cascaded<Option<CssLength>>>,
    opacity: Option<Cascaded<u8>>,
    color: Option<Cascaded<Color>>,
    background_color: Option<Cascaded<Option<Color>>>,
    background_gradient: Option<Cascaded<Option<LinearGradient>>>,
    font_family: Option<Cascaded<FontFamily>>,
    font_size_px: Option<Cascaded<i32>>,
    letter_spacing: Option<Cascaded<LetterSpacing>>,
    bold: Option<Cascaded<bool>>,
    underline: Option<Cascaded<bool>>,
    text_align: Option<Cascaded<TextAlign>>,
    text_transform: Option<Cascaded<TextTransform>>,
    line_height: Option<Cascaded<LineHeight>>,
    margin: Option<Cascaded<Edges>>,
    margin_auto: Option<Cascaded<AutoEdges>>,
    border_width: Option<Cascaded<Edges>>,
    border_style: Option<Cascaded<BorderStyle>>,
    border_color: Option<Cascaded<Color>>,
    border_radius_px: Option<Cascaded<i32>>,
    padding: Option<Cascaded<CssEdges>>,
    width_px: Option<Cascaded<Option<CssLength>>>,
    min_width_px: Option<Cascaded<Option<CssLength>>>,
    max_width_px: Option<Cascaded<Option<CssLength>>>,
    height_px: Option<Cascaded<Option<i32>>>,
    min_height_px: Option<Cascaded<Option<i32>>>,
    flex_justify_content: Option<Cascaded<FlexJustifyContent>>,
    flex_align_items: Option<Cascaded<FlexAlignItems>>,
    flex_direction: Option<Cascaded<FlexDirection>>,
    flex_wrap: Option<Cascaded<FlexWrap>>,
    flex_grow: Option<Cascaded<i32>>,
    flex_shrink: Option<Cascaded<i32>>,
    flex_basis_px: Option<Cascaded<Option<i32>>>,
    flex_gap_px: Option<Cascaded<i32>>,
}

impl StyleBuilder {
    pub(super) fn new(base: ComputedStyle, viewport: Option<(i32, i32)>) -> StyleBuilder {
        let custom_properties = base.custom_properties.clone();
        StyleBuilder {
            base,
            viewport,
            custom_properties_declared: HashMap::new(),
            custom_properties,
            display: None,
            visibility: None,
            position: None,
            float: None,
            top_px: None,
            right_px: None,
            bottom_px: None,
            left_px: None,
            opacity: None,
            color: None,
            background_color: None,
            background_gradient: None,
            font_family: None,
            font_size_px: None,
            letter_spacing: None,
            bold: None,
            underline: None,
            text_align: None,
            text_transform: None,
            line_height: None,
            margin: None,
            margin_auto: None,
            border_width: None,
            border_style: None,
            border_color: None,
            border_radius_px: None,
            padding: None,
            width_px: None,
            min_width_px: None,
            max_width_px: None,
            height_px: None,
            min_height_px: None,
            flex_justify_content: None,
            flex_align_items: None,
            flex_direction: None,
            flex_wrap: None,
            flex_grow: None,
            flex_shrink: None,
            flex_basis_px: None,
            flex_gap_px: None,
        }
    }

    pub(super) fn parse_css_length_px(&self, value: &str) -> Option<i32> {
        let (viewport_width_px, viewport_height_px) = match self.viewport {
            Some((width, height)) => (Some(width), Some(height)),
            None => (None, None),
        };
        parse_css_length_px_with_viewport(value, viewport_width_px, viewport_height_px)
    }

    pub(super) fn parse_css_length(&self, value: &str) -> Option<CssLength> {
        let (viewport_width_px, viewport_height_px) = match self.viewport {
            Some((width, height)) => (Some(width), Some(height)),
            None => (None, None),
        };
        length::parse_css_length(value, viewport_width_px, viewport_height_px)
    }

    pub(super) fn finish(self) -> ComputedStyle {
        let font_size_px = self
            .font_size_px
            .map(|v| v.value)
            .unwrap_or(self.base.font_size_px);
        let letter_spacing_px = self
            .letter_spacing
            .map(|v| v.value)
            .unwrap_or(LetterSpacing::Px(self.base.letter_spacing_px))
            .resolve_px(font_size_px);

        ComputedStyle {
            display: self.display.map(|v| v.value).unwrap_or(self.base.display),
            visibility: self
                .visibility
                .map(|v| v.value)
                .unwrap_or(self.base.visibility),
            position: self.position.map(|v| v.value).unwrap_or(self.base.position),
            float: self.float.map(|v| v.value).unwrap_or(self.base.float),
            custom_properties: self.custom_properties,
            top_px: self.top_px.map(|v| v.value).unwrap_or(self.base.top_px),
            right_px: self.right_px.map(|v| v.value).unwrap_or(self.base.right_px),
            bottom_px: self
                .bottom_px
                .map(|v| v.value)
                .unwrap_or(self.base.bottom_px),
            left_px: self.left_px.map(|v| v.value).unwrap_or(self.base.left_px),
            opacity: self.opacity.map(|v| v.value).unwrap_or(self.base.opacity),
            color: self.color.map(|v| v.value).unwrap_or(self.base.color),
            background_color: self
                .background_color
                .map(|v| v.value)
                .unwrap_or(self.base.background_color),
            background_gradient: self
                .background_gradient
                .map(|v| v.value)
                .unwrap_or(self.base.background_gradient),
            font_family: self
                .font_family
                .map(|v| v.value)
                .unwrap_or(self.base.font_family),
            font_size_px,
            letter_spacing_px,
            bold: self.bold.map(|v| v.value).unwrap_or(self.base.bold),
            underline: self
                .underline
                .map(|v| v.value)
                .unwrap_or(self.base.underline),
            text_align: self
                .text_align
                .map(|v| v.value)
                .unwrap_or(self.base.text_align),
            text_transform: self
                .text_transform
                .map(|v| v.value)
                .unwrap_or(self.base.text_transform),
            line_height: self
                .line_height
                .map(|v| v.value)
                .unwrap_or(self.base.line_height),
            margin: self.margin.map(|v| v.value).unwrap_or(self.base.margin),
            margin_auto: self
                .margin_auto
                .map(|v| v.value)
                .unwrap_or(self.base.margin_auto),
            border_width: self
                .border_width
                .map(|v| v.value)
                .unwrap_or(self.base.border_width),
            border_style: self
                .border_style
                .map(|v| v.value)
                .unwrap_or(self.base.border_style),
            border_color: self
                .border_color
                .map(|v| v.value)
                .unwrap_or(self.base.border_color),
            border_radius_px: self
                .border_radius_px
                .map(|v| v.value)
                .unwrap_or(self.base.border_radius_px),
            padding: self.padding.map(|v| v.value).unwrap_or(self.base.padding),
            width_px: self.width_px.map(|v| v.value).unwrap_or(self.base.width_px),
            min_width_px: self
                .min_width_px
                .map(|v| v.value)
                .unwrap_or(self.base.min_width_px),
            max_width_px: self
                .max_width_px
                .map(|v| v.value)
                .unwrap_or(self.base.max_width_px),
            height_px: self
                .height_px
                .map(|v| v.value)
                .unwrap_or(self.base.height_px),
            min_height_px: self
                .min_height_px
                .map(|v| v.value)
                .unwrap_or(self.base.min_height_px),
            flex_justify_content: self
                .flex_justify_content
                .map(|v| v.value)
                .unwrap_or(self.base.flex_justify_content),
            flex_align_items: self
                .flex_align_items
                .map(|v| v.value)
                .unwrap_or(self.base.flex_align_items),
            flex_direction: self
                .flex_direction
                .map(|v| v.value)
                .unwrap_or(self.base.flex_direction),
            flex_wrap: self
                .flex_wrap
                .map(|v| v.value)
                .unwrap_or(self.base.flex_wrap),
            flex_grow: self
                .flex_grow
                .map(|v| v.value)
                .unwrap_or(self.base.flex_grow),
            flex_shrink: self
                .flex_shrink
                .map(|v| v.value)
                .unwrap_or(self.base.flex_shrink),
            flex_basis_px: self
                .flex_basis_px
                .map(|v| v.value)
                .unwrap_or(self.base.flex_basis_px),
            flex_gap_px: self
                .flex_gap_px
                .map(|v| v.value)
                .unwrap_or(self.base.flex_gap_px),
        }
    }

    pub(super) fn apply_presentational_hints(&mut self, element: &Element) {
        let priority = CascadePriority {
            specificity: CascadeSpecificity {
                inline: 0,
                ids: 0,
                classes: 0,
                tags: 0,
            },
            order: 0,
        };

        if element.name == "body" {
            self.apply_margin(
                Edges {
                    top: 8,
                    right: 8,
                    bottom: 8,
                    left: 8,
                },
                priority,
            );
        }

        if matches!(element.name.as_str(), "b" | "strong") {
            self.apply_bold(true, priority);
        }

        match element.name.as_str() {
            "h1" => {
                self.apply_font_size_px(32, priority);
                self.apply_bold(true, priority);
                self.apply_margin(
                    Edges {
                        top: 21,
                        right: 0,
                        bottom: 21,
                        left: 0,
                    },
                    priority,
                );
            }
            "h2" => {
                self.apply_font_size_px(24, priority);
                self.apply_bold(true, priority);
                self.apply_margin(
                    Edges {
                        top: 20,
                        right: 0,
                        bottom: 20,
                        left: 0,
                    },
                    priority,
                );
            }
            "h3" => {
                self.apply_font_size_px(19, priority);
                self.apply_bold(true, priority);
                self.apply_margin(
                    Edges {
                        top: 19,
                        right: 0,
                        bottom: 19,
                        left: 0,
                    },
                    priority,
                );
            }
            _ => {}
        }

        if element.name == "center" {
            self.apply_text_align(TextAlign::Center, priority);
        }

        if element.name == "td" && element.attributes.get("align").is_none() {
            self.apply_text_align(TextAlign::Left, priority);
        }

        if element.name == "font" {
            if let Some(color) = element.attributes.get("color").and_then(parse_css_color) {
                self.apply_color(color, priority);
            }
        }

        if let Some(bg) = element
            .attributes
            .get("bgcolor")
            .and_then(Color::from_css_hex)
        {
            self.apply_background_color(Some(bg), priority);
        }

        if let Some(width) = element
            .attributes
            .get("width")
            .and_then(parse_html_length_px)
        {
            self.apply_width(Some(CssLength::Px(width)), priority);
        }

        if let Some(height) = element
            .attributes
            .get("height")
            .and_then(parse_html_length_px)
        {
            self.apply_height(Some(height), priority);
        }

        if let Some(align) = element.attributes.get("align") {
            let align = match align.trim().to_ascii_lowercase().as_str() {
                "left" => Some(TextAlign::Left),
                "center" => Some(TextAlign::Center),
                "right" => Some(TextAlign::Right),
                _ => None,
            };
            if let Some(align) = align {
                self.apply_text_align(align, priority);
            }
        }
    }

    pub(super) fn apply_matched_custom_properties(&mut self, matched: &[MatchedRule<'_>]) {
        for matched in matched {
            let priority = CascadePriority {
                specificity: CascadeSpecificity::from_selector(matched.specificity),
                order: matched.order,
            };
            for decl in &matched.rule.declarations {
                if !decl.name.starts_with("--") {
                    continue;
                }
                custom_properties::apply_custom_property_declaration(
                    &mut self.custom_properties_declared,
                    &decl.name,
                    &decl.value,
                    priority,
                );
            }
        }
    }

    pub(super) fn apply_inline_style_custom_properties(&mut self, element: &Element) {
        let Some(style_attr) = element.attributes.style.as_deref() else {
            return;
        };

        let priority = CascadePriority {
            specificity: CascadeSpecificity {
                inline: 1,
                ids: 0,
                classes: 0,
                tags: 0,
            },
            order: u32::MAX,
        };

        for decl in crate::css::parse_inline_declarations(style_attr) {
            if !decl.name.starts_with("--") {
                continue;
            }
            custom_properties::apply_custom_property_declaration(
                &mut self.custom_properties_declared,
                &decl.name,
                &decl.value,
                priority,
            );
        }
    }

    pub(super) fn finalize_custom_properties(&mut self) {
        self.custom_properties = CustomProperties::merge(
            &self.base.custom_properties,
            &self.custom_properties_declared,
        );
    }

    pub(super) fn apply_matched_styles(&mut self, matched: &[MatchedRule<'_>]) {
        for matched in matched {
            let priority = CascadePriority {
                specificity: CascadeSpecificity::from_selector(matched.specificity),
                order: matched.order,
            };
            for decl in &matched.rule.declarations {
                if decl.name.starts_with("--") {
                    continue;
                }
                declarations::apply_declaration(self, &decl.name, &decl.value, priority);
            }
        }
    }

    pub(super) fn apply_inline_style(&mut self, element: &Element) {
        let Some(style_attr) = element.attributes.style.as_deref() else {
            return;
        };

        let priority = CascadePriority {
            specificity: CascadeSpecificity {
                inline: 1,
                ids: 0,
                classes: 0,
                tags: 0,
            },
            order: u32::MAX,
        };

        for decl in crate::css::parse_inline_declarations(style_attr) {
            if decl.name.starts_with("--") {
                continue;
            }
            declarations::apply_declaration(self, &decl.name, &decl.value, priority);
        }
    }

    pub(super) fn resolve_vars<'a>(&'a self, value: &'a str) -> Option<std::borrow::Cow<'a, str>> {
        self.custom_properties.resolve_vars(value)
    }

    pub(super) fn apply_display(&mut self, value: Display, priority: CascadePriority) {
        apply_cascade(&mut self.display, value, priority);
    }

    pub(super) fn apply_visibility(&mut self, value: Visibility, priority: CascadePriority) {
        apply_cascade(&mut self.visibility, value, priority);
    }

    pub(super) fn apply_position(&mut self, value: Position, priority: CascadePriority) {
        apply_cascade(&mut self.position, value, priority);
    }

    pub(super) fn apply_float(&mut self, value: Float, priority: CascadePriority) {
        apply_cascade(&mut self.float, value, priority);
    }

    pub(super) fn apply_top(&mut self, value: Option<CssLength>, priority: CascadePriority) {
        apply_cascade(&mut self.top_px, value, priority);
    }

    pub(super) fn apply_right(&mut self, value: Option<CssLength>, priority: CascadePriority) {
        apply_cascade(&mut self.right_px, value, priority);
    }

    pub(super) fn apply_bottom(&mut self, value: Option<CssLength>, priority: CascadePriority) {
        apply_cascade(&mut self.bottom_px, value, priority);
    }

    pub(super) fn apply_left(&mut self, value: Option<CssLength>, priority: CascadePriority) {
        apply_cascade(&mut self.left_px, value, priority);
    }

    pub(super) fn apply_opacity(&mut self, value: u8, priority: CascadePriority) {
        apply_cascade(&mut self.opacity, value, priority);
    }

    pub(super) fn apply_color(&mut self, value: Color, priority: CascadePriority) {
        apply_cascade(&mut self.color, value, priority);
    }

    pub(super) fn apply_background_color(
        &mut self,
        value: Option<Color>,
        priority: CascadePriority,
    ) {
        apply_cascade(&mut self.background_color, value, priority);
    }

    pub(super) fn apply_background_gradient(
        &mut self,
        value: Option<LinearGradient>,
        priority: CascadePriority,
    ) {
        apply_cascade(&mut self.background_gradient, value, priority);
    }

    pub(super) fn apply_font_family(&mut self, value: FontFamily, priority: CascadePriority) {
        apply_cascade(&mut self.font_family, value, priority);
    }

    pub(super) fn apply_font_size_px(&mut self, value: i32, priority: CascadePriority) {
        apply_cascade(&mut self.font_size_px, value, priority);
    }

    pub(super) fn apply_letter_spacing(&mut self, value: LetterSpacing, priority: CascadePriority) {
        apply_cascade(&mut self.letter_spacing, value, priority);
    }

    pub(super) fn apply_bold(&mut self, value: bool, priority: CascadePriority) {
        apply_cascade(&mut self.bold, value, priority);
    }

    pub(super) fn apply_underline(&mut self, value: bool, priority: CascadePriority) {
        apply_cascade(&mut self.underline, value, priority);
    }

    pub(super) fn apply_text_align(&mut self, value: TextAlign, priority: CascadePriority) {
        apply_cascade(&mut self.text_align, value, priority);
    }

    pub(super) fn apply_text_transform(&mut self, value: TextTransform, priority: CascadePriority) {
        apply_cascade(&mut self.text_transform, value, priority);
    }

    pub(super) fn apply_line_height(&mut self, value: LineHeight, priority: CascadePriority) {
        apply_cascade(&mut self.line_height, value, priority);
    }

    pub(super) fn apply_margin(&mut self, value: Edges, priority: CascadePriority) {
        apply_cascade(&mut self.margin, value, priority);
    }

    pub(super) fn apply_margin_auto(&mut self, value: AutoEdges, priority: CascadePriority) {
        apply_cascade(&mut self.margin_auto, value, priority);
    }

    pub(super) fn apply_border_width(&mut self, value: Edges, priority: CascadePriority) {
        apply_cascade(&mut self.border_width, value, priority);
    }

    pub(super) fn apply_border_style(&mut self, value: BorderStyle, priority: CascadePriority) {
        apply_cascade(&mut self.border_style, value, priority);
    }

    pub(super) fn apply_border_color(&mut self, value: Color, priority: CascadePriority) {
        apply_cascade(&mut self.border_color, value, priority);
    }

    pub(super) fn apply_border_radius_px(&mut self, value: i32, priority: CascadePriority) {
        apply_cascade(&mut self.border_radius_px, value, priority);
    }

    pub(super) fn apply_padding(&mut self, value: CssEdges, priority: CascadePriority) {
        apply_cascade(&mut self.padding, value, priority);
    }

    pub(super) fn apply_width(&mut self, value: Option<CssLength>, priority: CascadePriority) {
        apply_cascade(&mut self.width_px, value, priority);
    }

    pub(super) fn apply_min_width(&mut self, value: Option<CssLength>, priority: CascadePriority) {
        apply_cascade(&mut self.min_width_px, value, priority);
    }

    pub(super) fn apply_max_width(&mut self, value: Option<CssLength>, priority: CascadePriority) {
        apply_cascade(&mut self.max_width_px, value, priority);
    }

    pub(super) fn apply_height(&mut self, value: Option<i32>, priority: CascadePriority) {
        apply_cascade(&mut self.height_px, value, priority);
    }

    pub(super) fn apply_min_height(&mut self, value: Option<i32>, priority: CascadePriority) {
        apply_cascade(&mut self.min_height_px, value, priority);
    }

    pub(super) fn apply_flex_justify_content(
        &mut self,
        value: FlexJustifyContent,
        priority: CascadePriority,
    ) {
        apply_cascade(&mut self.flex_justify_content, value, priority);
    }

    pub(super) fn apply_flex_align_items(
        &mut self,
        value: FlexAlignItems,
        priority: CascadePriority,
    ) {
        apply_cascade(&mut self.flex_align_items, value, priority);
    }

    pub(super) fn apply_flex_direction(&mut self, value: FlexDirection, priority: CascadePriority) {
        apply_cascade(&mut self.flex_direction, value, priority);
    }

    pub(super) fn apply_flex_wrap(&mut self, value: FlexWrap, priority: CascadePriority) {
        apply_cascade(&mut self.flex_wrap, value, priority);
    }

    pub(super) fn apply_flex_grow(&mut self, value: i32, priority: CascadePriority) {
        apply_cascade(&mut self.flex_grow, value, priority);
    }

    pub(super) fn apply_flex_shrink(&mut self, value: i32, priority: CascadePriority) {
        apply_cascade(&mut self.flex_shrink, value, priority);
    }

    pub(super) fn apply_flex_basis(&mut self, value: Option<i32>, priority: CascadePriority) {
        apply_cascade(&mut self.flex_basis_px, value, priority);
    }

    pub(super) fn apply_flex_gap_px(&mut self, value: i32, priority: CascadePriority) {
        apply_cascade(&mut self.flex_gap_px, value, priority);
    }

    pub(super) fn apply_padding_component(
        &mut self,
        update: impl FnOnce(CssEdges) -> CssEdges,
        priority: CascadePriority,
    ) {
        let base = self
            .padding
            .as_ref()
            .map(|v| v.value)
            .unwrap_or(self.base.padding);
        let updated = update(base);
        self.apply_padding(updated, priority);
    }

    pub(super) fn apply_margin_component(
        &mut self,
        update: impl FnOnce(Edges) -> Edges,
        priority: CascadePriority,
    ) {
        let base = self
            .margin
            .as_ref()
            .map(|v| v.value)
            .unwrap_or(self.base.margin);
        let updated = update(base);
        self.apply_margin(updated, priority);
    }

    pub(super) fn apply_margin_auto_component(
        &mut self,
        update: impl FnOnce(AutoEdges) -> AutoEdges,
        priority: CascadePriority,
    ) {
        let base = self
            .margin_auto
            .as_ref()
            .map(|v| v.value)
            .unwrap_or(self.base.margin_auto);
        let updated = update(base);
        self.apply_margin_auto(updated, priority);
    }

    pub(super) fn apply_border_width_component(
        &mut self,
        update: impl FnOnce(Edges) -> Edges,
        priority: CascadePriority,
    ) {
        let base = self
            .border_width
            .as_ref()
            .map(|v| v.value)
            .unwrap_or(self.base.border_width);
        let updated = update(base);
        self.apply_border_width(updated, priority);
    }

    pub(super) fn parse_css_line_height(&self, value: &str) -> Option<LineHeight> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return None;
        }

        if trimmed
            .chars()
            .all(|ch| ch.is_ascii_digit() || ch == '.' || ch == '-')
        {
            let multiplier: f32 = trimmed.parse().ok()?;
            return Some(LineHeight::Number(multiplier));
        }

        if trimmed.eq_ignore_ascii_case("normal") {
            return Some(LineHeight::Normal);
        }

        if let Some(px) = self.parse_css_length_px(value) {
            return Some(LineHeight::Px(px));
        }

        None
    }
}

fn apply_cascade<T: Copy>(slot: &mut Option<Cascaded<T>>, value: T, priority: CascadePriority) {
    let should_set = match slot.as_ref() {
        Some(existing) => priority >= existing.priority,
        None => true,
    };
    if should_set {
        *slot = Some(Cascaded { value, priority });
    }
}
