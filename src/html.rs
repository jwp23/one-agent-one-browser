use crate::dom::{Document, Element, Node};

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
            children: Vec::new(),
        }];

        while let Some(fragment) = self.next_fragment() {
            match fragment {
                Fragment::Text(text) => {
                    if !text.is_empty() {
                        stack
                            .last_mut()
                            .expect("stack never empty")
                            .children
                            .push(Node::Text(text));
                    }
                }
                Fragment::StartTag { name, self_closing } => {
                    if self_closing || is_void_element(&name) {
                        stack
                            .last_mut()
                            .expect("stack never empty")
                            .children
                            .push(Node::Element(Element {
                                name,
                                children: Vec::new(),
                            }));
                    } else {
                        stack.push(Element {
                            name,
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

        let (name, self_closing) = parse_tag_name(raw);
        if name.is_empty() {
            return self.next_fragment();
        }

        let name = normalize_tag_name(name);

        if is_end {
            Some(Fragment::EndTag { name })
        } else {
            Some(Fragment::StartTag { name, self_closing })
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
    StartTag { name: String, self_closing: bool },
    EndTag { name: String },
    Text(String),
}

fn parse_tag_name(tag_contents: &str) -> (&str, bool) {
    let self_closing = tag_contents.trim_end().ends_with('/');
    let name_end = tag_contents
        .find(|ch: char| ch.is_whitespace() || ch == '/' || ch == '>')
        .unwrap_or(tag_contents.len());
    let name = &tag_contents[..name_end];
    (name, self_closing)
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
}

