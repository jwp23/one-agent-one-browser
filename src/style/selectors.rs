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

    if !compound_matches(&selector.parts[selector.parts.len() - 1], element, ancestors) {
        return false;
    }

    let mut ancestor_index = ancestors.len();
    for part in selector.parts[..selector.parts.len() - 1].iter().rev() {
        let mut matched = false;
        while ancestor_index > 0 {
            ancestor_index -= 1;
            if compound_matches(part, ancestors[ancestor_index], &ancestors[..ancestor_index]) {
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

fn compound_matches(selector: &crate::css::CompoundSelector, element: &Element, ancestors: &[&Element]) -> bool {
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
        if !pseudo_matches(*pseudo, element, ancestors) {
            return false;
        }
    }

    true
}

fn pseudo_matches(pseudo: PseudoClass, element: &Element, ancestors: &[&Element]) -> bool {
    match pseudo {
        PseudoClass::Link => element.name == "a" && element.attributes.get("href").is_some(),
        PseudoClass::Visited => false,
        PseudoClass::Hover => false,
        PseudoClass::Root => element.name == "html",
        PseudoClass::NthChild(pattern) => nth_child_matches(element, ancestors, pattern),
    }
}

fn nth_child_matches(element: &Element, ancestors: &[&Element], pattern: crate::css::NthChildPattern) -> bool {
    let Some(parent) = ancestors.last() else {
        return false;
    };
    let Some(index) = nth_child_index(parent, element) else {
        return false;
    };
    matches_an_plus_b(index, pattern.a, pattern.b)
}

fn nth_child_index(parent: &Element, element: &Element) -> Option<usize> {
    let mut index = 0usize;
    for child in &parent.children {
        let crate::dom::Node::Element(el) = child else {
            continue;
        };
        index = index.saturating_add(1);
        if std::ptr::eq(el, element) {
            return Some(index);
        }
    }
    None
}

fn matches_an_plus_b(index: usize, a: i32, b: i32) -> bool {
    if index == 0 {
        return false;
    }
    let index = index.min(i32::MAX as usize) as i32;

    if a == 0 {
        return index == b;
    }

    if a > 0 {
        if index < b {
            return false;
        }
        (index - b) % a == 0
    } else {
        if index > b {
            return false;
        }
        let step = -a;
        (b - index) % step == 0
    }
}
