use crate::dom::{Document, Element, Node};

pub fn execute_inline_scripts(document: &mut Document) {
    let mut scripts = Vec::new();
    collect_inline_classic_scripts(&document.root, &mut scripts);

    for source in scripts {
        for assignment in parse_text_content_assignments(&source) {
            if let Some(element) = document.find_first_element_by_id_mut(&assignment.element_id) {
                element.set_text_content(assignment.text);
            }
        }
    }
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
}
