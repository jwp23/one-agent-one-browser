#[derive(Clone, Debug, Default)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

impl Stylesheet {
    pub fn parse(source: &str) -> Stylesheet {
        Parser::new(source).parse_stylesheet()
    }
}

#[derive(Clone, Debug)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
    pub order: u32,
}

#[derive(Clone, Debug)]
pub struct Declaration {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug)]
pub struct Selector {
    pub parts: Vec<CompoundSelector>,
}

impl Selector {
    pub fn specificity(&self) -> Specificity {
        self.parts
            .iter()
            .fold(Specificity::default(), |acc, part| acc.add(part.specificity()))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Specificity {
    pub ids: u16,
    pub classes: u16,
    pub tags: u16,
}

impl Specificity {
    pub fn add(self, other: Specificity) -> Specificity {
        Specificity {
            ids: self.ids.saturating_add(other.ids),
            classes: self.classes.saturating_add(other.classes),
            tags: self.tags.saturating_add(other.tags),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CompoundSelector {
    pub tag: Option<String>,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub attributes: Vec<AttributeSelector>,
    pub pseudo_classes: Vec<PseudoClass>,
}

impl CompoundSelector {
    pub fn specificity(&self) -> Specificity {
        let mut specificity = Specificity::default();
        if self.id.is_some() {
            specificity.ids = 1;
        }
        specificity.classes = self
            .classes
            .len()
            .saturating_add(self.attributes.len())
            .saturating_add(self.pseudo_classes.len())
            .min(u16::MAX as usize) as u16;
        if self.tag.is_some() {
            specificity.tags = 1;
        }
        specificity
    }
}

#[derive(Clone, Debug)]
pub struct AttributeSelector {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PseudoClass {
    Link,
    Visited,
    Hover,
}

pub fn parse_inline_declarations(source: &str) -> Vec<Declaration> {
    parse_declarations(source)
}

fn parse_declarations(source: &str) -> Vec<Declaration> {
    let mut parser = DeclarationParser::new(source);
    parser.parse_all()
}

struct Parser<'a> {
    input: &'a str,
    cursor: usize,
    order: u32,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Parser<'a> {
        Parser {
            input,
            cursor: 0,
            order: 0,
        }
    }

    fn parse_stylesheet(mut self) -> Stylesheet {
        let mut rules = Vec::new();

        while self.skip_ws_and_comments() {
            if self.peek_char() == Some('@') {
                self.skip_at_rule();
                continue;
            }

            let Some(selectors_text) = self.consume_until('{') else {
                break;
            };
            let selectors = parse_selector_group(selectors_text);

            if self.peek_char() != Some('{') {
                break;
            }
            self.cursor += 1;

            let block = self.consume_block_contents();
            let declarations = parse_declarations(block);

            if !selectors.is_empty() && !declarations.is_empty() {
                rules.push(Rule {
                    selectors,
                    declarations,
                    order: self.order,
                });
                self.order = self.order.saturating_add(1);
            }
        }

        Stylesheet { rules }
    }

    fn skip_ws_and_comments(&mut self) -> bool {
        let mut progressed = false;
        loop {
            let before = self.cursor;
            self.skip_whitespace();
            self.skip_comment();
            if self.cursor == before {
                break;
            }
            progressed = true;
        }
        self.cursor < self.input.len() || progressed
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek_char() {
            if !ch.is_whitespace() {
                break;
            }
            self.cursor += ch.len_utf8();
        }
    }

    fn skip_comment(&mut self) {
        if !self.input[self.cursor..].starts_with("/*") {
            return;
        }
        self.cursor += 2;
        if let Some(end) = self.input[self.cursor..].find("*/") {
            self.cursor += end + 2;
        } else {
            self.cursor = self.input.len();
        }
    }

    fn skip_at_rule(&mut self) {
        if self.peek_char() != Some('@') {
            return;
        }

        while let Some(ch) = self.peek_char() {
            self.cursor += ch.len_utf8();
            if ch == ';' {
                return;
            }
            if ch == '{' {
                self.skip_balanced_block();
                return;
            }
        }
    }

    fn skip_balanced_block(&mut self) {
        let mut depth = 1usize;
        while let Some(ch) = self.peek_char() {
            self.cursor += ch.len_utf8();
            match ch {
                '{' => depth = depth.saturating_add(1),
                '}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return;
                    }
                }
                '"' | '\'' => {
                    self.skip_quoted_string(ch);
                }
                '/' => {
                    self.cursor = self.cursor.saturating_sub(1);
                    self.skip_comment();
                }
                _ => {}
            }
        }
    }

    fn skip_quoted_string(&mut self, quote: char) {
        while let Some(ch) = self.peek_char() {
            self.cursor += ch.len_utf8();
            if ch == '\\' {
                if let Some(next) = self.peek_char() {
                    self.cursor += next.len_utf8();
                }
                continue;
            }
            if ch == quote {
                return;
            }
        }
    }

    fn consume_until(&mut self, delimiter: char) -> Option<&'a str> {
        let start = self.cursor;
        while let Some(ch) = self.peek_char() {
            if ch == delimiter {
                return Some(self.input[start..self.cursor].trim());
            }
            if ch == '/' && self.input[self.cursor..].starts_with("/*") {
                self.skip_comment();
                continue;
            }
            self.cursor += ch.len_utf8();
        }
        None
    }

    fn consume_block_contents(&mut self) -> &'a str {
        let start = self.cursor;
        let mut depth = 1usize;
        while let Some(ch) = self.peek_char() {
            if ch == '/' && self.input[self.cursor..].starts_with("/*") {
                self.skip_comment();
                continue;
            }

            self.cursor += ch.len_utf8();
            match ch {
                '{' => depth = depth.saturating_add(1),
                '}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        let end = self.cursor - 1;
                        return self.input[start..end].trim();
                    }
                }
                '"' | '\'' => self.skip_quoted_string(ch),
                _ => {}
            }
        }

        self.input[start..].trim()
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.cursor..].chars().next()
    }
}

