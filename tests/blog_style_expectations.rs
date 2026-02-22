use one_agent_one_browser::dom::{Document, Element, Node};
use one_agent_one_browser::geom::Color;
use one_agent_one_browser::style::{ComputedStyle, StyleComputer};

#[test]
fn blog_test_core_styles_match_expectations() {
    let html_source = std::fs::read_to_string("tests/cases/blog-test.html").unwrap();
    let doc = one_agent_one_browser::html::parse_document(&html_source);
    let styles = StyleComputer::from_document(&doc);

    let (post_link, post_link_ancestors) = find_first_element(&doc, |el| {
        el.name == "a"
            && el
                .attributes
                .get("href")
                .is_some_and(|href| href == "/cursor-implied-success-without-evidence/")
    })
    .expect("post link exists");

    let post_link_style = compute_style_for_element(&styles, post_link, &post_link_ancestors);
    assert_eq!(
        post_link_style.color,
        Color {
            r: 0xb8,
            g: 0xb8,
            b: 0xb8,
            a: 0xff,
        }
    );
    assert_eq!(post_link_style.font_size_px, 17);

    let (post_date, post_date_ancestors) = find_first_element(&doc, |el| {
        el.name == "span" && el.attributes.has_class("post-date")
    })
    .expect("post date exists");
    let post_date_style = compute_style_for_element(&styles, post_date, &post_date_ancestors);
    assert_eq!(
        post_date_style.color,
        Color {
            r: 0x77,
            g: 0x77,
            b: 0x77,
            a: 0xff,
        }
    );
    assert_eq!(post_date_style.font_size_px, 15);

    let (footer, footer_ancestors) =
        find_first_element(&doc, |el| el.name == "footer").expect("footer exists");
    let footer_style = compute_style_for_element(&styles, footer, &footer_ancestors);
    assert_eq!(
        footer_style.color,
        Color {
            r: 0x55,
            g: 0x55,
            b: 0x55,
            a: 0xff,
        }
    );
    assert_eq!(footer_style.font_size_px, 13);
}

fn find_first_element<'a>(
    doc: &'a Document,
    predicate: impl Fn(&Element) -> bool,
) -> Option<(&'a Element, Vec<&'a Element>)> {
    let mut ancestors = Vec::new();
    walk_element(&doc.root, &mut ancestors, &predicate)
}

fn walk_element<'a>(
    element: &'a Element,
    ancestors: &mut Vec<&'a Element>,
    predicate: &impl Fn(&Element) -> bool,
) -> Option<(&'a Element, Vec<&'a Element>)> {
    if predicate(element) {
        return Some((element, ancestors.clone()));
    }

    ancestors.push(element);
    for child in &element.children {
        if let Node::Element(el) = child {
            if let Some(found) = walk_element(el, ancestors, predicate) {
                ancestors.pop();
                return Some(found);
            }
        }
    }
    ancestors.pop();
    None
}

fn compute_style_for_element(
    styles: &StyleComputer,
    element: &Element,
    ancestors: &[&Element],
) -> ComputedStyle {
    let mut parent_style = ComputedStyle::root_defaults();
    for (idx, ancestor) in ancestors.iter().enumerate() {
        parent_style = styles.compute_style(ancestor, &parent_style, &ancestors[..idx]);
    }
    styles.compute_style(element, &parent_style, ancestors)
}
