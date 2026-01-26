use crate::css::{PseudoClass, Rule, Selector, Specificity};
use crate::dom::Element;

pub(super) fn match_rule(rule: &Rule, element: &Element, ancestors: &[&Element]) -> Option<(Specificity, u32)> {
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
    if selector.unsupported {
        return false;
    }

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
        PseudoClass::Root => element.name == "html",
    }
}
