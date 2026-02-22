use crate::dom::{Document, Element, Node};

pub fn execute_inline_scripts(document: &mut Document) {
    let mut scripts = Vec::new();
    collect_inline_classic_scripts(&document.root, &mut scripts);

    for source in scripts {
        if let Some(classes) = parse_document_element_class_name_assignment(&source)
            && !should_skip_root_class_assignment(document, &classes)
            && let Some(html) = document.find_first_element_by_name_mut("html")
        {
            html.attributes.classes = classes.split_whitespace().map(str::to_owned).collect();
        }

        for assignment in parse_text_content_assignments(&source) {
            if let Some(element) = document.find_first_element_by_id_mut(&assignment.element_id) {
                element.set_text_content(assignment.text);
            }
        }
    }

    inject_vector_appearance_fallback(document);
}

fn should_skip_root_class_assignment(document: &Document, assigned_classes: &str) -> bool {
    // We intentionally keep server-rendered no-JS classes unless we have a full JS runtime.
    let Some(html) = document.find_first_element_by_name("html") else {
        return false;
    };

    html.attributes.has_class("client-nojs")
        && assigned_classes
            .split_whitespace()
            .any(|class_name| class_name == "client-js")
}

#[derive(Debug, PartialEq, Eq)]
struct TextContentAssignment {
    element_id: String,
    text: String,
}

fn collect_inline_classic_scripts(element: &Element, out: &mut Vec<String>) {
    if element.name == "script"
        && is_classic_javascript_type(element.attributes.get("type"))
        && element.attributes.get("src").is_none()
    {
        let mut source = String::new();
        for child in &element.children {
            if let Node::Text(text) = child {
                source.push_str(text);
                source.push('\n');
            }
        }
        out.push(source);
    }

    for child in &element.children {
        if let Node::Element(el) = child {
            collect_inline_classic_scripts(el, out);
        }
    }
}

fn is_classic_javascript_type(script_type: Option<&str>) -> bool {
    let Some(script_type) = script_type else {
        return true;
    };

    let script_type = script_type.trim();
    if script_type.is_empty() {
        return true;
    }

    let mime = script_type
        .split(';')
        .next()
        .unwrap_or(script_type)
        .trim()
        .to_ascii_lowercase();

    matches!(
        mime.as_str(),
        "text/javascript" | "application/javascript" | "text/ecmascript" | "application/ecmascript"
    )
}

fn parse_text_content_assignments(script: &str) -> Vec<TextContentAssignment> {
    const GET_BY_ID: &str = "document.getElementById";

    let mut out = Vec::new();
    let mut cursor = 0usize;

    while cursor < script.len() {
        let Some(offset) = script[cursor..].find(GET_BY_ID) else {
            break;
        };
        let start = cursor + offset;
        let next = match parse_text_content_assignment(script, start) {
            Some((assignment, next)) => {
                out.push(assignment);
                next
            }
            None => start + GET_BY_ID.len(),
        };
        cursor = next.min(script.len());
    }

    out
}

fn parse_document_element_class_name_assignment(script: &str) -> Option<String> {
    const MARKER: &str = "document.documentElement.className";
    let start = script.find(MARKER)?;
    let mut cursor = start + MARKER.len();
    cursor = skip_whitespace(script, cursor);
    cursor = consume_char(script, cursor, '=')?;
    cursor = skip_whitespace(script, cursor);

    let ch = script[cursor..].chars().next()?;
    if ch == '\'' || ch == '"' {
        let (class_value, _) = parse_js_string_literal(script, cursor)?;
        return Some(class_value);
    }

    let (identifier, _) = parse_js_identifier(script, cursor)?;
    parse_js_variable_string_literal(script, identifier.as_str())
}

fn parse_js_variable_string_literal(script: &str, variable_name: &str) -> Option<String> {
    for keyword in ["var", "let", "const"] {
        let mut cursor = 0usize;
        while cursor < script.len() {
            let Some(offset) = script[cursor..].find(keyword) else {
                break;
            };
            let start = cursor + offset;
            let before_ok = start == 0
                || !script[..start]
                    .chars()
                    .next_back()
                    .is_some_and(is_js_identifier_char);
            if !before_ok {
                cursor = start + keyword.len();
                continue;
            }
            let mut pos = start + keyword.len();
            pos = skip_whitespace(script, pos);
            let Some((name, next)) = parse_js_identifier(script, pos) else {
                cursor = start + keyword.len();
                continue;
            };
            if name != variable_name {
                cursor = next;
                continue;
            }
            pos = skip_whitespace(script, next);
            let Some(after_equals) = consume_char(script, pos, '=') else {
                cursor = next;
                continue;
            };
            let value_start = skip_whitespace(script, after_equals);
            let (value, _) = parse_js_string_literal(script, value_start)?;
            return Some(value);
        }
    }
    None
}

