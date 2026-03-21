use crate::dom::{Attributes, Document, Element, ElementId, Node};
use std::collections::HashMap;

pub fn execute(document: &mut Document, source: &str) -> Result<(), String> {
    let tokens = Lexer::new(source).tokenize()?;
    let program = Parser::new(tokens).parse_program()?;
    Executor::new(document).execute_program(&program)
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Statement {
    VariableDeclaration {
        name: String,
        init: Option<Expression>,
    },
    Expression(Expression),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Expression {
    Identifier(String),
    String(String),
    Number(i64),
    Member {
        object: Box<Expression>,
        property: String,
    },
    Call {
        callee: Box<Expression>,
        arguments: Vec<Expression>,
    },
    Assignment {
        target: Box<Expression>,
        value: Box<Expression>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Token {
    Identifier(String),
    String(String),
    Number(i64),
    KeywordVar,
    KeywordLet,
    KeywordConst,
    Dot,
    LeftParen,
    RightParen,
    Comma,
    Semicolon,
    Equal,
    Eof,
}

struct Lexer<'a> {
    input: &'a str,
    cursor: usize,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, cursor: 0 }
    }

    fn tokenize(mut self) -> Result<Vec<Token>, String> {
        let mut out = Vec::new();

        while let Some(ch) = self.peek_char() {
            match ch {
                ch if ch.is_ascii_whitespace() => {
                    self.advance_char();
                }
                '/' if self.starts_with("//") => {
                    self.consume_line_comment();
                }
                '/' if self.starts_with("/*") => {
                    self.consume_block_comment()?;
                }
                '.' => {
                    self.advance_char();
                    out.push(Token::Dot);
                }
                '(' => {
                    self.advance_char();
                    out.push(Token::LeftParen);
                }
                ')' => {
                    self.advance_char();
                    out.push(Token::RightParen);
                }
                ',' => {
                    self.advance_char();
                    out.push(Token::Comma);
                }
                ';' => {
                    self.advance_char();
                    out.push(Token::Semicolon);
                }
                '=' => {
                    self.advance_char();
                    out.push(Token::Equal);
                }
                '"' | '\'' => out.push(Token::String(self.consume_string()?)),
                ch if ch.is_ascii_digit() => out.push(Token::Number(self.consume_number()?)),
                ch if is_identifier_start(ch) => {
                    let identifier = self.consume_identifier();
                    out.push(match identifier.as_str() {
                        "var" => Token::KeywordVar,
                        "let" => Token::KeywordLet,
                        "const" => Token::KeywordConst,
                        _ => Token::Identifier(identifier),
                    });
                }
                _ => {
                    return Err(format!("Unsupported token in JS source near byte {}", self.cursor));
                }
            }
        }

        out.push(Token::Eof);
        Ok(out)
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.cursor..].chars().next()
    }

    fn starts_with(&self, prefix: &str) -> bool {
        self.input[self.cursor..].starts_with(prefix)
    }

    fn advance_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.cursor += ch.len_utf8();
        Some(ch)
    }

    fn consume_line_comment(&mut self) {
        while let Some(ch) = self.advance_char() {
            if ch == '\n' {
                break;
            }
        }
    }

    fn consume_block_comment(&mut self) -> Result<(), String> {
        self.cursor += 2;
        while self.cursor < self.input.len() {
            if self.starts_with("*/") {
                self.cursor += 2;
                return Ok(());
            }
            self.advance_char();
        }
        Err("Unterminated block comment".to_owned())
    }

    fn consume_string(&mut self) -> Result<String, String> {
        let quote = self
            .advance_char()
            .ok_or_else(|| "Missing string delimiter".to_owned())?;
        let mut out = String::new();

        while let Some(ch) = self.advance_char() {
            if ch == quote {
                return Ok(out);
            }
            if ch == '\\' {
                let escaped = self
                    .advance_char()
                    .ok_or_else(|| "Unterminated string escape".to_owned())?;
                match escaped {
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    '\\' => out.push('\\'),
                    '\'' => out.push('\''),
                    '"' => out.push('"'),
                    other => out.push(other),
                }
                continue;
            }
            out.push(ch);
        }

        Err("Unterminated string literal".to_owned())
    }

    fn consume_number(&mut self) -> Result<i64, String> {
        let start = self.cursor;
        while self.peek_char().is_some_and(|ch| ch.is_ascii_digit()) {
            self.advance_char();
        }
        self.input[start..self.cursor]
            .parse::<i64>()
            .map_err(|_| "Invalid numeric literal".to_owned())
    }

    fn consume_identifier(&mut self) -> String {
        let start = self.cursor;
        self.advance_char();
        while self.peek_char().is_some_and(is_identifier_continue) {
            self.advance_char();
        }
        self.input[start..self.cursor].to_owned()
    }
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, cursor: 0 }
    }

    fn parse_program(mut self) -> Result<Vec<Statement>, String> {
        let mut out = Vec::new();
        while !self.is_at_end() {
            while self.match_token(&Token::Semicolon) {}
            if self.is_at_end() {
                break;
            }
            out.push(self.parse_statement()?);
            while self.match_token(&Token::Semicolon) {}
        }
        Ok(out)
    }

    fn parse_statement(&mut self) -> Result<Statement, String> {
        if self.match_any(&[Token::KeywordVar, Token::KeywordLet, Token::KeywordConst]) {
            let name = self.expect_identifier()?;
            let init = if self.match_token(&Token::Equal) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            return Ok(Statement::VariableDeclaration { name, init });
        }

        Ok(Statement::Expression(self.parse_expression()?))
    }

    fn parse_expression(&mut self) -> Result<Expression, String> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expression, String> {
        let expression = self.parse_call_member()?;
        if self.match_token(&Token::Equal) {
            let value = self.parse_assignment()?;
            return Ok(Expression::Assignment {
                target: Box::new(expression),
                value: Box::new(value),
            });
        }
        Ok(expression)
    }

    fn parse_call_member(&mut self) -> Result<Expression, String> {
        let mut expression = self.parse_primary()?;

        loop {
            if self.match_token(&Token::Dot) {
                let property = self.expect_identifier()?;
                expression = Expression::Member {
                    object: Box::new(expression),
                    property,
                };
                continue;
            }

            if self.match_token(&Token::LeftParen) {
                let mut arguments = Vec::new();
                if !self.check(&Token::RightParen) {
                    loop {
                        arguments.push(self.parse_expression()?);
                        if !self.match_token(&Token::Comma) {
                            break;
                        }
                    }
                }
                self.expect(&Token::RightParen)?;
                expression = Expression::Call {
                    callee: Box::new(expression),
                    arguments,
                };
                continue;
            }

            break;
        }

        Ok(expression)
    }

    fn parse_primary(&mut self) -> Result<Expression, String> {
        match self.advance() {
            Token::Identifier(name) => Ok(Expression::Identifier(name)),
            Token::String(value) => Ok(Expression::String(value)),
            Token::Number(value) => Ok(Expression::Number(value)),
            Token::LeftParen => {
                let expression = self.parse_expression()?;
                self.expect(&Token::RightParen)?;
                Ok(expression)
            }
            other => Err(format!("Unexpected token in JS expression: {other:?}")),
        }
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek(), Token::Eof)
    }

    fn peek(&self) -> Token {
        self.tokens.get(self.cursor).cloned().unwrap_or(Token::Eof)
    }

    fn advance(&mut self) -> Token {
        let token = self.peek();
        self.cursor = self.cursor.saturating_add(1);
        token
    }

    fn match_any(&mut self, tokens: &[Token]) -> bool {
        for token in tokens {
            if self.check(token) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn match_token(&mut self, token: &Token) -> bool {
        if self.check(token) {
            self.advance();
            return true;
        }
        false
    }

    fn check(&self, token: &Token) -> bool {
        self.peek() == *token
    }

    fn expect(&mut self, token: &Token) -> Result<(), String> {
        if self.match_token(token) {
            return Ok(());
        }
        Err(format!("Expected token {token:?}, found {:?}", self.peek()))
    }

    fn expect_identifier(&mut self) -> Result<String, String> {
        match self.advance() {
            Token::Identifier(name) => Ok(name),
            other => Err(format!("Expected identifier, found {other:?}")),
        }
    }
}

#[derive(Clone, Debug)]
enum Value {
    Undefined,
    String(String),
    Bool(bool),
    Number(i64),
    Window,
    Document,
    LiveElement(ElementId),
    DetachedElement(Element),
    DetachedText(String),
    ClassList(ElementId),
    NodeList(Vec<ElementId>),
}

struct Executor<'a> {
    document: &'a mut Document,
    env: HashMap<String, Value>,
}