fn parse_selector_group(input: &str) -> Vec<Selector> {
    input
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(parse_selector)
        .collect()
}

fn parse_selector(selector: &str) -> Selector {
    let parts = selector
        .split_whitespace()
        .map(str::trim)
        .filter(|p| !p.is_empty() && *p != ">")
        .map(parse_compound_selector)
        .collect();
    Selector { parts }
}

fn parse_compound_selector(mut input: &str) -> CompoundSelector {
    let mut selector = CompoundSelector::default();
    input = input.trim();

    let tag_end = input
        .find(|ch: char| matches!(ch, '.' | '#' | ':' | '['))
        .unwrap_or(input.len());
    let tag = input[..tag_end].trim();
    if !tag.is_empty() && tag != "*" {
        selector.tag = Some(tag.to_ascii_lowercase());
    }
    input = &input[tag_end..];

    while !input.is_empty() {
        let mut chars = input.chars();
        let Some(prefix) = chars.next() else { break };
        match prefix {
            '.' => {
                let (name, rest) = split_simple_name(chars.as_str());
                if !name.is_empty() {
                    selector.classes.push(name.to_owned());
                }
                input = rest;
            }
            '#' => {
                let (name, rest) = split_simple_name(chars.as_str());
                if !name.is_empty() {
                    selector.id = Some(name.to_owned());
                }
                input = rest;
            }
            ':' => {
                let (name, rest) = split_simple_name(chars.as_str());
                if let Some(pseudo) = parse_pseudo_class(name) {
                    selector.pseudo_classes.push(pseudo);
                }
                input = rest;
            }
            '[' => {
                let (attr, rest) = split_until(input, ']');
                if let Some(attr) = attr.strip_prefix('[') {
                    if let Some(sel) = parse_attribute_selector(attr) {
                        selector.attributes.push(sel);
                    }
                }
                input = rest;
            }
            _ => break,
        }
    }

    selector
}

