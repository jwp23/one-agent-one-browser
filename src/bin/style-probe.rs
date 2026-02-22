use one_agent_one_browser::css::Stylesheet;
use one_agent_one_browser::dom::{Document, Element, Node};
use one_agent_one_browser::js;
use one_agent_one_browser::style::{ComputedStyle, StyleComputer};
use std::env;
use std::sync::Arc;

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            std::process::ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 3 {
        return Err(
            "Usage: style-probe <html-file> <css-file-1> <css-file-2> [--width N] [--height N]"
                .to_owned(),
        );
    }

    let mut width_px = 1366i32;
    let mut height_px = 768i32;
    let mut positional = Vec::new();
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--width" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "Missing value for --width".to_owned())?;
                width_px = value
                    .parse::<i32>()
                    .map_err(|_| format!("Invalid --width value: {value}"))?;
                i += 2;
            }
            "--height" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "Missing value for --height".to_owned())?;
                height_px = value
                    .parse::<i32>()
                    .map_err(|_| format!("Invalid --height value: {value}"))?;
                i += 2;
            }
            _ => {
                positional.push(args[i].clone());
                i += 1;
            }
        }
    }

    if positional.len() != 3 {
        return Err(
            "Usage: style-probe <html-file> <css-file-1> <css-file-2> [--width N] [--height N]"
                .to_owned(),
        );
    }

    let html_source = std::fs::read_to_string(&positional[0])
        .map_err(|err| format!("Failed to read {}: {err}", positional[0]))?;
    let css_one = std::fs::read_to_string(&positional[1])
        .map_err(|err| format!("Failed to read {}: {err}", positional[1]))?;
    let css_two = std::fs::read_to_string(&positional[2])
        .map_err(|err| format!("Failed to read {}: {err}", positional[2]))?;

    let mut document = one_agent_one_browser::html::parse_document(&html_source);
    js::execute_inline_scripts(&mut document);

    let mut sheets = Vec::new();
    let mut inline_css = String::new();
    collect_style_text(&document.root, &mut inline_css);
    if !inline_css.trim().is_empty() {
        sheets.push(Arc::new(Stylesheet::parse(&inline_css)));
    }
    sheets.push(Arc::new(Stylesheet::parse(&css_one)));
    sheets.push(Arc::new(Stylesheet::parse(&css_two)));
    let styles = StyleComputer::from_stylesheets(sheets);

    if let Some(html) = document.find_first_element_by_name("html") {
        println!("html classes: {}", html.attributes.classes.join(" "));
    }

    probe_path(
        &document,
        &styles,
        width_px,
        height_px,
        "vector-main-menu-dropdown-content",
        |element| {
            element.attributes.has_class("vector-dropdown-content")
                && element
                    .attributes
                    .id
                    .as_deref()
                    .is_none_or(|id| id != "vector-appearance-unpinned-container")
        },
        Some("vector-main-menu-dropdown"),
    )?;

    probe_path(
        &document,
        &styles,
        width_px,
        height_px,
        "vector-appearance-dropdown-content",
        |element| element.attributes.has_class("vector-dropdown-content"),
        Some("vector-appearance-dropdown"),
    )?;

    probe_path(
        &document,
        &styles,
        width_px,
        height_px,
        "vector-page-tools-dropdown-content",
        |element| element.attributes.has_class("vector-dropdown-content"),
        Some("vector-page-tools-dropdown"),
    )?;

    probe_path(
        &document,
        &styles,
        width_px,
        height_px,
        "vector-user-links-dropdown-content",
        |element| element.attributes.has_class("vector-dropdown-content"),
        Some("vector-user-links-dropdown"),
    )?;

    probe_path(
        &document,
        &styles,
        width_px,
        height_px,
        "vector-main-menu-dropdown-checkbox",
        |element| element.attributes.id.as_deref() == Some("vector-main-menu-dropdown-checkbox"),
        None,
    )?;

    probe_path(
        &document,
        &styles,
        width_px,
        height_px,
        "vector-main-menu-dropdown-label-text",
        |element| element.attributes.has_class("vector-dropdown-label-text"),
        Some("vector-main-menu-dropdown-label"),
    )?;

    probe_path(
        &document,
        &styles,
        width_px,
        height_px,
        "search-toggle-span",
        |element| element.name == "span",
        Some("p-search"),
    )?;

    probe_path(
        &document,
        &styles,
        width_px,
        height_px,
        "vector-typeahead-search-container",
        |element| element.attributes.has_class("vector-typeahead-search-container"),
        Some("p-search"),
    )?;

    probe_path(
        &document,
        &styles,
        width_px,
        height_px,
        "mw-logo",
        |element| element.attributes.has_class("mw-logo"),
        None,
    )?;

    probe_path(
        &document,
        &styles,
        width_px,
        height_px,
        "mw-body",
        |element| element.attributes.has_class("mw-body"),
        None,
    )?;

    probe_path(
        &document,
        &styles,
        width_px,
        height_px,
        "vector-column-end",
        |element| element.attributes.has_class("vector-column-end"),
        None,
    )?;

    probe_path(
        &document,
        &styles,
        width_px,
        height_px,
        "vector-appearance",
        |element| element.attributes.id.as_deref() == Some("vector-appearance"),
        None,
    )?;

    probe_path(
        &document,
        &styles,
        width_px,
        height_px,
        "oab-appearance-fallback",
        |element| element.attributes.has_class("oab-appearance-fallback"),
        None,
    )?;

    probe_path(
        &document,
        &styles,
        width_px,
        height_px,
        "vector-appearance-first-label",
        |element| element.name == "label",
        Some("vector-appearance"),
    )?;

    Ok(())
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