impl<'a> Executor<'a> {
    fn new(document: &'a mut Document) -> Self {
        let mut env = HashMap::new();
        env.insert("window".to_owned(), Value::Window);
        env.insert("document".to_owned(), Value::Document);
        Self { document, env }
    }

    fn execute_program(&mut self, statements: &[Statement]) -> Result<(), String> {
        for statement in statements {
            self.execute_statement(statement)?;
        }
        Ok(())
    }

    fn execute_statement(&mut self, statement: &Statement) -> Result<Value, String> {
        match statement {
            Statement::VariableDeclaration { name, init } => {
                let value = if let Some(init) = init {
                    self.evaluate_expression(init)?
                } else {
                    Value::Undefined
                };
                self.env.insert(name.clone(), value.clone());
                Ok(value)
            }
            Statement::Expression(expression) => self.evaluate_expression(expression),
        }
    }

    fn evaluate_expression(&mut self, expression: &Expression) -> Result<Value, String> {
        match expression {
            Expression::Identifier(name) => Ok(self.env.get(name).cloned().unwrap_or(Value::Undefined)),
            Expression::String(value) => Ok(Value::String(value.clone())),
            Expression::Number(value) => Ok(Value::Number(*value)),
            Expression::Member { object, property } => {
                let object = self.evaluate_expression(object)?;
                self.get_member_value(object, property)
            }
            Expression::Call { callee, arguments } => self.evaluate_call(callee, arguments),
            Expression::Assignment { target, value } => {
                let value = self.evaluate_expression(value)?;
                self.assign_target(target, value.clone())?;
                Ok(value)
            }
        }
    }