fn parse_js_identifier(source: &str, start: usize) -> Option<(String, usize)> {
    let mut cursor = start;
    let first = source[cursor..].chars().next()?;
    if !is_js_identifier_start_char(first) {
        return None;
    }
    cursor += first.len_utf8();

    while cursor < source.len() {
        let Some(ch) = source[cursor..].chars().next() else {
            break;
        };
        if !is_js_identifier_char(ch) {
            break;
        }
        cursor += ch.len_utf8();
    }

    Some((source[start..cursor].to_owned(), cursor))
}

fn is_js_identifier_start_char(ch: char) -> bool {
    ch == '_' || ch == '$' || ch.is_ascii_alphabetic()
}

fn is_js_identifier_char(ch: char) -> bool {
    is_js_identifier_start_char(ch) || ch.is_ascii_digit()
}

fn parse_text_content_assignment(
    script: &str,
    start: usize,
) -> Option<(TextContentAssignment, usize)> {
    const GET_BY_ID: &str = "document.getElementById";
    const TEXT_CONTENT: &str = "textContent";

    if !script[start..].starts_with(GET_BY_ID) {
        return None;
    }

    let mut cursor = start + GET_BY_ID.len();
    cursor = skip_whitespace(script, cursor);
    cursor = consume_char(script, cursor, '(')?;
    cursor = skip_whitespace(script, cursor);

    let (element_id, next) = parse_js_string_literal(script, cursor)?;
    cursor = skip_whitespace(script, next);
    cursor = consume_char(script, cursor, ')')?;
    cursor = skip_whitespace(script, cursor);
    cursor = consume_char(script, cursor, '.')?;

    if !script[cursor..].starts_with(TEXT_CONTENT) {
        return None;
    }
    cursor += TEXT_CONTENT.len();
    cursor = skip_whitespace(script, cursor);
    cursor = consume_char(script, cursor, '=')?;
    cursor = skip_whitespace(script, cursor);

    let (text, next) = parse_js_string_literal(script, cursor)?;
    cursor = skip_whitespace(script, next);
    if let Some(next) = consume_char(script, cursor, ';') {
        cursor = next;
    }

    Some((TextContentAssignment { element_id, text }, cursor))
}

fn parse_js_string_literal(source: &str, start: usize) -> Option<(String, usize)> {
    let quote = source[start..].chars().next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }

    let mut out = String::new();
    let mut escaped = false;
    let mut cursor = start + quote.len_utf8();

    while cursor < source.len() {
        let ch = source[cursor..].chars().next()?;
        cursor += ch.len_utf8();

        if escaped {
            escaped = false;
            match ch {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                '\\' => out.push('\\'),
                '\'' => out.push('\''),
                '"' => out.push('"'),
                'u' => {
                    let after = cursor;
                    let end = after.checked_add(4)?;
                    if end > source.len() {
                        return None;
                    }
                    let hex = &source[after..end];
                    let value = u32::from_str_radix(hex, 16).ok()?;
                    let chr = char::from_u32(value)?;
                    out.push(chr);
                    cursor = end;
                }
                _ => out.push(ch),
            }
            continue;
        }

        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == quote {
            return Some((out, cursor));
        }
        out.push(ch);
    }

    None
}

fn skip_whitespace(source: &str, start: usize) -> usize {
    let mut cursor = start;
    while cursor < source.len() {
        let Some(ch) = source[cursor..].chars().next() else {
            break;
        };
        if !ch.is_ascii_whitespace() {
            break;
        }
        cursor += ch.len_utf8();
    }
    cursor
}

fn consume_char(source: &str, start: usize, expected: char) -> Option<usize> {
    let ch = source[start..].chars().next()?;
    if ch != expected {
        return None;
    }
    Some(start + ch.len_utf8())
}