fn split_simple_name(input: &str) -> (&str, &str) {
    let end = input
        .find(|ch: char| matches!(ch, '.' | '#' | ':' | '['))
        .unwrap_or(input.len());
    (input[..end].trim(), &input[end..])
}

fn split_until(input: &str, delimiter: char) -> (&str, &str) {
    let Some(end) = input.find(delimiter) else {
        return (input, "");
    };
    (&input[..=end], &input[end + 1..])
}

fn parse_attribute_selector(input: &str) -> Option<AttributeSelector> {
    let mut rest = input.trim();
    if rest.is_empty() {
        return None;
    }

    let name_end = rest
        .find(|ch: char| ch.is_whitespace() || ch == '=')
        .unwrap_or(rest.len());
    let name = rest[..name_end].trim().to_ascii_lowercase();
    rest = rest[name_end..].trim_start();

    if name.is_empty() {
        return None;
    }

    if !rest.starts_with('=') {
        return Some(AttributeSelector { name, value: None });
    }

    rest = rest[1..].trim_start();
    let (value, remaining) = parse_attribute_value(rest);
    let value = value.map(|v| v.to_owned());
    let _ = remaining;
    Some(AttributeSelector { name, value })
}

fn parse_attribute_value(input: &str) -> (Option<&str>, &str) {
    let mut rest = input.trim_start();
    if rest.is_empty() {
        return (None, rest);
    }

    let quote = match rest.chars().next() {
        Some('\'') => Some('\''),
        Some('"') => Some('"'),
        _ => None,
    };

    if let Some(quote) = quote {
        rest = &rest[1..];
        let end = rest.find(quote).unwrap_or(rest.len());
        let value = &rest[..end];
        let rest = rest.get(end + 1..).unwrap_or("");
        return (Some(value), rest);
    }

    let end = rest
        .find(|ch: char| ch.is_whitespace() || ch == ']')
        .unwrap_or(rest.len());
    (Some(&rest[..end]), &rest[end..])
}

fn parse_pseudo_class(name: &str) -> Option<PseudoClass> {
    match name {
        "link" => Some(PseudoClass::Link),
        "visited" => Some(PseudoClass::Visited),
        "hover" => Some(PseudoClass::Hover),
        _ => None,
    }
}

struct DeclarationParser<'a> {
    input: &'a str,
    cursor: usize,
}

