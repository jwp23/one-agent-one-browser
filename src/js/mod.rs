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

    let state = vector_appearance_state(document);
    let Some(appearance) = document.find_first_element_by_id_mut("vector-appearance") else {
        return;
    };
    if contains_descendant_class(appearance, "vector-menu")
        || contains_descendant_class(appearance, "oab-appearance-fallback")
    {
        return;
    }

    appearance
        .children
        .extend(build_vector_appearance_fallback(&state));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VectorAppearanceTextSize {
    Small,
    Standard,
    Large,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VectorAppearanceTheme {
    Automatic,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct VectorAppearanceState {
    birthday_mode_enabled: bool,
    text_size: VectorAppearanceTextSize,
    standard_width: bool,
    theme: VectorAppearanceTheme,
}

fn vector_appearance_state(document: &Document) -> VectorAppearanceState {
    let birthday_mode_enabled = html_has_class(document, "wp25eastereggs-enable-clientpref-1");

    let text_size = if html_has_class(document, "vector-feature-custom-font-size-clientpref-0") {
        VectorAppearanceTextSize::Small
    } else if html_has_class(document, "vector-feature-custom-font-size-clientpref-2") {
        VectorAppearanceTextSize::Large
    } else {
        VectorAppearanceTextSize::Standard
    };

    let standard_width = !html_has_class(document, "vector-feature-limited-width-clientpref-0");

    let theme = if html_has_class(document, "skin-theme-clientpref-night") {
        VectorAppearanceTheme::Dark
    } else if html_has_class(document, "skin-theme-clientpref-os") {
        VectorAppearanceTheme::Automatic
    } else {
        VectorAppearanceTheme::Light
    };

    VectorAppearanceState {
        birthday_mode_enabled,
        text_size,
        standard_width,
        theme,
    }
}

fn html_has_class(document: &Document, class_name: &str) -> bool {
    document
        .find_first_element_by_name("html")
        .is_some_and(|html| html.attributes.has_class(class_name))
}

fn build_vector_appearance_fallback(state: &VectorAppearanceState) -> Vec<Node> {
    vec![
        Node::Element(build_vector_appearance_portlet(
            "skin-client-prefs-wp25eastereggs-enable",
            &[
                "oab-appearance-fallback",
                "mw-portlet-skin-client-prefs-wp25eastereggs-enable",
            ],
            vec![Node::Text("Birthday mode (Baby Globe)".to_owned())],
            vec![
                build_radio_control(
                    "skin-client-pref-wp25eastereggs-enable-group",
                    "skin-client-pref-wp25eastereggs-enable-value-0",
                    "0",
                    "Disabled",
                    !state.birthday_mode_enabled,
                ),
                build_radio_control(
                    "skin-client-pref-wp25eastereggs-enable-group",
                    "skin-client-pref-wp25eastereggs-enable-value-1",
                    "1",
                    "Enabled",
                    state.birthday_mode_enabled,
                ),
            ],
            vec![Node::Element(build_element(
                "span",
                &[("id", "wp25eastereggs-enable-beta-notice")],
                vec![Node::Element(build_element(
                    "a",
                    &[
                        (
                            "href",
                            "https://wikimediafoundation.org/wikipedia25/wikipedia-mascot/?utm_campaign=wpam&utm_source=wpam&utm_medium=wpamen",
                        ),
                        ("target", "_blank"),
                    ],
                    vec![Node::Text("Learn more about Birthday mode".to_owned())],
                ))],
            ))],
            Vec::new(),
        )),
        Node::Element(build_vector_appearance_portlet(
            "skin-client-prefs-vector-feature-custom-font-size",
            &["mw-portlet-skin-client-prefs-vector-feature-custom-font-size"],
            vec![Node::Text("Text".to_owned())],
            vec![
                build_radio_control(
                    "skin-client-pref-vector-feature-custom-font-size-group",
                    "skin-client-pref-vector-feature-custom-font-size-value-0",
                    "0",
                    "Small",
                    state.text_size == VectorAppearanceTextSize::Small,
                ),
                build_radio_control(
                    "skin-client-pref-vector-feature-custom-font-size-group",
                    "skin-client-pref-vector-feature-custom-font-size-value-1",
                    "1",
                    "Standard",
                    state.text_size == VectorAppearanceTextSize::Standard,
                ),
                build_radio_control(
                    "skin-client-pref-vector-feature-custom-font-size-group",
                    "skin-client-pref-vector-feature-custom-font-size-value-2",
                    "2",
                    "Large",
                    state.text_size == VectorAppearanceTextSize::Large,
                ),
            ],
            Vec::new(),
            vec![Node::Element(build_element(
                "span",
                &[("class", "skin-client-pref-exclusion-notice")],
                vec![Node::Text(
                    "This page always uses small font size".to_owned(),
                )],
            ))],
        )),
        Node::Element(build_vector_appearance_portlet(
            "skin-client-prefs-vector-feature-limited-width",
            &["mw-portlet-skin-client-prefs-vector-feature-limited-width"],
            vec![Node::Text("Width".to_owned())],
            vec![
                build_radio_control(
                    "skin-client-pref-vector-feature-limited-width-group",
                    "skin-client-pref-vector-feature-limited-width-value-1",
                    "1",
                    "Standard",
                    state.standard_width,
                ),
                build_radio_control(
                    "skin-client-pref-vector-feature-limited-width-group",
                    "skin-client-pref-vector-feature-limited-width-value-0",
                    "0",
                    "Wide",
                    !state.standard_width,
                ),
            ],
            Vec::new(),
            vec![Node::Element(build_element(
                "span",
                &[("class", "skin-client-pref-exclusion-notice")],
                vec![Node::Text(
                    "The content is as wide as possible for your browser window.".to_owned(),
                )],
            ))],
        )),
        Node::Element(build_vector_appearance_portlet(
            "skin-client-prefs-skin-theme",
            &["mw-portlet-skin-client-prefs-skin-theme"],
            vec![
                Node::Text("Color ".to_owned()),
                Node::Element(build_element(
                    "span",
                    &[],
                    vec![Node::Element(build_element(
                        "span",
                        &[],
                        vec![Node::Text("(beta)".to_owned())],
                    ))],
                )),
            ],
            vec![
                build_radio_control(
                    "skin-client-pref-skin-theme-group",
                    "skin-client-pref-skin-theme-value-os",
                    "os",
                    "Automatic",
                    state.theme == VectorAppearanceTheme::Automatic,
                ),
                build_radio_control(
                    "skin-client-pref-skin-theme-group",
                    "skin-client-pref-skin-theme-value-day",
                    "day",
                    "Light",
                    state.theme == VectorAppearanceTheme::Light,
                ),
                build_radio_control(
                    "skin-client-pref-skin-theme-group",
                    "skin-client-pref-skin-theme-value-night",
                    "night",
                    "Dark",
                    state.theme == VectorAppearanceTheme::Dark,
                ),
            ],
            vec![Node::Element(build_element(
                "span",
                &[("id", "skin-theme-beta-notice")],
                Vec::new(),
            ))],
            vec![Node::Element(build_element(
                "span",
                &[("class", "skin-client-pref-exclusion-notice")],
                vec![Node::Text("This page is always in light mode.".to_owned())],
            ))],
        )),
    ]
}

fn build_vector_appearance_portlet(
    id: &str,
    extra_classes: &[&str],
    heading_children: Vec<Node>,
    radios: Vec<Element>,
    item_trailing_children: Vec<Node>,
    content_trailing_children: Vec<Node>,
) -> Element {
    let mut classes = vec!["mw-portlet", "vector-menu"];
    classes.extend_from_slice(extra_classes);

    let mut item_children = vec![Node::Element(build_element(
        "form",
        &[],
        radios.into_iter().map(Node::Element).collect(),
    ))];
    item_children.extend(item_trailing_children);

    let mut content_children = vec![Node::Element(build_element(
        "ul",
        &[("class", "vector-menu-content-list")],
        vec![Node::Element(build_element(
            "li",
            &[("class", "mw-list-item mw-list-item-js")],
            vec![Node::Element(build_element("div", &[], item_children))],
        ))],
    ))];
    content_children.extend(content_trailing_children);

    build_element_owned(
        "div",
        vec![
            ("id".to_owned(), id.to_owned()),
            ("class".to_owned(), classes.join(" ")),
        ],
        vec![
            Node::Element(build_element(
                "div",
                &[("class", "vector-menu-heading")],
                heading_children,
            )),
            Node::Element(build_element(
                "div",
                &[("class", "vector-menu-content")],
                content_children,
            )),
        ],
    )
}

fn build_radio_control(name: &str, id: &str, value: &str, text: &str, checked: bool) -> Element {
    let mut input_attrs = vec![
        ("name".to_owned(), name.to_owned()),
        ("id".to_owned(), id.to_owned()),
        ("type".to_owned(), "radio".to_owned()),
        ("value".to_owned(), value.to_owned()),
        ("data-event-name".to_owned(), id.to_owned()),
        ("class".to_owned(), "cdx-radio__input".to_owned()),
    ];
    if checked {
        input_attrs.push(("checked".to_owned(), "checked".to_owned()));
    }

    build_element(
        "div",
        &[("class", "cdx-radio")],
        vec![
            Node::Element(build_element_owned("input", input_attrs, Vec::new())),
            Node::Element(build_element(
                "span",
                &[("class", "cdx-radio__icon")],
                Vec::new(),
            )),
            Node::Element(build_element(
                "label",
                &[("class", "cdx-label cdx-radio__label"), ("for", id)],
                vec![Node::Element(build_element(
                    "span",
                    &[("class", "cdx-label__label__text")],
                    vec![Node::Text(text.to_owned())],
                ))],
            )),
        ],
    )
}

fn build_element(name: &str, attrs: &[(&str, &str)], children: Vec<Node>) -> Element {
    build_element_owned(
        name,
        attrs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect(),
        children,
    )
}

fn build_element_owned(name: &str, attrs: Vec<(String, String)>, children: Vec<Node>) -> Element {
    let mut attributes = crate::dom::Attributes::default();
    for (key, value) in attrs {
        attributes.insert(key, value);
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
<html class="skin-vector wp25eastereggs-enable-clientpref-1 vector-feature-custom-font-size-clientpref-1 vector-feature-limited-width-clientpref-1 skin-theme-clientpref-day">
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
        assert!(contains_descendant_class(panel, "oab-appearance-fallback"));
        assert!(contains_descendant_class(panel, "cdx-radio"));
        assert!(
            panel
                .find_first_element_by_id("skin-client-pref-wp25eastereggs-enable-value-1")
                .is_some_and(|input| input.attributes.get("checked").is_some())
        );
        assert!(
            panel
                .find_first_element_by_id(
                    "skin-client-pref-vector-feature-custom-font-size-value-1"
                )
                .is_some_and(|input| input.attributes.get("checked").is_some())
        );
        assert!(
            panel
                .find_first_element_by_id("skin-client-pref-vector-feature-limited-width-value-1")
                .is_some_and(|input| input.attributes.get("checked").is_some())
        );
        assert!(
            panel
                .find_first_element_by_id("skin-client-pref-skin-theme-value-day")
                .is_some_and(|input| input.attributes.get("checked").is_some())
        );
        assert!(!contains_descendant_text(panel, "(o) Standard"));
    }

    #[test]
    fn skips_vector_appearance_fallback_when_panel_already_has_content() {
        let html = r#"
<html>
  <body>
    <div id="vector-appearance">
      <div class="vector-pinnable-header">Appearance</div>
      <div class="vector-menu">Existing content</div>
    </div>
  </body>
</html>
"#;
        let mut document = crate::html::parse_document(html);
        execute_inline_scripts(&mut document);
        let panel = document
            .find_first_element_by_id("vector-appearance")
            .expect("missing vector appearance panel");
        assert!(!contains_descendant_class(panel, "oab-appearance-fallback"));
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