    fn assign_target(&mut self, target: &Expression, value: Value) -> Result<(), String> {
        match target {
            Expression::Identifier(name) => {
                self.env.insert(name.clone(), value);
                Ok(())
            }
            Expression::Member { object, property } => {
                if let Expression::Identifier(name) = object.as_ref()
                    && let Some(bound) = self.env.get_mut(name)
                    && set_detached_member_value(bound, property, &value)
                {
                    return Ok(());
                }
                let object = self.evaluate_expression(object)?;
                self.set_member_value(object, property, value)
            }
            _ => Err("Unsupported assignment target".to_owned()),
        }
    }

    fn get_member_value(&mut self, object: Value, property: &str) -> Result<Value, String> {
        match object {
            Value::Window => match property {
                "document" => Ok(Value::Document),
                "window" => Ok(Value::Window),
                _ => Ok(Value::Undefined),
            },
            Value::Document => match property {
                "documentElement" => Ok(self
                    .document
                    .find_first_element_by_name("html")
                    .map(|element| Value::LiveElement(element.node_id))
                    .unwrap_or(Value::Undefined)),
                "body" => Ok(self
                    .document
                    .find_first_element_by_name("body")
                    .map(|element| Value::LiveElement(element.node_id))
                    .unwrap_or(Value::Undefined)),
                _ => Ok(Value::Undefined),
            },
            Value::LiveElement(node_id) => {
                let Some(element) = self.document.find_element_by_node_id(node_id) else {
                    return Ok(Value::Undefined);
                };
                match property {
                    "classList" => Ok(Value::ClassList(node_id)),
                    "className" => Ok(Value::String(element.attributes.classes.join(" "))),
                    "textContent" => Ok(Value::String(collect_text_content(element))),
                    "id" => Ok(Value::String(
                        element.attributes.id.clone().unwrap_or_default(),
                    )),
                    "checked" => Ok(Value::Bool(element.attributes.get("checked").is_some())),
                    "disabled" => Ok(Value::Bool(element.attributes.get("disabled").is_some())),
                    "value" => Ok(Value::String(
                        element.attributes.get("value").unwrap_or("").to_owned(),
                    )),
                    _ => Ok(Value::Undefined),
                }
            }
            Value::NodeList(ids) => match property {
                "length" => Ok(Value::Number(ids.len().try_into().unwrap_or(i64::MAX))),
                _ => Ok(Value::Undefined),
            },
            _ => Ok(Value::Undefined),
        }
    }