fn inject_vector_appearance_fallback(document: &mut Document) {
    ensure_vector_appearance_landmark_visible(&mut document.root);

    let Some(appearance) = document.find_first_element_by_id_mut("vector-appearance") else {
        return;
    };
    if contains_descendant_class(appearance, "oab-appearance-fallback") {
        return;
    }

    appearance.children.push(Node::Element(build_element(
        "div",
        &[("class", "vector-menu-content oab-appearance-fallback")],
        vec![
            Node::Element(build_element(
                "div",
                &[("style", "font-size:12px;font-weight:bold;margin-top:8px")],
                vec![Node::Text("Text".to_owned())],
            )),
            Node::Element(build_element(
                "div",
                &[("style", "font-size:12px")],
                vec![
                    Node::Element(build_radio_label(
                        "oab-appearance-text",
                        "small",
                        false,
                        "Small",
                    )),
                    Node::Text(" ".to_owned()),
                    Node::Element(build_radio_label(
                        "oab-appearance-text",
                        "standard",
                        true,
                        "Standard",
                    )),
                    Node::Text(" ".to_owned()),
                    Node::Element(build_radio_label(
                        "oab-appearance-text",
                        "large",
                        false,
                        "Large",
                    )),
                ],
            )),
            Node::Element(build_element(
                "div",
                &[("style", "font-size:12px;font-weight:bold;margin-top:8px")],
                vec![Node::Text("Width".to_owned())],
            )),
            Node::Element(build_element(
                "div",
                &[("style", "font-size:12px")],
                vec![
                    Node::Element(build_radio_label(
                        "oab-appearance-width",
                        "standard",
                        true,
                        "Standard",
                    )),
                    Node::Text(" ".to_owned()),
                    Node::Element(build_radio_label(
                        "oab-appearance-width",
                        "wide",
                        false,
                        "Wide",
                    )),
                ],
            )),
            Node::Element(build_element(
                "div",
                &[("style", "font-size:12px;font-weight:bold;margin-top:8px")],
                vec![Node::Text("Color (beta)".to_owned())],
            )),
            Node::Element(build_element(
                "div",
                &[("style", "font-size:12px;margin-bottom:8px")],
                vec![
                    Node::Element(build_radio_label(
                        "oab-appearance-color",
                        "auto",
                        true,
                        "Automatic",
                    )),
                    Node::Text(" ".to_owned()),
                    Node::Element(build_radio_label(
                        "oab-appearance-color",
                        "light",
                        false,
                        "Light",
                    )),
                    Node::Text(" ".to_owned()),
                    Node::Element(build_radio_label(
                        "oab-appearance-color",
                        "dark",
                        false,
                        "Dark",
                    )),
                ],
            )),
        ],
    )));
}

fn build_radio_label(_name: &str, _value: &str, checked: bool, text: &str) -> Element {
    let marker = if checked { "(o)" } else { "( )" };
    build_element(
        "label",
        &[("style", "white-space:nowrap")],
        vec![Node::Text(format!("{marker} {text}"))],
    )
}

fn build_element(name: &str, attrs: &[(&str, &str)], children: Vec<Node>) -> Element {
    let mut attributes = crate::dom::Attributes::default();
    for (key, value) in attrs {
        attributes.insert((*key).to_owned(), (*value).to_owned());
    }
    Element {
        name: name.to_owned(),
        attributes,
        children,
    }
}

fn contains_descendant_class(element: &Element, class_name: &str) -> bool {
    if element.attributes.has_class(class_name) {
        return true;
    }
    for child in &element.children {
        let Node::Element(el) = child else {
            continue;
        };
        if contains_descendant_class(el, class_name) {
            return true;
        }
    }
    false
}

fn ensure_vector_appearance_landmark_visible(root: &mut Element) {
    if root.name == "nav"
        && root.attributes.has_class("vector-appearance-landmark")
        && contains_descendant_id(root, "vector-appearance-pinned-container")
    {
        append_inline_style(root, "display:block");
    }

    for child in &mut root.children {
        let Node::Element(el) = child else {
            continue;
        };
        ensure_vector_appearance_landmark_visible(el);
    }
}

fn contains_descendant_id(element: &Element, id: &str) -> bool {
    if element.attributes.id.as_deref() == Some(id) {
        return true;
    }
    for child in &element.children {
        let Node::Element(el) = child else {
            continue;
        };
        if contains_descendant_id(el, id) {
            return true;
        }
    }
    false
}