fn find_path_by<'a, F>(element: &'a Element, path: &mut Vec<&'a Element>, predicate: &F) -> bool
where
    F: Fn(&Element) -> bool,
{
    path.push(element);
    if predicate(element) {
        return true;
    }
    for child in &element.children {
        let Node::Element(el) = child else {
            continue;
        };
        if find_path_by(el, path, predicate) {
            return true;
        }
    }
    let _ = path.pop();
    false
}

fn probe_path<F>(
    document: &Document,
    styles: &StyleComputer,
    width_px: i32,
    height_px: i32,
    label: &str,
    predicate: F,
    within_id: Option<&str>,
) -> Result<(), String>
where
    F: Fn(&Element) -> bool,
{
    let root = document
        .find_first_element_by_name("html")
        .unwrap_or(&document.root);

    let mut full_path = Vec::new();
    if let Some(within_id) = within_id {
        let mut container_path = Vec::new();
        let found_container = find_path_by(root, &mut container_path, &|element| {
            element.attributes.id.as_deref() == Some(within_id)
        });
        if !found_container {
            return Err(format!("Could not find container #{within_id}"));
        }
        let container = container_path
            .last()
            .copied()
            .ok_or_else(|| format!("Could not find container #{within_id}"))?;

        let mut sub_path = Vec::new();
        let found_target = find_path_by(container, &mut sub_path, &predicate);
        if !found_target {
            return Err(format!("Could not find target {label} inside #{within_id}"));
        }

        full_path.extend(container_path);
        full_path.extend(sub_path.into_iter().skip(1));
    } else {
        let found = find_path_by(root, &mut full_path, &predicate);
        if !found {
            return Err(format!("Could not find target {label}"));
        }
    }

    println!("-- {label} --");
    let mut ancestors: Vec<&Element> = Vec::new();
    let mut parent_style = ComputedStyle::root_defaults();
    let mut path_chain = Vec::new();
    for element in &full_path {
        let style = styles.compute_style_in_viewport(
            element,
            &parent_style,
            &ancestors,
            width_px,
            height_px,
        );
        path_chain.push(format!(
            "{}#{} .{} display={:?} visibility={:?} opacity={}",
            element.name,
            element.attributes.id.as_deref().unwrap_or("-"),
            element.attributes.classes.join("."),
            style.display,
            style.visibility,
            style.opacity
        ));
        parent_style = style.clone();
        ancestors.push(element);
        if std::ptr::eq(*element, full_path[full_path.len().saturating_sub(1)]) {
            println!("path:");
            for step in &path_chain {
                println!("  {step}");
            }
            println!(
                "element=<{} id={:?} class=\"{}\">",
                element.name,
                element.attributes.id,
                element.attributes.classes.join(" ")
            );
            let mut text_preview = String::new();
            collect_descendant_text(element, &mut text_preview);
            if !text_preview.trim().is_empty() {
                let preview = text_preview
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                println!("text_preview={preview:?}");
            }
            println!(
                "display={:?} visibility={:?} position={:?} opacity={} height={:?} width={:?} flex_grow={} flex_shrink={} grid_area={:?} font_size={} line_height={:?} color=({}, {}, {})",
                style.display,
                style.visibility,
                style.position,
                style.opacity,
                style.height_px,
                style.width_px,
                style.flex_grow,
                style.flex_shrink,
                style.grid_area,
                style.font_size_px,
                style.line_height,
                style.color.r,
                style.color.g,
                style.color.b
            );
            println!(
                "margin={:?} padding={:?} top={:?} left={:?} grid_columns={:?} grid_areas={:?}",
                style.margin,
                style.padding,
                style.top_px,
                style.left_px,
                style.grid_template_columns,
                style.grid_template_areas
            );
        }
    }

    Ok(())
}

fn collect_descendant_text(element: &Element, out: &mut String) {
    for child in &element.children {
        match child {
            Node::Text(text) => {
                out.push_str(text);
                out.push(' ');
            }
            Node::Element(el) => collect_descendant_text(el, out),
        }
    }
}
