mod engine;

use crate::dom::{Document, Element, Node};

pub fn execute_inline_scripts(document: &mut Document) {
    let mut scripts = Vec::new();
    collect_inline_classic_scripts(&document.root, &mut scripts);

    execute_script_sources(document, &scripts);
}

pub fn execute_script_sources(document: &mut Document, sources: &[String]) {
    let trace_errors = trace_js_errors_enabled();
    let mut runtime = engine::Runtime::new(document);

    for (index, source) in sources.iter().enumerate() {
        if let Err(err) = runtime.execute(document, source)
            && trace_errors
        {
            eprintln!(
                "js[{index}] bytes={} err={} src={}",
                source.len(),
                sanitize_diagnostic(&err),
                script_preview(source)
            );
        }
    }

    inject_vector_appearance_fallback(document);
}

fn trace_js_errors_enabled() -> bool {
    std::env::var_os("OAB_TRACE_JS_ERRORS").is_some()
}

fn sanitize_diagnostic(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '\n' | '\r' | '\t' => ' ',
            _ => ch,
        })
        .collect()
}

fn script_preview(source: &str) -> String {
    let compact = sanitize_diagnostic(source);
    let compact = compact.trim();
    let mut preview = String::new();
    for ch in compact.chars().take(96) {
        preview.push(ch);
    }
    if compact.chars().count() > 96 {
        preview.push('…');
    }
    preview
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
    Element::new(name, attributes, children)
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
    fn executes_document_element_class_assignment_that_switches_to_client_js() {
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
        assert!(html.attributes.has_class("client-js"));
        assert!(html.attributes.has_class("skin-vector"));
        assert!(!html.attributes.has_class("client-nojs"));
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
    fn executes_document_element_class_list_mutations() {
        let html = r#"
<html class="client-nojs">
  <body>
    <script>
      document.documentElement.classList.add("client-js", "vector-animations-ready");
      document.documentElement.classList.remove("client-nojs");
    </script>
  </body>
</html>
"#;
        let mut document = crate::html::parse_document(html);
        execute_inline_scripts(&mut document);
        let html = document
            .find_first_element_by_name("html")
            .expect("missing html element");
        assert!(html.attributes.has_class("client-js"));
        assert!(html.attributes.has_class("vector-animations-ready"));
        assert!(!html.attributes.has_class("client-nojs"));
    }

    #[test]
    fn executes_get_element_by_id_class_list_mutations() {
        let html = r#"
<html>
  <body>
    <div id="target" class="collapsed"></div>
    <script>
      document.getElementById("target").classList.add("expanded");
      document.getElementById("target").classList.remove("collapsed");
    </script>
  </body>
</html>
"#;
        let mut document = crate::html::parse_document(html);
        execute_inline_scripts(&mut document);
        let target = document
            .find_first_element_by_id("target")
            .expect("missing target element");
        assert!(target.attributes.has_class("expanded"));
        assert!(!target.attributes.has_class("collapsed"));
    }

    #[test]
    fn executes_jquery_add_class_mutation() {
        let html = r#"
<html class="client-nojs">
  <body>
    <script>
      $('html').addClass('ve-available vector-animations-ready');
      $('html').removeClass('client-nojs');
    </script>
  </body>
</html>
"#;
        let mut document = crate::html::parse_document(html);
        execute_inline_scripts(&mut document);
        let html = document
            .find_first_element_by_name("html")
            .expect("missing html element");
        assert!(html.attributes.has_class("ve-available"));
        assert!(html.attributes.has_class("vector-animations-ready"));
        assert!(!html.attributes.has_class("client-nojs"));
    }

    #[test]
    fn executes_wikipedia_client_bootstrap_script() {
        let html = r#"
<html class="client-nojs">
  <body>
    <script>
      (function(){
        var className="client-js skin-vector";
        var cookie=document.cookie.match(/(?:^|; )enwikimwclientpreferences=([^;]+)/);
        if(cookie){
          cookie[1].split('%2C').forEach(function(pref){
            className=className.replace(new RegExp('(^| )'+pref.replace(/-clientpref-\w+$|[^\w-]+/g,'')+'-clientpref-\\w+( |$)'),'$1'+pref+'$2');
          });
        }
        document.documentElement.className=className;
      }());
    </script>
  </body>
</html>
"#;
        let mut document = crate::html::parse_document(html);
        execute_inline_scripts(&mut document);
        let html = document
            .find_first_element_by_name("html")
            .expect("missing html element");
        assert!(html.attributes.has_class("client-js"));
        assert!(html.attributes.has_class("skin-vector"));
        assert!(!html.attributes.has_class("client-nojs"));
    }

    #[test]
    fn shares_globals_across_script_sources() {
        let mut document = crate::html::parse_document(r#"<html><body></body></html>"#);

        execute_script_sources(
            &mut document,
            &[
                "window.sharedState = { ready: true };".to_owned(),
                "document.body.textContent = sharedState.ready;".to_owned(),
            ],
        );

        let body = document
            .find_first_element_by_name("body")
            .expect("missing body element");
        assert_eq!(contains_descendant_text(body, "true"), true);
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
