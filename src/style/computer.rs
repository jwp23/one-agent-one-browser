use super::builder::{MatchedRule, StyleBuilder};
use super::{ComputedStyle, Display};
use crate::css::{CompoundSelector, Stylesheet};
use crate::dom::{Document, Element, Node};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub struct StyleComputer {
    stylesheets: Vec<Arc<Stylesheet>>,
    rules: Vec<RuleRef>,
    index: SelectorIndex,
}

impl StyleComputer {
    pub fn empty() -> StyleComputer {
        StyleComputer {
            stylesheets: Vec::new(),
            rules: Vec::new(),
            index: SelectorIndex::default(),
        }
    }

    pub fn from_css(css_source: &str) -> StyleComputer {
        let sheet = Arc::new(Stylesheet::parse(css_source));
        StyleComputer::from_stylesheets(vec![sheet])
    }

    pub fn from_stylesheets(stylesheets: Vec<Arc<Stylesheet>>) -> StyleComputer {
        let (rules, index) = build_rule_index(&stylesheets);
        StyleComputer {
            stylesheets,
            rules,
            index,
        }
    }

    pub fn from_document(document: &Document) -> StyleComputer {
        let mut css_source = String::new();
        collect_style_text(&document.root, &mut css_source);
        StyleComputer::from_css(&css_source)
    }

    pub fn compute_style(
        &self,
        element: &Element,
        parent: &ComputedStyle,
        ancestors: &[&Element],
    ) -> ComputedStyle {
        self.compute_style_impl(element, parent, ancestors, None)
    }

    pub fn compute_style_in_viewport(
        &self,
        element: &Element,
        parent: &ComputedStyle,
        ancestors: &[&Element],
        viewport_width_px: i32,
        viewport_height_px: i32,
    ) -> ComputedStyle {
        self.compute_style_impl(
            element,
            parent,
            ancestors,
            Some((viewport_width_px.max(0), viewport_height_px.max(0))),
        )
    }

    fn compute_style_impl(
        &self,
        element: &Element,
        parent: &ComputedStyle,
        ancestors: &[&Element],
        viewport: Option<(i32, i32)>,
    ) -> ComputedStyle {
        let display = default_display_for_element(element);
        let style = ComputedStyle::inherit_from(parent, display);
        let mut builder = StyleBuilder::new(style, viewport);

        builder.apply_presentational_hints(element);

        let matched = self.match_rules(element, ancestors);
        builder.apply_matched_custom_properties(&matched);
        builder.apply_inline_style_custom_properties(element);
        builder.finalize_custom_properties();
        builder.apply_matched_styles(&matched);
        builder.apply_inline_style(element);

        builder.finish()
    }

    fn match_rules<'a>(&'a self, element: &Element, ancestors: &[&Element]) -> Vec<MatchedRule<'a>> {
        let mut seen = HashSet::<usize>::new();
        let mut matched = Vec::<MatchedRule<'a>>::new();

        let mut consider = |rule_id: usize| {
            if !seen.insert(rule_id) {
                return;
            }
            let Some(rule_ref) = self.rules.get(rule_id) else {
                return;
            };
            let Some(sheet) = self.stylesheets.get(rule_ref.sheet_index) else {
                return;
            };
            let Some(rule) = sheet.rules.get(rule_ref.rule_index) else {
                return;
            };
            let Some((specificity, _)) = super::selectors::match_rule(rule, element, ancestors)
            else {
                return;
            };
            matched.push(MatchedRule {
                rule,
                specificity,
                order: rule_ref.order,
            });
        };

        for &rule_id in &self.index.universal {
            consider(rule_id);
        }

        if let Some(id) = element.attributes.id.as_deref() {
            if let Some(rule_ids) = self.index.by_id.get(id) {
                for &rule_id in rule_ids {
                    consider(rule_id);
                }
            }
        }

        for class in &element.attributes.classes {
            if let Some(rule_ids) = self.index.by_class.get(class) {
                for &rule_id in rule_ids {
                    consider(rule_id);
                }
            }
        }

        if let Some(rule_ids) = self.index.by_tag.get(element.name.as_str()) {
            for &rule_id in rule_ids {
                consider(rule_id);
            }
        }

        matched.sort_by_key(|rule| rule.order);
        matched
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
    if element.name == "#document" {
        return Display::Block;
    }

    if matches!(
        element.name.as_str(),
        "head" | "style" | "script" | "meta" | "link" | "title"
    ) {
        return Display::None;
    }

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
        "html" | "body" | "div" | "p" | "center" | "header" | "main" | "footer" | "nav" | "ul"
        | "ol" | "li" | "h1" | "h2" | "h3" | "blockquote" | "pre" => Display::Block,
        "img" | "svg" | "button" | "input" => Display::InlineBlock,
        "br" => Display::Inline,
        _ => Display::Inline,
    }
}