    fn set_member_value(
        &mut self,
        object: Value,
        property: &str,
        value: Value,
    ) -> Result<(), String> {
        match object {
            Value::LiveElement(node_id) => {
                let Some(element) = self.document.find_element_by_node_id_mut(node_id) else {
                    return Ok(());
                };
                match property {
                    "className" => {
                        element.attributes.classes = value
                            .to_string_value()
                            .split_whitespace()
                            .map(str::to_owned)
                            .collect();
                        Ok(())
                    }
                    "textContent" => {
                        element.set_text_content(value.to_string_value());
                        Ok(())
                    }
                    "id" => {
                        let next = value.to_string_value();
                        if next.is_empty() {
                            element.attributes.remove("id");
                        } else {
                            element.attributes.insert("id".to_owned(), next);
                        }
                        Ok(())
                    }
                    "checked" | "disabled" => {
                        if value.is_truthy() {
                            element
                                .attributes
                                .insert(property.to_owned(), property.to_owned());
                        } else {
                            element.attributes.remove(property);
                        }
                        Ok(())
                    }
                    "value" => {
                        element
                            .attributes
                            .insert("value".to_owned(), value.to_string_value());
                        Ok(())
                    }
                    _ => Err(format!("Unsupported element property assignment: {property}")),
                }
            }
            _ => Err(format!("Unsupported assignment through property {property}")),
        }
    }

    fn evaluate_call(
        &mut self,
        callee: &Expression,
        arguments: &[Expression],
    ) -> Result<Value, String> {
        let evaluated_arguments = arguments
            .iter()
            .map(|argument| self.evaluate_expression(argument))
            .collect::<Result<Vec<_>, _>>()?;

        let Expression::Member { object, property } = callee else {
            return Err("Unsupported JS call target".to_owned());
        };
        let object = self.evaluate_expression(object)?;

        match object {
            Value::Document => self.call_document_method(property, &evaluated_arguments),
            Value::LiveElement(node_id) => {
                self.call_element_method(node_id, property, &evaluated_arguments)
            }
            Value::ClassList(node_id) => {
                self.call_class_list_method(node_id, property, &evaluated_arguments)
            }
            _ => Err(format!("Unsupported method call: {property}")),
        }
    }

    fn call_document_method(
        &mut self,
        property: &str,
        arguments: &[Value],
    ) -> Result<Value, String> {
        match property {
            "getElementById" => {
                let id = first_string_argument(arguments)?;
                Ok(self
                    .document
                    .find_first_element_by_id(id.as_str())
                    .map(|element| Value::LiveElement(element.node_id))
                    .unwrap_or(Value::Undefined))
            }
            "querySelector" => {
                let selector = first_string_argument(arguments)?;
                Ok(self
                    .document
                    .query_selector(selector.as_str())
                    .map(|element| Value::LiveElement(element.node_id))
                    .unwrap_or(Value::Undefined))
            }
            "querySelectorAll" => {
                let selector = first_string_argument(arguments)?;
                Ok(Value::NodeList(
                    self.document.query_selector_all_ids(selector.as_str()),
                ))
            }
            "createElement" => {
                let name = first_string_argument(arguments)?;
                Ok(Value::DetachedElement(Element::new(
                    name.to_ascii_lowercase(),
                    Attributes::default(),
                    Vec::new(),
                )))
            }
            "createTextNode" => Ok(Value::DetachedText(first_string_argument(arguments)?)),
            _ => Err(format!("Unsupported document method: {property}")),
        }
    }

    fn call_element_method(
        &mut self,
        node_id: ElementId,
        property: &str,
        arguments: &[Value],
    ) -> Result<Value, String> {
        match property {
            "querySelector" => {
                let selector = first_string_argument(arguments)?;
                let element_id = self
                    .document
                    .find_element_by_node_id(node_id)
                    .and_then(|element| element.query_selector(selector.as_str()))
                    .map(|element| element.node_id);
                Ok(element_id.map(Value::LiveElement).unwrap_or(Value::Undefined))
            }
            "querySelectorAll" => {
                let selector = first_string_argument(arguments)?;
                let ids = self
                    .document
                    .find_element_by_node_id(node_id)
                    .map(|element| {
                        element
                            .query_selector_all(selector.as_str())
                            .into_iter()
                            .map(|element| element.node_id)
                            .collect()
                    })
                    .unwrap_or_default();
                Ok(Value::NodeList(ids))
            }
            "appendChild" => self.insert_child(node_id, arguments, InsertPosition::Append),
            "prepend" => self.insert_child(node_id, arguments, InsertPosition::Prepend),
            "replaceWith" => self.replace_element(node_id, arguments),
            "remove" => {
                self.document.remove_element(node_id);
                Ok(Value::Undefined)
            }
            _ => Err(format!("Unsupported element method: {property}")),
        }
    }

