use crate::css::{PseudoClass, Rule, Selector, Specificity, Stylesheet};
use crate::dom::{Document, Element, Node};
use crate::geom::{Color, Edges};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Display {
    Block,
    Inline,
    Table,
    TableRow,
    TableCell,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    Visible,
    Hidden,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontFamily {
    SansSerif,
    Monospace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComputedStyle {
    pub display: Display,
    pub visibility: Visibility,
    pub color: Color,
    pub background_color: Option<Color>,
    pub font_family: FontFamily,
    pub font_size_px: i32,
    pub bold: bool,
    pub underline: bool,
    pub text_align: TextAlign,
    pub line_height_px: Option<i32>,
    pub margin: Edges,
    pub padding: Edges,
    pub width_px: Option<i32>,
    pub min_width_px: Option<i32>,
    pub height_px: Option<i32>,
}

impl ComputedStyle {
    pub fn root_defaults() -> ComputedStyle {
        ComputedStyle {
            display: Display::Block,
            visibility: Visibility::Visible,
            color: Color::BLACK,
            background_color: None,
            font_family: FontFamily::SansSerif,
            font_size_px: 16,
            bold: false,
            underline: false,
            text_align: TextAlign::Left,
            line_height_px: None,
            margin: Edges::ZERO,
            padding: Edges::ZERO,
            width_px: None,
            min_width_px: None,
            height_px: None,
        }
    }

    pub fn inherit_from(parent: &ComputedStyle, display: Display) -> ComputedStyle {
        ComputedStyle {
            display,
            visibility: Visibility::Visible,
            color: parent.color,
            background_color: None,
            font_family: parent.font_family,
            font_size_px: parent.font_size_px,
            bold: parent.bold,
            underline: parent.underline,
            text_align: parent.text_align,
            line_height_px: parent.line_height_px,
            margin: Edges::ZERO,
            padding: Edges::ZERO,
            width_px: None,
            min_width_px: None,
            height_px: None,
        }
    }
}

pub struct StyleComputer {
    stylesheet: Stylesheet,
}

impl StyleComputer {
    pub fn from_document(document: &Document) -> StyleComputer {
        let mut css_source = String::new();
        collect_style_text(&document.root, &mut css_source);
        StyleComputer {
            stylesheet: Stylesheet::parse(&css_source),
        }
    }

    pub fn compute_style(
        &self,
        element: &Element,
        parent: &ComputedStyle,
        ancestors: &[&Element],
    ) -> ComputedStyle {
        let display = default_display_for_element(element);
        let style = ComputedStyle::inherit_from(parent, display);
        let mut builder = StyleBuilder::new(style);

        builder.apply_presentational_hints(element);
        builder.apply_stylesheet(&self.stylesheet, element, ancestors);
        builder.apply_inline_style(element);

        builder.finish()
    }
}

fn collect_style_text(element: &Element, out: &mut String) {
    if element.name == "style" {
        for child in &element.children {
            if let Node::Text(text) = child {
                out.push_str(text);
                out.push('\n');
            }
        }
    }

    for child in &element.children {
        if let Node::Element(el) = child {
            collect_style_text(el, out);
        }
    }
}

fn default_display_for_element(element: &Element) -> Display {
    if element.name == "table" {
        return Display::Table;
    }
    if element.name == "tr" {
        return Display::TableRow;
    }
    if element.name == "td" {
        return Display::TableCell;
    }

    match element.name.as_str() {
        "html" | "body" | "div" | "p" | "center" => Display::Block,
        "br" => Display::Inline,
        _ => Display::Inline,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct CascadePriority {
    specificity: CascadeSpecificity,
    order: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct CascadeSpecificity {
    inline: u16,
    ids: u16,
    classes: u16,
    tags: u16,
}

impl CascadeSpecificity {
    fn from_selector(specificity: Specificity) -> CascadeSpecificity {
        CascadeSpecificity {
            inline: 0,
            ids: specificity.ids,
            classes: specificity.classes,
            tags: specificity.tags,
        }
    }
}

struct Cascaded<T> {
    value: T,
    priority: CascadePriority,
}

struct StyleBuilder {
    base: ComputedStyle,
    display: Option<Cascaded<Display>>,
    visibility: Option<Cascaded<Visibility>>,
    color: Option<Cascaded<Color>>,
    background_color: Option<Cascaded<Option<Color>>>,
    font_family: Option<Cascaded<FontFamily>>,
    font_size_px: Option<Cascaded<i32>>,
    bold: Option<Cascaded<bool>>,
    underline: Option<Cascaded<bool>>,
    text_align: Option<Cascaded<TextAlign>>,
    line_height_px: Option<Cascaded<Option<i32>>>,
    margin: Option<Cascaded<Edges>>,
    padding: Option<Cascaded<Edges>>,
    width_px: Option<Cascaded<Option<i32>>>,
    min_width_px: Option<Cascaded<Option<i32>>>,
    height_px: Option<Cascaded<Option<i32>>>,
}

impl StyleBuilder {
    fn new(base: ComputedStyle) -> StyleBuilder {
        StyleBuilder {
            base,
            display: None,
            visibility: None,
            color: None,
            background_color: None,
            font_family: None,
            font_size_px: None,
            bold: None,
            underline: None,
            text_align: None,
            line_height_px: None,
            margin: None,
            padding: None,
            width_px: None,
            min_width_px: None,
            height_px: None,
        }
    }

    fn finish(self) -> ComputedStyle {
        ComputedStyle {
            display: self.display.map(|v| v.value).unwrap_or(self.base.display),
            visibility: self
                .visibility
                .map(|v| v.value)
                .unwrap_or(self.base.visibility),
            color: self.color.map(|v| v.value).unwrap_or(self.base.color),
            background_color: self
                .background_color
                .map(|v| v.value)
                .unwrap_or(self.base.background_color),
            font_family: self
                .font_family
                .map(|v| v.value)
                .unwrap_or(self.base.font_family),
            font_size_px: self
                .font_size_px
                .map(|v| v.value)
                .unwrap_or(self.base.font_size_px),
            bold: self.bold.map(|v| v.value).unwrap_or(self.base.bold),
            underline: self
                .underline
                .map(|v| v.value)
                .unwrap_or(self.base.underline),
            text_align: self
                .text_align
                .map(|v| v.value)
                .unwrap_or(self.base.text_align),
            line_height_px: self
                .line_height_px
                .map(|v| v.value)
                .unwrap_or(self.base.line_height_px),
            margin: self.margin.map(|v| v.value).unwrap_or(self.base.margin),
            padding: self.padding.map(|v| v.value).unwrap_or(self.base.padding),
            width_px: self.width_px.map(|v| v.value).unwrap_or(self.base.width_px),
            min_width_px: self
                .min_width_px
                .map(|v| v.value)
                .unwrap_or(self.base.min_width_px),
            height_px: self
                .height_px
                .map(|v| v.value)
                .unwrap_or(self.base.height_px),
        }
    }

    fn apply_presentational_hints(&mut self, element: &Element) {
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

        if let Some(bg) = element.attributes.get("bgcolor").and_then(Color::from_css_hex) {
            self.apply_background_color(Some(bg), priority);
        }

        if let Some(width) = element.attributes.get("width").and_then(parse_html_length_px) {
            self.apply_width(Some(width), priority);
        }

        if let Some(height) = element.attributes.get("height").and_then(parse_html_length_px) {
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

    fn apply_stylesheet(&mut self, sheet: &Stylesheet, element: &Element, ancestors: &[&Element]) {
        for rule in &sheet.rules {
            let Some((specificity, order)) = match_rule(rule, element, ancestors) else {
                continue;
            };
            let priority = CascadePriority {
                specificity: CascadeSpecificity::from_selector(specificity),
                order,
            };
            for decl in &rule.declarations {
                self.apply_declaration(&decl.name, &decl.value, priority);
            }
        }
    }

    fn apply_inline_style(&mut self, element: &Element) {
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
            self.apply_declaration(&decl.name, &decl.value, priority);
        }
    }

    fn apply_declaration(&mut self, name: &str, value: &str, priority: CascadePriority) {
        match name {
            "display" => {
                if value.eq_ignore_ascii_case("none") {
                    self.apply_display(Display::None, priority);
                } else if value.eq_ignore_ascii_case("block") {
                    self.apply_display(Display::Block, priority);
                } else if value.eq_ignore_ascii_case("inline") {
                    self.apply_display(Display::Inline, priority);
                }
            }
            "visibility" => {
                if value.eq_ignore_ascii_case("hidden") {
                    self.apply_visibility(Visibility::Hidden, priority);
                } else if value.eq_ignore_ascii_case("visible") {
                    self.apply_visibility(Visibility::Visible, priority);
                }
            }
            "color" => {
                if let Some(color) = parse_css_color(value) {
                    self.apply_color(color, priority);
                }
            }
            "background-color" => {
                if let Some(color) = parse_css_color(value) {
                    self.apply_background_color(Some(color), priority);
                } else if value.eq_ignore_ascii_case("transparent") {
                    self.apply_background_color(None, priority);
                }
            }
            "font-family" => {
                let family = if value.to_ascii_lowercase().contains("monospace") {
                    FontFamily::Monospace
                } else {
                    FontFamily::SansSerif
                };
                self.apply_font_family(family, priority);
            }
            "font-size" => {
                if let Some(px) = parse_css_length_px(value) {
                    self.apply_font_size_px(px, priority);
                }
            }
            "font-weight" => {
                if value.eq_ignore_ascii_case("bold") {
                    self.apply_bold(true, priority);
                } else if value.eq_ignore_ascii_case("normal") {
                    self.apply_bold(false, priority);
                }
            }
            "text-decoration" => {
                if value.eq_ignore_ascii_case("underline") {
                    self.apply_underline(true, priority);
                } else if value.eq_ignore_ascii_case("none") {
                    self.apply_underline(false, priority);
                }
            }
            "text-align" => {
                let align = match value.trim().to_ascii_lowercase().as_str() {
                    "left" => Some(TextAlign::Left),
                    "center" => Some(TextAlign::Center),
                    "right" => Some(TextAlign::Right),
                    _ => None,
                };
                if let Some(align) = align {
                    self.apply_text_align(align, priority);
                }
            }
            "line-height" => {
                if let Some(px) = parse_css_length_px(value) {
                    self.apply_line_height_px(Some(px), priority);
                } else if value.eq_ignore_ascii_case("normal") {
                    self.apply_line_height_px(None, priority);
                }
            }
            "padding" => {
                if let Some(edges) = parse_css_box_edges(value) {
                    self.apply_padding(edges, priority);
                }
            }
            "padding-left" => {
                if let Some(px) = parse_css_length_px(value) {
                    self.apply_padding_component(|e| Edges {
                        left: px,
                        ..e
                    }, priority);
                }
            }
            "padding-right" => {
                if let Some(px) = parse_css_length_px(value) {
                    self.apply_padding_component(|e| Edges {
                        right: px,
                        ..e
                    }, priority);
                }
            }
            "padding-top" => {
                if let Some(px) = parse_css_length_px(value) {
                    self.apply_padding_component(|e| Edges {
                        top: px,
                        ..e
                    }, priority);
                }
            }
            "padding-bottom" => {
                if let Some(px) = parse_css_length_px(value) {
                    self.apply_padding_component(|e| Edges {
                        bottom: px,
                        ..e
                    }, priority);
                }
            }
            "margin" => {
                if let Some(edges) = parse_css_box_edges(value) {
                    self.apply_margin(edges, priority);
                }
            }
            "margin-left" => {
                if let Some(px) = parse_css_length_px(value) {
                    self.apply_margin_component(|e| Edges {
                        left: px,
                        ..e
                    }, priority);
                }
            }
            "margin-right" => {
                if let Some(px) = parse_css_length_px(value) {
                    self.apply_margin_component(|e| Edges {
                        right: px,
                        ..e
                    }, priority);
                }
            }
            "margin-top" => {
                if let Some(px) = parse_css_length_px(value) {
                    self.apply_margin_component(|e| Edges {
                        top: px,
                        ..e
                    }, priority);
                }
            }
            "margin-bottom" => {
                if let Some(px) = parse_css_length_px(value) {
                    self.apply_margin_component(|e| Edges {
                        bottom: px,
                        ..e
                    }, priority);
                }
            }
            "width" => {
                if let Some(px) = parse_css_length_px(value) {
                    self.apply_width(Some(px), priority);
                }
            }
            "min-width" => {
                if let Some(px) = parse_css_length_px(value) {
                    self.apply_min_width(Some(px), priority);
                }
            }
            "height" => {
                if let Some(px) = parse_css_length_px(value) {
                    self.apply_height(Some(px), priority);
                }
            }
            _ => {}
        }
    }

    fn apply_display(&mut self, value: Display, priority: CascadePriority) {
        apply_cascade(&mut self.display, value, priority);
    }

    fn apply_visibility(&mut self, value: Visibility, priority: CascadePriority) {
        apply_cascade(&mut self.visibility, value, priority);
    }

    fn apply_color(&mut self, value: Color, priority: CascadePriority) {
        apply_cascade(&mut self.color, value, priority);
    }

    fn apply_background_color(&mut self, value: Option<Color>, priority: CascadePriority) {
        apply_cascade(&mut self.background_color, value, priority);
    }

    fn apply_font_family(&mut self, value: FontFamily, priority: CascadePriority) {
        apply_cascade(&mut self.font_family, value, priority);
    }

    fn apply_font_size_px(&mut self, value: i32, priority: CascadePriority) {
        apply_cascade(&mut self.font_size_px, value, priority);
    }

    fn apply_bold(&mut self, value: bool, priority: CascadePriority) {
        apply_cascade(&mut self.bold, value, priority);
    }

    fn apply_underline(&mut self, value: bool, priority: CascadePriority) {
        apply_cascade(&mut self.underline, value, priority);
    }

    fn apply_text_align(&mut self, value: TextAlign, priority: CascadePriority) {
        apply_cascade(&mut self.text_align, value, priority);
    }

    fn apply_line_height_px(&mut self, value: Option<i32>, priority: CascadePriority) {
        apply_cascade(&mut self.line_height_px, value, priority);
    }

    fn apply_margin(&mut self, value: Edges, priority: CascadePriority) {
        apply_cascade(&mut self.margin, value, priority);
    }

    fn apply_padding(&mut self, value: Edges, priority: CascadePriority) {
        apply_cascade(&mut self.padding, value, priority);
    }

    fn apply_width(&mut self, value: Option<i32>, priority: CascadePriority) {
        apply_cascade(&mut self.width_px, value, priority);
    }

    fn apply_min_width(&mut self, value: Option<i32>, priority: CascadePriority) {
        apply_cascade(&mut self.min_width_px, value, priority);
    }

    fn apply_height(&mut self, value: Option<i32>, priority: CascadePriority) {
        apply_cascade(&mut self.height_px, value, priority);
    }

    fn apply_margin_component<F>(&mut self, update: F, priority: CascadePriority)
    where
        F: FnOnce(Edges) -> Edges,
    {
        let current = self.margin.as_ref().map(|v| v.value).unwrap_or(self.base.margin);
        self.apply_margin(update(current), priority);
    }

    fn apply_padding_component<F>(&mut self, update: F, priority: CascadePriority)
    where
        F: FnOnce(Edges) -> Edges,
    {
        let current = self
            .padding
            .as_ref()
            .map(|v| v.value)
            .unwrap_or(self.base.padding);
        self.apply_padding(update(current), priority);
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

fn match_rule(rule: &Rule, element: &Element, ancestors: &[&Element]) -> Option<(Specificity, u32)> {
    let mut best: Option<Specificity> = None;
    for selector in &rule.selectors {
        if selector_matches(selector, element, ancestors) {
            let spec = selector.specificity();
            best = Some(best.map_or(spec, |b| b.max(spec)));
        }
    }
    best.map(|spec| (spec, rule.order))
}

fn selector_matches(selector: &Selector, element: &Element, ancestors: &[&Element]) -> bool {
    if selector.parts.is_empty() {
        return false;
    }

    if !compound_matches(&selector.parts[selector.parts.len() - 1], element) {
        return false;
    }

    let mut ancestor_index = ancestors.len();
    for part in selector.parts[..selector.parts.len() - 1].iter().rev() {
        let mut matched = false;
        while ancestor_index > 0 {
            ancestor_index -= 1;
            if compound_matches(part, ancestors[ancestor_index]) {
                matched = true;
                break;
            }
        }
        if !matched {
            return false;
        }
    }

    true
}

fn compound_matches(selector: &crate::css::CompoundSelector, element: &Element) -> bool {
    if let Some(tag) = selector.tag.as_deref() {
        if element.name != tag {
            return false;
        }
    }

    if let Some(id) = selector.id.as_deref() {
        if element.attributes.id.as_deref() != Some(id) {
            return false;
        }
    }

    for class in &selector.classes {
        if !element.attributes.has_class(class) {
            return false;
        }
    }

    for attr in &selector.attributes {
        let Some(value) = element.attributes.get(&attr.name) else {
            return false;
        };
        if let Some(expected) = attr.value.as_deref() {
            if value != expected {
                return false;
            }
        }
    }

    for pseudo in &selector.pseudo_classes {
        if !pseudo_matches(*pseudo, element) {
            return false;
        }
    }

    true
}

fn pseudo_matches(pseudo: PseudoClass, element: &Element) -> bool {
    match pseudo {
        PseudoClass::Link => element.name == "a" && element.attributes.get("href").is_some(),
        PseudoClass::Visited => false,
        PseudoClass::Hover => false,
    }
}

fn parse_css_color(value: &str) -> Option<Color> {
    let value = value.trim();
    if let Some(color) = Color::from_css_hex(value) {
        return Some(color);
    }
    match value.to_ascii_lowercase().as_str() {
        "black" => Some(Color::BLACK),
        "white" => Some(Color::WHITE),
        _ => None,
    }
}

fn parse_css_length_px(value: &str) -> Option<i32> {
    let value = value.trim();
    if value == "0" {
        return Some(0);
    }

    let mut end = 0usize;
    for (idx, ch) in value.char_indices() {
        if !(ch.is_ascii_digit() || ch == '.' || ch == '-') {
            break;
        }
        end = idx + ch.len_utf8();
    }
    if end == 0 {
        return None;
    }

    let number: f32 = value[..end].parse().ok()?;
    let unit = value[end..].trim().to_ascii_lowercase();
    let px = match unit.as_str() {
        "px" | "" => number,
        "pt" => number * (96.0 / 72.0),
        _ => return None,
    };
    Some(px.round() as i32)
}

fn parse_css_box_edges(value: &str) -> Option<Edges> {
    let lengths: Vec<i32> = value
        .split_whitespace()
        .filter_map(parse_css_length_px)
        .collect();

    match lengths.as_slice() {
        [] => None,
        [all] => Some(Edges {
            top: *all,
            right: *all,
            bottom: *all,
            left: *all,
        }),
        [vertical, horizontal] => Some(Edges {
            top: *vertical,
            right: *horizontal,
            bottom: *vertical,
            left: *horizontal,
        }),
        [top, horizontal, bottom] => Some(Edges {
            top: *top,
            right: *horizontal,
            bottom: *bottom,
            left: *horizontal,
        }),
        [top, right, bottom, left] => Some(Edges {
            top: *top,
            right: *right,
            bottom: *bottom,
            left: *left,
        }),
        _ => None,
    }
}

fn parse_html_length_px(value: &str) -> Option<i32> {
    let value = value.trim();
    if value.ends_with('%') {
        return None;
    }

    parse_css_length_px(value).or_else(|| value.parse::<i32>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selector_matches_descendant() {
        let doc = crate::html::parse_document("<div class='a'><span><b>t</b></span></div>");
        let computer = StyleComputer {
            stylesheet: Stylesheet::parse(".a b { color: #ffffff; }"),
        };
        let root_style = ComputedStyle::root_defaults();
        let root = doc.render_root();
        let mut ancestors = Vec::new();

        fn find_b<'a>(el: &'a Element) -> &'a Element {
            el.find_first_element_by_name("b").unwrap()
        }

        let b = find_b(root);
        ancestors.push(root);
        let style = computer.compute_style(b, &root_style, &ancestors);
        assert_eq!(style.color, Color::WHITE);
    }
}