#[derive(Clone, Copy, Debug)]
struct RuleRef {
    sheet_index: usize,
    rule_index: usize,
    order: u32,
}

#[derive(Default)]
struct SelectorIndex {
    by_id: HashMap<String, Vec<usize>>,
    by_class: HashMap<String, Vec<usize>>,
    by_tag: HashMap<String, Vec<usize>>,
    universal: Vec<usize>,
}

impl SelectorIndex {
    fn insert_rule(&mut self, rule_id: usize, rule: &crate::css::Rule) {
        for selector in &rule.selectors {
            let Some(last) = selector.parts.last() else {
                continue;
            };
            match selector_bucket_key(last) {
                SelectorBucketKey::Id(id) => self.by_id.entry(id.to_owned()).or_default().push(rule_id),
                SelectorBucketKey::Class(classes) => {
                    for class in classes {
                        self.by_class.entry(class.to_owned()).or_default().push(rule_id);
                    }
                }
                SelectorBucketKey::Tag(tag) => self.by_tag.entry(tag.to_owned()).or_default().push(rule_id),
                SelectorBucketKey::Universal => self.universal.push(rule_id),
            }
        }
    }
}

enum SelectorBucketKey<'a> {
    Id(&'a str),
    Class(&'a [String]),
    Tag(&'a str),
    Universal,
}

fn selector_bucket_key(last: &CompoundSelector) -> SelectorBucketKey<'_> {
    if let Some(id) = last.id.as_deref() {
        return SelectorBucketKey::Id(id);
    }
    if !last.classes.is_empty() {
        return SelectorBucketKey::Class(&last.classes);
    }
    if let Some(tag) = last.tag.as_deref() {
        return SelectorBucketKey::Tag(tag);
    }
    SelectorBucketKey::Universal
}

fn build_rule_index(stylesheets: &[Arc<Stylesheet>]) -> (Vec<RuleRef>, SelectorIndex) {
    let mut rules = Vec::new();
    let mut index = SelectorIndex::default();
    let mut order: u32 = 0;

    for (sheet_index, sheet) in stylesheets.iter().enumerate() {
        for (rule_index, rule) in sheet.rules.iter().enumerate() {
            let rule_id = rules.len();
            rules.push(RuleRef {
                sheet_index,
                rule_index,
                order,
            });
            order = order.saturating_add(1);
            index.insert_rule(rule_id, rule);
        }
    }

    (rules, index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Color;

    #[test]
    fn selector_matches_descendant() {
        let doc = crate::html::parse_document("<div class='a'><span><b>t</b></span></div>");
        let computer = StyleComputer::from_css(".a b { color: #ffffff; }");
        let root_style = ComputedStyle::root_defaults();
        let div = doc
            .find_first_element_by_name("div")
            .expect("div element exists");
        let span = div
            .find_first_element_by_name("span")
            .expect("span element exists");
        let b = span.find_first_element_by_name("b").expect("b exists");
        let ancestors = vec![div, span];

        let style = computer.compute_style(b, &root_style, &ancestors);
        assert_eq!(style.color, Color::WHITE);
    }
}