    fn call_class_list_method(
        &mut self,
        node_id: ElementId,
        property: &str,
        arguments: &[Value],
    ) -> Result<Value, String> {
        let Some(element) = self.document.find_element_by_node_id_mut(node_id) else {
            return Ok(Value::Undefined);
        };

        match property {
            "add" => {
                for token in collect_class_tokens(arguments) {
                    if !element.attributes.has_class(token.as_str()) {
                        element.attributes.classes.push(token);
                    }
                }
                Ok(Value::Undefined)
            }
            "remove" => {
                let tokens = collect_class_tokens(arguments);
                element
                    .attributes
                    .classes
                    .retain(|existing| !tokens.iter().any(|token| token == existing));
                Ok(Value::Undefined)
            }
            "contains" => {
                let token = first_string_argument(arguments)?;
                Ok(Value::Bool(element.attributes.has_class(token.as_str())))
            }
            _ => Err(format!("Unsupported classList method: {property}")),
        }
    }

    fn insert_child(
        &mut self,
        parent_id: ElementId,
        arguments: &[Value],
        position: InsertPosition,
    ) -> Result<Value, String> {
        let Some(node) = arguments.first().cloned() else {
            return Ok(Value::Undefined);
        };

        match node {
            Value::DetachedElement(mut element) => {
                self.document.assign_element_ids(&mut element);
                let inserted_id = element.node_id;
                let inserted = match position {
                    InsertPosition::Append => {
                        self.document.append_child_to(parent_id, Node::Element(element))
                    }
                    InsertPosition::Prepend => {
                        self.document.prepend_child_to(parent_id, Node::Element(element))
                    }
                };
                if inserted {
                    Ok(Value::LiveElement(inserted_id))
                } else {
                    Ok(Value::Undefined)
                }
            }
            Value::DetachedText(text) => {
                let inserted = match position {
                    InsertPosition::Append => self.document.append_child_to(parent_id, Node::Text(text)),
                    InsertPosition::Prepend => {
                        self.document.prepend_child_to(parent_id, Node::Text(text))
                    }
                };
                Ok(if inserted { Value::Undefined } else { Value::Undefined })
            }
            Value::LiveElement(existing_id) => {
                let Some(existing) = self.document.find_element_by_node_id(existing_id).cloned() else {
                    return Ok(Value::Undefined);
                };
                let inserted = match position {
                    InsertPosition::Append => self.document.append_child_to(parent_id, Node::Element(existing)),
                    InsertPosition::Prepend => self.document.prepend_child_to(parent_id, Node::Element(existing)),
                };
                Ok(if inserted {
                    Value::LiveElement(existing_id)
                } else {
                    Value::Undefined
                })
            }
            _ => Err("Unsupported node argument for append/prepend".to_owned()),
        }
    }

    fn replace_element(&mut self, node_id: ElementId, arguments: &[Value]) -> Result<Value, String> {
        let Some(value) = arguments.first().cloned() else {
            return Ok(Value::Undefined);
        };
        match value {
            Value::DetachedElement(mut element) => {
                self.document.assign_element_ids(&mut element);
                let replacement_id = element.node_id;
                if self.document.replace_element_with(node_id, element) {
                    Ok(Value::LiveElement(replacement_id))
                } else {
                    Ok(Value::Undefined)
                }
            }
            Value::LiveElement(existing_id) => {
                let Some(existing) = self.document.find_element_by_node_id(existing_id).cloned() else {
                    return Ok(Value::Undefined);
                };
                if self.document.replace_element_with(node_id, existing) {
                    Ok(Value::LiveElement(existing_id))
                } else {
                    Ok(Value::Undefined)
                }
            }
            _ => Err("Unsupported replaceWith argument".to_owned()),
        }
    }
}

#[derive(Clone, Copy)]
enum InsertPosition {
    Append,
    Prepend,
}

impl Value {
    fn to_string_value(&self) -> String {
        match self {
            Value::Undefined => String::new(),
            Value::String(value) => value.clone(),
            Value::Bool(value) => {
                if *value {
                    "true".to_owned()
                } else {
                    "false".to_owned()
                }
            }
            Value::Number(value) => value.to_string(),
            Value::DetachedText(value) => value.clone(),
            _ => String::new(),
        }
    }

