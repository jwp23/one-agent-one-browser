use crate::dom::{Attributes, Document, Element, Node};

pub fn parse_document(source: &str) -> Document {
    let mut parser = Parser::new(source);
    parser.parse_document()
}

struct Parser<'a> {
    input: &'a str,
    cursor: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, cursor: 0 }
    }

    fn parse_document(&mut self) -> Document {
        let mut stack: Vec<Element> = vec![Element {
            name: "#document".to_owned(),
            attributes: Attributes::default(),
            children: Vec::new(),
        }];

        while let Some(fragment) = self.next_fragment() {
            match fragment {
                Fragment::Text(text) => {
                    let text = decode_html_entities(&text);
                    if !text.is_empty() {
                        stack
                            .last_mut()
                            .expect("stack never empty")
                            .children
                            .push(Node::Text(text));
                    }
                }
                Fragment::StartTag {
                    name,
                    attributes,
                    self_closing,
                } => {
                    if self_closing || is_void_element(&name) {
                        stack
                            .last_mut()
                            .expect("stack never empty")
                            .children
                            .push(Node::Element(Element {
                                name,
                                attributes,
                                children: Vec::new(),
                            }));
                    } else {
                        stack.push(Element {
                            name,
                            attributes,
                            children: Vec::new(),
                        });
                    }
                }
                Fragment::EndTag { name } => {
                    self.close_element(&mut stack, &name);
                }
            }
        }

        while stack.len() > 1 {
            self.close_top(&mut stack);
        }

        let root = stack
            .pop()
            .expect("stack had root")
            .children
            .into_iter()
            .find_map(|node| match node {
                Node::Element(el) => Some(el),
                Node::Text(_) => None,
            })
            .unwrap_or(Element {
                name: "html".to_owned(),
                attributes: Attributes::default(),
                children: Vec::new(),
            });

        Document { root }
    }

    fn close_element(&self, stack: &mut Vec<Element>, name: &str) {
        if stack.len() <= 1 {
            return;
        }

        if let Some(index) = stack.iter().rposition(|el| el.name == name) {
            while stack.len() - 1 >= index {
                self.close_top(stack);
            }
        }
    }

    fn close_top(&self, stack: &mut Vec<Element>) {
        if stack.len() <= 1 {
            return;
        }

        let element = stack.pop().expect("len > 1 implies pop ok");
        stack
            .last_mut()
            .expect("stack never empty")
            .children
            .push(Node::Element(element));
    }

    fn next_fragment(&mut self) -> Option<Fragment> {
        if self.cursor >= self.input.len() {
            return None;
        }

        let bytes = self.input.as_bytes();
        if bytes[self.cursor] != b'<' {
            return Some(Fragment::Text(self.consume_text_until('<')));
        }

        if self.starts_with("<!--") {
            self.consume_comment();
            return self.next_fragment();
        }

        if self.starts_with("<!") || self.starts_with("<?") {
            self.consume_until('>');
            return self.next_fragment();
        }

        self.cursor += 1;
        let raw = self.consume_until('>');
        let raw = raw.trim();
        if raw.is_empty() {
            return self.next_fragment();
        }

        let is_end = raw.starts_with('/');
        let raw = raw.strip_prefix('/').unwrap_or(raw).trim_start();

        let (name, rest, self_closing) = parse_tag_name(raw);
        if name.is_empty() {
            return self.next_fragment();
        }

        let name = normalize_tag_name(name);

        if is_end {
            Some(Fragment::EndTag { name })
        } else {
            let attributes = parse_attributes(rest);
            Some(Fragment::StartTag {
                name,
                attributes,
                self_closing,
            })
        }
    }

    fn consume_text_until(&mut self, delimiter: char) -> String {
        let start = self.cursor;
        let until = self
            .input
            .get(self.cursor..)
            .and_then(|rest| rest.find(delimiter))
            .map(|offset| self.cursor + offset)
            .unwrap_or(self.input.len());
        self.cursor = until;
        self.input[start..until].to_owned()
    }

    fn consume_comment(&mut self) {
        if !self.starts_with("<!--") {
            return;
        }

        self.cursor += "<!--".len();
        if let Some(end_offset) = self.input.get(self.cursor..).and_then(|rest| rest.find("-->"))
        {
            self.cursor += end_offset + "-->".len();
        } else {
            self.cursor = self.input.len();
        }
    }

    fn consume_until(&mut self, delimiter: char) -> &str {
        let start = self.cursor;
        while self.cursor < self.input.len() {
            if self.input.as_bytes()[self.cursor] == delimiter as u8 {
                let end = self.cursor;
                self.cursor += 1;
                return &self.input[start..end];
            }
            self.cursor += 1;
        }
        &self.input[start..]
    }

    fn starts_with(&self, s: &str) -> bool {
        self.input[self.cursor..].starts_with(s)
    }
}