impl<'a> DeclarationParser<'a> {
    fn new(input: &'a str) -> DeclarationParser<'a> {
        DeclarationParser { input, cursor: 0 }
    }

    fn parse_all(&mut self) -> Vec<Declaration> {
        let mut declarations = Vec::new();

        while self.skip_ws_and_comments() {
            if self.peek_char() == Some('}') {
                return declarations;
            }

            let Some(name) = self.consume_name() else {
                break;
            };
            self.skip_ws_and_comments();

            if self.peek_char() != Some(':') {
                self.consume_until(';');
                continue;
            }
            self.cursor += 1;

            let value = self.consume_value();
            if !name.is_empty() && !value.is_empty() {
                declarations.push(Declaration { name, value });
            }

            self.skip_ws_and_comments();
            if self.peek_char() == Some(';') {
                self.cursor += 1;
            }
        }

        declarations
    }

    fn skip_ws_and_comments(&mut self) -> bool {
        let mut progressed = false;
        loop {
            let before = self.cursor;
            self.skip_whitespace();
            self.skip_comment();
            if self.cursor == before {
                break;
            }
            progressed = true;
        }
        self.cursor < self.input.len() || progressed
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek_char() {
            if !ch.is_whitespace() {
                break;
            }
            self.cursor += ch.len_utf8();
        }
    }

    fn skip_comment(&mut self) {
        if !self.input[self.cursor..].starts_with("/*") {
            return;
        }
        self.cursor += 2;
        if let Some(end) = self.input[self.cursor..].find("*/") {
            self.cursor += end + 2;
        } else {
            self.cursor = self.input.len();
        }
    }

    fn consume_name(&mut self) -> Option<String> {
        self.skip_ws_and_comments();
        let start = self.cursor;
        while let Some(ch) = self.peek_char() {
            if ch.is_whitespace() || ch == ':' || ch == ';' || ch == '{' || ch == '}' {
                break;
            }
            self.cursor += ch.len_utf8();
        }
        let name = self.input[start..self.cursor].trim();
        if name.is_empty() {
            return None;
        }
        Some(name.to_ascii_lowercase())
    }

    fn consume_value(&mut self) -> String {
        let start = self.cursor;
        let mut depth_parens = 0usize;
        let mut quote: Option<char> = None;

        while let Some(ch) = self.peek_char() {
            if quote.is_some() {
                self.cursor += ch.len_utf8();
                if ch == '\\' {
                    if let Some(next) = self.peek_char() {
                        self.cursor += next.len_utf8();
                    }
                    continue;
                }
                if Some(ch) == quote {
                    quote = None;
                }
                continue;
            }

            match ch {
                '"' | '\'' => {
                    quote = Some(ch);
                    self.cursor += ch.len_utf8();
                }
                '(' => {
                    depth_parens = depth_parens.saturating_add(1);
                    self.cursor += 1;
                }
                ')' => {
                    depth_parens = depth_parens.saturating_sub(1);
                    self.cursor += 1;
                }
                ';' | '}' if depth_parens == 0 => break,
                '/' if self.input[self.cursor..].starts_with("/*") => self.skip_comment(),
                _ => self.cursor += ch.len_utf8(),
            }
        }

        self.input[start..self.cursor].trim().to_owned()
    }

    fn consume_until(&mut self, delimiter: char) {
        while let Some(ch) = self.peek_char() {
            self.cursor += ch.len_utf8();
            if ch == delimiter {
                return;
            }
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.cursor..].chars().next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rules_and_declarations() {
        let sheet = Stylesheet::parse(
            "body { color: #000000; font-size: 10pt; }\n\
             a:link { text-decoration: none; }",
        );
        assert_eq!(sheet.rules.len(), 2);
        assert_eq!(sheet.rules[0].selectors.len(), 1);
        assert_eq!(sheet.rules[0].declarations.len(), 2);
    }

    #[test]
    fn ignores_media_queries() {
        let sheet = Stylesheet::parse(
            "@media only screen { body { color: #ffffff; } }\n\
             body { color: #000000; }",
        );
        assert_eq!(sheet.rules.len(), 1);
        assert_eq!(sheet.rules[0].declarations[0].value, "#000000");
    }

    #[test]
    fn parses_descendant_and_class_selectors() {
        let sheet = Stylesheet::parse(".title a:link { color: #000000; }");
        let selector = &sheet.rules[0].selectors[0];
        assert_eq!(selector.parts.len(), 2);
        assert_eq!(selector.parts[0].classes, vec!["title".to_owned()]);
        assert_eq!(selector.parts[1].tag.as_deref(), Some("a"));
        assert_eq!(selector.parts[1].pseudo_classes, vec![PseudoClass::Link]);
    }

    #[test]
    fn parses_attribute_selectors() {
        let sheet = Stylesheet::parse("input[type='submit'] { font-family: monospace; }");
        let selector = &sheet.rules[0].selectors[0];
        assert_eq!(selector.parts.len(), 1);
        assert_eq!(selector.parts[0].tag.as_deref(), Some("input"));
        assert_eq!(selector.parts[0].attributes.len(), 1);
        assert_eq!(selector.parts[0].attributes[0].name, "type");
        assert_eq!(selector.parts[0].attributes[0].value.as_deref(), Some("submit"));
    }

    #[test]
    fn parses_inline_declarations() {
        let decls = parse_inline_declarations("padding:2px; width: 10px");
        assert_eq!(decls.len(), 2);
        assert_eq!(decls[0].name, "padding");
        assert_eq!(decls[0].value, "2px");
    }
}