fn append_inline_style(element: &mut Element, declaration: &str) {
    let mut style = element.attributes.style.take().unwrap_or_default();
    if !style.is_empty() && !style.trim_end().ends_with(';') {
        style.push(';');
    }
    style.push_str(declaration);
    element.attributes.style = Some(style);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_get_element_by_id_text_content_assignment() {
        let script = r#"document.getElementById("greeting").textContent = "Hello World!";"#;
        let assignments = parse_text_content_assignments(script);
        assert_eq!(
            assignments,
            vec![TextContentAssignment {
                element_id: "greeting".to_owned(),
                text: "Hello World!".to_owned(),
            }]
        );
    }

    #[test]
    fn executes_inline_script_assignment_against_dom() {
        let html = r#"
<!DOCTYPE html>
<html>
  <body>
    <h1 id="greeting">Welcome</h1>
    <script>
      document.getElementById("greeting").textContent = "Hello World!";
    </script>
  </body>
</html>
"#;
        let mut document = crate::html::parse_document(html);
        execute_inline_scripts(&mut document);
        let greeting = document
            .find_first_element_by_id("greeting")
            .expect("missing greeting");
        assert_eq!(
            greeting.children,
            vec![Node::Text("Hello World!".to_owned())]
        );
    }

    #[test]
    fn ignores_non_javascript_script_type() {
        let html = r#"
<html>
  <body>
    <h1 id="greeting">Welcome</h1>
    <script type="application/json">
      document.getElementById("greeting").textContent = "Hello World!";
    </script>
  </body>
</html>
"#;
        let mut document = crate::html::parse_document(html);
        execute_inline_scripts(&mut document);
        let greeting = document
            .find_first_element_by_id("greeting")
            .expect("missing greeting");
        assert_eq!(greeting.children, vec![Node::Text("Welcome".to_owned())]);
    }

    #[test]
    fn parses_document_element_class_assignment_via_variable() {
        let script = r#"
            var className = "client-js vector-feature-a-enabled";
            document.documentElement.className = className;
        "#;
        assert_eq!(
            parse_document_element_class_name_assignment(script),
            Some("client-js vector-feature-a-enabled".to_owned())
        );
    }

    #[test]
    fn keeps_nojs_root_classes_when_inline_script_switches_to_client_js() {
        let html = r#"
<html class="client-nojs">
  <body>
    <script>
      var className = "client-js skin-vector";
      document.documentElement.className = className;
    </script>
  </body>
</html>
"#;
        let mut document = crate::html::parse_document(html);
        execute_inline_scripts(&mut document);
        let html = document
            .find_first_element_by_name("html")
            .expect("missing html element");
        assert!(html.attributes.has_class("client-nojs"));
        assert!(!html.attributes.has_class("client-js"));
    }

    #[test]
    fn executes_document_element_class_assignment_without_nojs_guard() {
        let html = r#"
<html class="initial">
  <body>
    <script>
      var className = "foo bar";
      document.documentElement.className = className;
    </script>
  </body>
</html>
"#;
        let mut document = crate::html::parse_document(html);
        execute_inline_scripts(&mut document);
        let html = document
            .find_first_element_by_name("html")
            .expect("missing html element");
        assert!(html.attributes.has_class("foo"));
        assert!(html.attributes.has_class("bar"));
        assert!(!html.attributes.has_class("initial"));
    }

    #[test]
    fn injects_vector_appearance_fallback_when_panel_is_empty() {
        let html = r#"
<html class="skin-vector">
  <body>
    <div id="vector-appearance"><div class="vector-pinnable-header">Appearance</div></div>
  </body>
</html>
"#;
        let mut document = crate::html::parse_document(html);
        execute_inline_scripts(&mut document);
        let panel = document
            .find_first_element_by_id("vector-appearance")
            .expect("missing vector appearance panel");
        assert!(contains_descendant_text(panel, "(o) Standard"));
    }

    #[test]
    fn forces_pinned_appearance_landmark_visible() {
        let html = r#"
<html>
  <body>
    <nav class="vector-appearance-landmark">
      <div id="vector-appearance-pinned-container">
        <div id="vector-appearance"></div>
      </div>
    </nav>
  </body>
</html>
"#;
        let mut document = crate::html::parse_document(html);
        execute_inline_scripts(&mut document);
        let nav = document
            .find_first_element_by_name("nav")
            .expect("missing appearance nav");
        let style = nav.attributes.style.as_deref().unwrap_or("");
        assert!(
            style.contains("display:block"),
            "expected inline display:block override, got: {style}"
        );
    }

    fn contains_descendant_text(element: &Element, needle: &str) -> bool {
        for child in &element.children {
            match child {
                Node::Text(text) => {
                    if text.contains(needle) {
                        return true;
                    }
                }
                Node::Element(el) => {
                    if contains_descendant_text(el, needle) {
                        return true;
                    }
                }
            }
        }
        false
    }
}