enum Fragment {
    StartTag {
        name: String,
        attributes: Attributes,
        self_closing: bool,
    },
    EndTag { name: String },
    Text(String),
}

fn parse_tag_name(tag_contents: &str) -> (&str, &str, bool) {
    let trimmed_end = tag_contents.trim_end();
    let self_closing = trimmed_end.ends_with('/');
    let tag_contents = trimmed_end.strip_suffix('/').unwrap_or(trimmed_end);

    let name_end = tag_contents
        .find(|ch: char| ch.is_whitespace() || ch == '/' || ch == '>')
        .unwrap_or(tag_contents.len());
    let name = &tag_contents[..name_end];
    let rest = &tag_contents[name_end..];
    (name, rest, self_closing)
}

fn normalize_tag_name(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

fn is_void_element(name: &str) -> bool {
    matches!(
        name,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

fn parse_attributes(mut input: &str) -> Attributes {
    let mut attrs = Attributes::default();

    loop {
        input = input.trim_start();
        if input.is_empty() {
            break;
        }

        let mut name_end = 0usize;
        for (idx, ch) in input.char_indices() {
            if ch.is_whitespace() || ch == '=' {
                break;
            }
            name_end = idx + ch.len_utf8();
        }
        if name_end == 0 {
            break;
        }

        let raw_name = &input[..name_end];
        let name = raw_name.trim().to_ascii_lowercase();
        input = &input[name_end..];
        input = input.trim_start();

        let mut value = String::new();
        if let Some(rest) = input.strip_prefix('=') {
            input = rest.trim_start();
            if let Some(quoted) = input.strip_prefix('"') {
                if let Some(end) = quoted.find('"') {
                    value = quoted[..end].to_owned();
                    input = &quoted[end + 1..];
                } else {
                    value = quoted.to_owned();
                    input = "";
                }
            } else if let Some(quoted) = input.strip_prefix('\'') {
                if let Some(end) = quoted.find('\'') {
                    value = quoted[..end].to_owned();
                    input = &quoted[end + 1..];
                } else {
                    value = quoted.to_owned();
                    input = "";
                }
            } else {
                let end = input
                    .find(|ch: char| ch.is_whitespace())
                    .unwrap_or(input.len());
                value = input[..end].to_owned();
                input = &input[end..];
            }
        }

        let value = decode_html_entities(&value);
        attrs.insert(name, value);
    }

    attrs
}

fn decode_html_entities(input: &str) -> String {
    if !input.contains('&') {
        return input.to_owned();
    }

    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp + 1..];

        let Some(semi) = rest.find(';') else {
            out.push('&');
            out.push_str(rest);
            return out;
        };

        let entity = &rest[..semi];
        rest = &rest[semi + 1..];

        match entity {
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "amp" => out.push('&'),
            "quot" => out.push('"'),
            "apos" => out.push('\''),
            "nbsp" => out.push('\u{00A0}'),
            _ if entity.starts_with("#x") || entity.starts_with("#X") => {
                if let Ok(value) = u32::from_str_radix(&entity[2..], 16) {
                    if let Some(ch) = char::from_u32(value) {
                        out.push(ch);
                    }
                }
            }
            _ if entity.starts_with('#') => {
                if let Ok(value) = entity[1..].parse::<u32>() {
                    if let Some(ch) = char::from_u32(value) {
                        out.push(ch);
                    }
                }
            }
            _ => {
                out.push('&');
                out.push_str(entity);
                out.push(';');
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_inline_markup() {
        let doc = parse_document("<p>Hello <strong>World</strong></p>");
        let root = doc.render_root();
        assert_eq!(root.name, "p");
        assert_eq!(
            root.children,
            vec![
                Node::Text("Hello ".to_owned()),
                Node::Element(Element {
                    name: "strong".to_owned(),
                    attributes: Attributes::default(),
                    children: vec![Node::Text("World".to_owned())],
                }),
            ]
        );
    }

    #[test]
    fn treats_void_elements_as_self_closing() {
        let doc = parse_document("<p>hi<br>there</p>");
        let root = doc.render_root();
        assert_eq!(root.name, "p");
        assert_eq!(root.children.len(), 3);
    }

    #[test]
    fn parses_attributes() {
        let doc = parse_document("<p id=\"a\" class='b c' data-x=1>Hello</p>");
        let root = doc.render_root();
        assert_eq!(root.name, "p");
        assert_eq!(root.attributes.id.as_deref(), Some("a"));
        assert_eq!(root.attributes.classes, vec!["b".to_owned(), "c".to_owned()]);
        assert_eq!(root.attributes.get("data-x"), Some("1"));
    }

    #[test]
    fn decodes_entities_in_text() {
        let doc = parse_document("<p>&lt; &amp; &gt; &#x27; &#39;</p>");
        let root = doc.render_root();
        assert_eq!(
            root.children,
            vec![Node::Text("< & > ' '".to_owned())]
        );
    }
}