    fn is_truthy(&self) -> bool {
        match self {
            Value::Undefined => false,
            Value::String(value) => !value.is_empty(),
            Value::Bool(value) => *value,
            Value::Number(value) => *value != 0,
            _ => true,
        }
    }
}

fn collect_text_content(element: &Element) -> String {
    let mut out = String::new();
    collect_text_content_into(element, &mut out);
    out
}

fn collect_text_content_into(element: &Element, out: &mut String) {
    for child in &element.children {
        match child {
            Node::Text(text) => out.push_str(text),
            Node::Element(element) => collect_text_content_into(element, out),
        }
    }
}

fn first_string_argument(arguments: &[Value]) -> Result<String, String> {
    let Some(value) = arguments.first() else {
        return Err("Missing string argument".to_owned());
    };
    Ok(value.to_string_value())
}

fn collect_class_tokens(arguments: &[Value]) -> Vec<String> {
    let mut out = Vec::new();
    for argument in arguments {
        for token in argument.to_string_value().split_whitespace() {
            if !token.is_empty() {
                out.push(token.to_owned());
            }
        }
    }
    out
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch == '$' || ch.is_ascii_alphabetic()
}

fn set_detached_member_value(target: &mut Value, property: &str, value: &Value) -> bool {
    let Value::DetachedElement(element) = target else {
        return false;
    };

    match property {
        "className" => {
            element.attributes.classes = value
                .to_string_value()
                .split_whitespace()
                .map(str::to_owned)
                .collect();
            true
        }
        "textContent" => {
            element.set_text_content(value.to_string_value());
            true
        }
        "id" => {
            let next = value.to_string_value();
            if next.is_empty() {
                element.attributes.remove("id");
            } else {
                element.attributes.insert("id".to_owned(), next);
            }
            true
        }
        "checked" | "disabled" => {
            if value.is_truthy() {
                element
                    .attributes
                    .insert(property.to_owned(), property.to_owned());
            } else {
                element.attributes.remove(property);
            }
            true
        }
        "value" => {
            element
                .attributes
                .insert("value".to_owned(), value.to_string_value());
            true
        }
        _ => false,
    }
}

fn is_identifier_continue(ch: char) -> bool {
    is_identifier_start(ch) || ch.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executes_text_content_assignment_against_dom() {
        let mut document = crate::html::parse_document(r#"<div id="greeting">Welcome</div>"#);

        execute(
            &mut document,
            r#"document.getElementById("greeting").textContent = "Hello World!";"#,
        )
        .expect("script should execute");

        let greeting = document
            .find_first_element_by_id("greeting")
            .expect("missing greeting");
        assert_eq!(greeting.children, vec![Node::Text("Hello World!".to_owned())]);
    }

    #[test]
    fn executes_class_list_operations_and_variable_bindings() {
        let mut document = crate::html::parse_document(
            r#"<html class="client-nojs"><body><div id="target" class="collapsed"></div></body></html>"#,
        );

        execute(
            &mut document,
            r#"
            let root = document.documentElement;
            root.classList.add("client-js", "vector-animations-ready");
            root.classList.remove("client-nojs");
            let target = document.getElementById("target");
            target.classList.add("expanded");
            target.classList.remove("collapsed");
            "#,
        )
        .expect("script should execute");

        let html = document
            .find_first_element_by_name("html")
            .expect("missing html element");
        let target = document
            .find_first_element_by_id("target")
            .expect("missing target");

        assert!(html.attributes.has_class("client-js"));
        assert!(html.attributes.has_class("vector-animations-ready"));
        assert!(!html.attributes.has_class("client-nojs"));
        assert!(target.attributes.has_class("expanded"));
        assert!(!target.attributes.has_class("collapsed"));
    }

    #[test]
    fn supports_create_element_and_append_child() {
        let mut document = crate::html::parse_document(r#"<div id="host"></div>"#);

        execute(
            &mut document,
            r#"
            const host = document.getElementById("host");
            const child = document.createElement("span");
            child.id = "child";
            child.textContent = "Hello";
            host.appendChild(child);
            "#,
        )
        .expect("script should execute");

        let child = document
            .find_first_element_by_id("child")
            .expect("missing child");
        assert_eq!(child.children, vec![Node::Text("Hello".to_owned())]);
    }
}
