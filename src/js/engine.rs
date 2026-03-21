use crate::dom::{Attributes, Document, Element, ElementId, Node};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

type ScopeRef = Rc<RefCell<Scope>>;
type ObjectRef = Rc<RefCell<HashMap<String, Value>>>;
type ArrayRef = Rc<RefCell<Vec<Value>>>;

pub fn execute(document: &mut Document, source: &str) -> Result<(), String> {
    let tokens = Lexer::new(source).tokenize()?;
    let program = Parser::new(tokens).parse_program()?;
    Executor::new(document).execute_program(&program)
}

#[derive(Clone, Debug)]
enum Statement {
    VariableDeclaration {
        name: String,
        init: Option<Expression>,
    },
    FunctionDeclaration {
        name: String,
        params: Vec<String>,
        body: Vec<Statement>,
    },
    Return(Option<Expression>),
    If {
        test: Expression,
        consequent: Box<Statement>,
        alternate: Option<Box<Statement>>,
    },
    Block(Vec<Statement>),
    TryCatch {
        try_block: Vec<Statement>,
        catch_param: String,
        catch_block: Vec<Statement>,
    },
    Expression(Expression),
}

#[derive(Clone, Debug)]
enum ObjectKey {
    Identifier(String),
    String(String),
    Number(f64),
}

#[derive(Clone, Copy, Debug)]
enum UnaryOperator {
    Not,
    Typeof,
}

#[derive(Clone, Copy, Debug)]
enum BinaryOperator {
    Add,
    In,
    Equal,
    NotEqual,
    StrictEqual,
    StrictNotEqual,
    LogicalAnd,
    LogicalOr,
}

#[derive(Clone, Debug)]
enum Expression {
    Identifier(String),
    This,
    String(String),
    Number(f64),
    Bool(bool),
    Null,
    Regex {
        source: String,
        flags: String,
    },
    Array(Vec<Expression>),
    Object(Vec<(ObjectKey, Expression)>),
    Function {
        params: Vec<String>,
        body: Vec<Statement>,
    },
    Member {
        object: Box<Expression>,
        property: String,
    },
    ComputedMember {
        object: Box<Expression>,
        property: Box<Expression>,
    },
    Call {
        callee: Box<Expression>,
        arguments: Vec<Expression>,
    },
    Assignment {
        target: Box<Expression>,
        value: Box<Expression>,
    },
    Unary {
        operator: UnaryOperator,
        argument: Box<Expression>,
    },
    Binary {
        left: Box<Expression>,
        operator: BinaryOperator,
        right: Box<Expression>,
    },
    New {
        callee: Box<Expression>,
        arguments: Vec<Expression>,
    },
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Identifier(String),
    String(String),
    Number(f64),
    Regex { source: String, flags: String },
    KeywordVar,
    KeywordLet,
    KeywordConst,
    KeywordFunction,
    KeywordReturn,
    KeywordIf,
    KeywordElse,
    KeywordTrue,
    KeywordFalse,
    KeywordNull,
    KeywordThis,
    KeywordNew,
    KeywordTypeof,
    KeywordTry,
    KeywordCatch,
    KeywordIn,
    Dot,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    LeftBrace,
    RightBrace,
    Comma,
    Semicolon,
    Colon,
    Equal,
    EqualEqual,
    EqualEqualEqual,
    Bang,
    BangEqual,
    BangEqualEqual,
    Plus,
    AndAnd,
    OrOr,
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
                '/' => out.push(self.consume_regex_literal()?),
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
                '[' => {
                    self.advance_char();
                    out.push(Token::LeftBracket);
                }
                ']' => {
                    self.advance_char();
                    out.push(Token::RightBracket);
                }
                '{' => {
                    self.advance_char();
                    out.push(Token::LeftBrace);
                }
                '}' => {
                    self.advance_char();
                    out.push(Token::RightBrace);
                }
                ',' => {
                    self.advance_char();
                    out.push(Token::Comma);
                }
                ';' => {
                    self.advance_char();
                    out.push(Token::Semicolon);
                }
                ':' => {
                    self.advance_char();
                    out.push(Token::Colon);
                }
                '+' => {
                    self.advance_char();
                    out.push(Token::Plus);
                }
                '&' if self.starts_with("&&") => {
                    self.cursor += 2;
                    out.push(Token::AndAnd);
                }
                '|' if self.starts_with("||") => {
                    self.cursor += 2;
                    out.push(Token::OrOr);
                }
                '=' if self.starts_with("===") => {
                    self.cursor += 3;
                    out.push(Token::EqualEqualEqual);
                }
                '=' if self.starts_with("==") => {
                    self.cursor += 2;
                    out.push(Token::EqualEqual);
                }
                '=' => {
                    self.advance_char();
                    out.push(Token::Equal);
                }
                '!' if self.starts_with("!==") => {
                    self.cursor += 3;
                    out.push(Token::BangEqualEqual);
                }
                '!' if self.starts_with("!=") => {
                    self.cursor += 2;
                    out.push(Token::BangEqual);
                }
                '!' => {
                    self.advance_char();
                    out.push(Token::Bang);
                }
                '"' | '\'' => out.push(Token::String(self.consume_string()?)),
                ch if ch.is_ascii_digit() => out.push(Token::Number(self.consume_number()?)),
                ch if is_identifier_start(ch) => {
                    let identifier = self.consume_identifier();
                    out.push(match identifier.as_str() {
                        "var" => Token::KeywordVar,
                        "let" => Token::KeywordLet,
                        "const" => Token::KeywordConst,
                        "function" => Token::KeywordFunction,
                        "return" => Token::KeywordReturn,
                        "if" => Token::KeywordIf,
                        "else" => Token::KeywordElse,
                        "true" => Token::KeywordTrue,
                        "false" => Token::KeywordFalse,
                        "null" => Token::KeywordNull,
                        "this" => Token::KeywordThis,
                        "new" => Token::KeywordNew,
                        "typeof" => Token::KeywordTypeof,
                        "try" => Token::KeywordTry,
                        "catch" => Token::KeywordCatch,
                        "in" => Token::KeywordIn,
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

    fn consume_number(&mut self) -> Result<f64, String> {
        let start = self.cursor;
        if self.starts_with("0x") || self.starts_with("0X") {
            self.cursor += 2;
            while self.peek_char().is_some_and(|ch| ch.is_ascii_hexdigit()) {
                self.advance_char();
            }
            let value = u64::from_str_radix(&self.input[start + 2..self.cursor], 16)
                .map_err(|_| "Invalid hex literal".to_owned())?;
            return Ok(value as f64);
        }

        while self.peek_char().is_some_and(|ch| ch.is_ascii_digit()) {
            self.advance_char();
        }
        if self.peek_char() == Some('.') {
            self.advance_char();
            while self.peek_char().is_some_and(|ch| ch.is_ascii_digit()) {
                self.advance_char();
            }
        }
        if self.peek_char().is_some_and(|ch| matches!(ch, 'e' | 'E')) {
            self.advance_char();
            if self.peek_char().is_some_and(|ch| matches!(ch, '+' | '-')) {
                self.advance_char();
            }
            while self.peek_char().is_some_and(|ch| ch.is_ascii_digit()) {
                self.advance_char();
            }
        }

        self.input[start..self.cursor]
            .parse::<f64>()
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

    fn consume_regex_literal(&mut self) -> Result<Token, String> {
        self.advance_char();
        let mut source = String::new();
        let mut escaped = false;
        let mut in_class = false;

        while let Some(ch) = self.advance_char() {
            if escaped {
                source.push(ch);
                escaped = false;
                continue;
            }
            match ch {
                '\\' => {
                    source.push(ch);
                    escaped = true;
                }
                '[' => {
                    source.push(ch);
                    in_class = true;
                }
                ']' => {
                    source.push(ch);
                    in_class = false;
                }
                '/' if !in_class => {
                    let flags = self.consume_regex_flags();
                    return Ok(Token::Regex { source, flags });
                }
                _ => source.push(ch),
            }
        }

        Err("Unterminated regex literal".to_owned())
    }

    fn consume_regex_flags(&mut self) -> String {
        let start = self.cursor;
        while self.peek_char().is_some_and(|ch| ch.is_ascii_alphabetic()) {
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
            self.match_token(&Token::Semicolon);
            return Ok(Statement::VariableDeclaration { name, init });
        }

        if self.match_token(&Token::KeywordFunction) {
            let name = self.expect_identifier()?;
            let (params, body) = self.parse_function_signature_and_body()?;
            return Ok(Statement::FunctionDeclaration { name, params, body });
        }

        if self.match_token(&Token::KeywordReturn) {
            let expression = if self.check(&Token::Semicolon)
                || self.check(&Token::RightBrace)
                || self.is_at_end()
            {
                None
            } else {
                Some(self.parse_expression()?)
            };
            self.match_token(&Token::Semicolon);
            return Ok(Statement::Return(expression));
        }

        if self.match_token(&Token::KeywordIf) {
            self.expect(&Token::LeftParen)?;
            let test = self.parse_expression()?;
            self.expect(&Token::RightParen)?;
            let consequent = Box::new(self.parse_statement()?);
            let alternate = if self.match_token(&Token::KeywordElse) {
                Some(Box::new(self.parse_statement()?))
            } else {
                None
            };
            return Ok(Statement::If {
                test,
                consequent,
                alternate,
            });
        }

        if self.match_token(&Token::KeywordTry) {
            let try_block = self.parse_block_statements()?;
            self.expect(&Token::KeywordCatch)?;
            self.expect(&Token::LeftParen)?;
            let catch_param = self.expect_identifier()?;
            self.expect(&Token::RightParen)?;
            let catch_block = self.parse_block_statements()?;
            return Ok(Statement::TryCatch {
                try_block,
                catch_param,
                catch_block,
            });
        }

        if self.check(&Token::LeftBrace) {
            return Ok(Statement::Block(self.parse_block_statements()?));
        }

        let expression = self.parse_expression()?;
        self.match_token(&Token::Semicolon);
        Ok(Statement::Expression(expression))
    }

    fn parse_block_statements(&mut self) -> Result<Vec<Statement>, String> {
        self.expect(&Token::LeftBrace)?;
        let mut out = Vec::new();
        while !self.check(&Token::RightBrace) && !self.is_at_end() {
            while self.match_token(&Token::Semicolon) {}
            if self.check(&Token::RightBrace) {
                break;
            }
            out.push(self.parse_statement()?);
        }
        self.expect(&Token::RightBrace)?;
        Ok(out)
    }

    fn parse_function_signature_and_body(&mut self) -> Result<(Vec<String>, Vec<Statement>), String> {
        self.expect(&Token::LeftParen)?;
        let mut params = Vec::new();
        if !self.check(&Token::RightParen) {
            loop {
                params.push(self.expect_identifier()?);
                if !self.match_token(&Token::Comma) {
                    break;
                }
            }
        }
        self.expect(&Token::RightParen)?;
        let body = self.parse_block_statements()?;
        Ok((params, body))
    }

    fn parse_expression(&mut self) -> Result<Expression, String> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expression, String> {
        let expression = self.parse_logical_or()?;
        if self.match_token(&Token::Equal) {
            let value = self.parse_assignment()?;
            return Ok(Expression::Assignment {
                target: Box::new(expression),
                value: Box::new(value),
            });
        }
        Ok(expression)
    }

    fn parse_logical_or(&mut self) -> Result<Expression, String> {
        let mut expression = self.parse_logical_and()?;
        while self.match_token(&Token::OrOr) {
            let right = self.parse_logical_and()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                operator: BinaryOperator::LogicalOr,
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn parse_logical_and(&mut self) -> Result<Expression, String> {
        let mut expression = self.parse_equality()?;
        while self.match_token(&Token::AndAnd) {
            let right = self.parse_equality()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                operator: BinaryOperator::LogicalAnd,
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn parse_equality(&mut self) -> Result<Expression, String> {
        let mut expression = self.parse_in_expression()?;
        loop {
            let operator = if self.match_token(&Token::EqualEqualEqual) {
                Some(BinaryOperator::StrictEqual)
            } else if self.match_token(&Token::BangEqualEqual) {
                Some(BinaryOperator::StrictNotEqual)
            } else if self.match_token(&Token::EqualEqual) {
                Some(BinaryOperator::Equal)
            } else if self.match_token(&Token::BangEqual) {
                Some(BinaryOperator::NotEqual)
            } else {
                None
            };

            let Some(operator) = operator else {
                break;
            };
            let right = self.parse_in_expression()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                operator,
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn parse_in_expression(&mut self) -> Result<Expression, String> {
        let mut expression = self.parse_additive()?;
        while self.match_token(&Token::KeywordIn) {
            let right = self.parse_additive()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                operator: BinaryOperator::In,
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn parse_additive(&mut self) -> Result<Expression, String> {
        let mut expression = self.parse_unary()?;
        while self.match_token(&Token::Plus) {
            let right = self.parse_unary()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                operator: BinaryOperator::Add,
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn parse_unary(&mut self) -> Result<Expression, String> {
        if self.match_token(&Token::Bang) {
            return Ok(Expression::Unary {
                operator: UnaryOperator::Not,
                argument: Box::new(self.parse_unary()?),
            });
        }
        if self.match_token(&Token::KeywordTypeof) {
            return Ok(Expression::Unary {
                operator: UnaryOperator::Typeof,
                argument: Box::new(self.parse_unary()?),
            });
        }
        if self.match_token(&Token::KeywordNew) {
            let callee = self.parse_call_member()?;
            let arguments = if self.match_token(&Token::LeftParen) {
                self.parse_call_arguments()?
            } else {
                Vec::new()
            };
            return Ok(Expression::New {
                callee: Box::new(callee),
                arguments,
            });
        }
        self.parse_call_member()
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

            if self.match_token(&Token::LeftBracket) {
                let property = self.parse_expression()?;
                self.expect(&Token::RightBracket)?;
                expression = Expression::ComputedMember {
                    object: Box::new(expression),
                    property: Box::new(property),
                };
                continue;
            }

            if self.match_token(&Token::LeftParen) {
                let arguments = self.parse_call_arguments()?;
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

    fn parse_call_arguments(&mut self) -> Result<Vec<Expression>, String> {
        let mut arguments = Vec::new();
        if !self.check(&Token::RightParen) {
            loop {
                arguments.push(self.parse_expression()?);
                if !self.match_token(&Token::Comma) {
                    break;
                }
                if self.check(&Token::RightParen) {
                    break;
                }
            }
        }
        self.expect(&Token::RightParen)?;
        Ok(arguments)
    }

    fn parse_primary(&mut self) -> Result<Expression, String> {
        match self.advance() {
            Token::Identifier(name) => Ok(Expression::Identifier(name)),
            Token::String(value) => Ok(Expression::String(value)),
            Token::Number(value) => Ok(Expression::Number(value)),
            Token::KeywordTrue => Ok(Expression::Bool(true)),
            Token::KeywordFalse => Ok(Expression::Bool(false)),
            Token::KeywordNull => Ok(Expression::Null),
            Token::KeywordThis => Ok(Expression::This),
            Token::Regex { source, flags } => Ok(Expression::Regex { source, flags }),
            Token::LeftParen => {
                let expression = self.parse_expression()?;
                self.expect(&Token::RightParen)?;
                Ok(expression)
            }
            Token::LeftBracket => {
                let mut items = Vec::new();
                if !self.check(&Token::RightBracket) {
                    loop {
                        items.push(self.parse_expression()?);
                        if !self.match_token(&Token::Comma) {
                            break;
                        }
                        if self.check(&Token::RightBracket) {
                            break;
                        }
                    }
                }
                self.expect(&Token::RightBracket)?;
                Ok(Expression::Array(items))
            }
            Token::LeftBrace => {
                let mut properties = Vec::new();
                if !self.check(&Token::RightBrace) {
                    loop {
                        let key = match self.advance() {
                            Token::Identifier(name) => ObjectKey::Identifier(name),
                            Token::String(value) => ObjectKey::String(value),
                            Token::Number(value) => ObjectKey::Number(value),
                            other => {
                                return Err(format!(
                                    "Unexpected token in JS object literal: {other:?}"
                                ));
                            }
                        };
                        self.expect(&Token::Colon)?;
                        let value = self.parse_expression()?;
                        properties.push((key, value));
                        if !self.match_token(&Token::Comma) {
                            break;
                        }
                        if self.check(&Token::RightBrace) {
                            break;
                        }
                    }
                }
                self.expect(&Token::RightBrace)?;
                Ok(Expression::Object(properties))
            }
            Token::KeywordFunction => {
                let _ = if matches!(self.peek(), Token::Identifier(_)) {
                    Some(self.advance())
                } else {
                    None
                };
                let (params, body) = self.parse_function_signature_and_body()?;
                Ok(Expression::Function { params, body })
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

#[derive(Clone)]
enum Value {
    Undefined,
    Null,
    String(String),
    Bool(bool),
    Number(f64),
    Regex(RegexValue),
    Object(ObjectRef),
    Array(ArrayRef),
    Function(Rc<FunctionValue>),
    Window,
    Document,
    LiveElement(ElementId),
    DetachedElement(Element),
    DetachedText(String),
    ClassList(ElementId),
    NodeList(Vec<ElementId>),
    JQueryCollection(Vec<ElementId>),
}

#[derive(Clone)]
struct RegexValue {
    source: String,
    flags: String,
}

#[derive(Clone)]
struct FunctionValue {
    kind: FunctionKind,
    properties: ObjectRef,
}

#[derive(Clone)]
enum FunctionKind {
    User(UserFunction),
    Native(NativeFunction),
}

#[derive(Clone)]
struct UserFunction {
    params: Vec<String>,
    body: Vec<Statement>,
    env: ScopeRef,
}

#[derive(Clone, Copy)]
enum NativeFunction {
    Dollar,
    ArrayIsArray,
    RegExpConstructor,
    FunctionConstructor,
    Noop,
}

struct Scope {
    bindings: HashMap<String, Value>,
    parent: Option<ScopeRef>,
}

impl Scope {
    fn new(parent: Option<ScopeRef>) -> ScopeRef {
        Rc::new(RefCell::new(Self {
            bindings: HashMap::new(),
            parent,
        }))
    }
}

struct Executor<'a> {
    document: &'a mut Document,
    global: ScopeRef,
    scope: ScopeRef,
}

enum ControlFlow {
    Continue(Value),
    Return(Value),
}

impl<'a> Executor<'a> {
    fn new(document: &'a mut Document) -> Self {
        let global = Scope::new(None);
        let mut executor = Self {
            document,
            global: global.clone(),
            scope: global,
        };
        executor.declare_global("this", Value::Window);
        executor.declare_global("window", Value::Window);
        executor.declare_global("document", Value::Document);
        executor.declare_global("$", executor.create_native_function(NativeFunction::Dollar));
        executor.declare_global(
            "RegExp",
            executor.create_native_function(NativeFunction::RegExpConstructor),
        );
        executor.declare_global(
            "Function",
            executor.create_native_function(NativeFunction::FunctionConstructor),
        );

        let array_ctor = executor.create_native_function(NativeFunction::Noop);
        let _ = executor.set_property_value(
            array_ctor.clone(),
            "isArray",
            executor.create_native_function(NativeFunction::ArrayIsArray),
        );
        executor.declare_global("Array", array_ctor);

        let promise_ctor = executor.create_native_function(NativeFunction::Noop);
        let promise_proto = executor.create_plain_object();
        let _ = executor.set_property_value(
            promise_proto.clone(),
            "finally",
            executor.create_native_function(NativeFunction::Noop),
        );
        let _ = executor.set_property_value(promise_ctor.clone(), "prototype", promise_proto);
        executor.declare_global("Promise", promise_ctor);

        executor
    }

    fn execute_program(&mut self, statements: &[Statement]) -> Result<(), String> {
        match self.execute_statements(statements)? {
            ControlFlow::Continue(_) | ControlFlow::Return(_) => Ok(()),
        }
    }

    fn execute_statements(&mut self, statements: &[Statement]) -> Result<ControlFlow, String> {
        let mut last = Value::Undefined;
        for statement in statements {
            match self.execute_statement(statement)? {
                ControlFlow::Continue(value) => last = value,
                ControlFlow::Return(value) => return Ok(ControlFlow::Return(value)),
            }
        }
        Ok(ControlFlow::Continue(last))
    }

    fn execute_statement(&mut self, statement: &Statement) -> Result<ControlFlow, String> {
        match statement {
            Statement::VariableDeclaration { name, init } => {
                let value = if let Some(init) = init {
                    self.evaluate_expression(init)?
                } else {
                    Value::Undefined
                };
                self.declare_binding(name, value.clone());
                Ok(ControlFlow::Continue(value))
            }
            Statement::FunctionDeclaration { name, params, body } => {
                let function = self.create_user_function(params.clone(), body.clone());
                self.declare_binding(name, function.clone());
                Ok(ControlFlow::Continue(function))
            }
            Statement::Return(expression) => {
                let value = if let Some(expression) = expression {
                    self.evaluate_expression(expression)?
                } else {
                    Value::Undefined
                };
                Ok(ControlFlow::Return(value))
            }
            Statement::If {
                test,
                consequent,
                alternate,
            } => {
                if self.evaluate_expression(test)?.is_truthy() {
                    self.execute_statement(consequent)
                } else if let Some(alternate) = alternate {
                    self.execute_statement(alternate)
                } else {
                    Ok(ControlFlow::Continue(Value::Undefined))
                }
            }
            Statement::Block(statements) => self.execute_statements(statements),
            Statement::TryCatch {
                try_block,
                catch_param,
                catch_block,
            } => match self.execute_statements(try_block) {
                Ok(result) => Ok(result),
                Err(err) => {
                    let previous_scope = self.scope.clone();
                    let catch_scope = Scope::new(Some(previous_scope.clone()));
                    catch_scope
                        .borrow_mut()
                        .bindings
                        .insert(catch_param.clone(), Value::String(err));
                    self.scope = catch_scope;
                    let result = self.execute_statements(catch_block);
                    self.scope = previous_scope;
                    result
                }
            },
            Statement::Expression(expression) => {
                Ok(ControlFlow::Continue(self.evaluate_expression(expression)?))
            }
        }
    }

    fn evaluate_expression(&mut self, expression: &Expression) -> Result<Value, String> {
        match expression {
            Expression::Identifier(name) => Ok(self.lookup_binding(name).unwrap_or(Value::Undefined)),
            Expression::This => Ok(self.lookup_binding("this").unwrap_or(Value::Window)),
            Expression::String(value) => Ok(Value::String(value.clone())),
            Expression::Number(value) => Ok(Value::Number(*value)),
            Expression::Bool(value) => Ok(Value::Bool(*value)),
            Expression::Null => Ok(Value::Null),
            Expression::Regex { source, flags } => Ok(Value::Regex(RegexValue {
                source: source.clone(),
                flags: flags.clone(),
            })),
            Expression::Array(items) => {
                let mut values = Vec::with_capacity(items.len());
                for item in items {
                    values.push(self.evaluate_expression(item)?);
                }
                Ok(Value::Array(Rc::new(RefCell::new(values))))
            }
            Expression::Object(properties) => {
                let object = self.create_plain_object();
                for (key, value) in properties {
                    let key = object_key_to_string(key);
                    let value = self.evaluate_expression(value)?;
                    self.set_property_value(object.clone(), key.as_str(), value)?;
                }
                Ok(object)
            }
            Expression::Function { params, body } => {
                Ok(self.create_user_function(params.clone(), body.clone()))
            }
            Expression::Member { object, property } => {
                let object = self.evaluate_expression(object)?;
                self.get_member_value(object, property)
            }
            Expression::ComputedMember { object, property } => {
                let object = self.evaluate_expression(object)?;
                let property = self.evaluate_expression(property)?.to_property_key();
                self.get_member_value(object, property.as_str())
            }
            Expression::Call { callee, arguments } => self.evaluate_call(callee, arguments),
            Expression::Assignment { target, value } => {
                let value = self.evaluate_expression(value)?;
                self.assign_target(target, value.clone())?;
                Ok(value)
            }
            Expression::Unary { operator, argument } => self.evaluate_unary(*operator, argument),
            Expression::Binary {
                left,
                operator,
                right,
            } => self.evaluate_binary(left, *operator, right),
            Expression::New { callee, arguments } => {
                let callee = self.evaluate_expression(callee)?;
                let arguments = arguments
                    .iter()
                    .map(|argument| self.evaluate_expression(argument))
                    .collect::<Result<Vec<_>, _>>()?;
                self.construct_value(callee, &arguments)
            }
        }
    }

    fn evaluate_unary(
        &mut self,
        operator: UnaryOperator,
        argument: &Expression,
    ) -> Result<Value, String> {
        match operator {
            UnaryOperator::Not => Ok(Value::Bool(!self.evaluate_expression(argument)?.is_truthy())),
            UnaryOperator::Typeof => Ok(Value::String(self.evaluate_expression(argument)?.type_name())),
        }
    }

    fn evaluate_binary(
        &mut self,
        left: &Expression,
        operator: BinaryOperator,
        right: &Expression,
    ) -> Result<Value, String> {
        match operator {
            BinaryOperator::LogicalAnd => {
                let left = self.evaluate_expression(left)?;
                if !left.is_truthy() {
                    return Ok(left);
                }
                self.evaluate_expression(right)
            }
            BinaryOperator::LogicalOr => {
                let left = self.evaluate_expression(left)?;
                if left.is_truthy() {
                    return Ok(left);
                }
                self.evaluate_expression(right)
            }
            BinaryOperator::Add => {
                let left = self.evaluate_expression(left)?;
                let right = self.evaluate_expression(right)?;
                if matches!(left, Value::String(_)) || matches!(right, Value::String(_)) {
                    return Ok(Value::String(format!(
                        "{}{}",
                        left.to_string_value(),
                        right.to_string_value()
                    )));
                }
                Ok(Value::Number(left.to_number_value() + right.to_number_value()))
            }
            BinaryOperator::In => {
                let property = self.evaluate_expression(left)?.to_property_key();
                let object = self.evaluate_expression(right)?;
                Ok(Value::Bool(self.has_property(object, property.as_str())))
            }
            BinaryOperator::Equal => {
                let left = self.evaluate_expression(left)?;
                let right = self.evaluate_expression(right)?;
                Ok(Value::Bool(loose_equals(&left, &right)))
            }
            BinaryOperator::NotEqual => {
                let left = self.evaluate_expression(left)?;
                let right = self.evaluate_expression(right)?;
                Ok(Value::Bool(!loose_equals(&left, &right)))
            }
            BinaryOperator::StrictEqual => {
                let left = self.evaluate_expression(left)?;
                let right = self.evaluate_expression(right)?;
                Ok(Value::Bool(strict_equals(&left, &right)))
            }
            BinaryOperator::StrictNotEqual => {
                let left = self.evaluate_expression(left)?;
                let right = self.evaluate_expression(right)?;
                Ok(Value::Bool(!strict_equals(&left, &right)))
            }
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

        match callee {
            Expression::Member { object, property } => {
                let receiver = self.evaluate_expression(object)?;
                self.call_member(receiver, property, &evaluated_arguments)
            }
            Expression::ComputedMember { object, property } => {
                let receiver = self.evaluate_expression(object)?;
                let property = self.evaluate_expression(property)?.to_property_key();
                self.call_member(receiver, property.as_str(), &evaluated_arguments)
            }
            _ => {
                let callee = self.evaluate_expression(callee)?;
                self.call_value(callee, Value::Window, &evaluated_arguments)
            }
        }
    }

    fn assign_target(&mut self, target: &Expression, value: Value) -> Result<(), String> {
        match target {
            Expression::Identifier(name) => {
                self.assign_binding(name, value);
                Ok(())
            }
            Expression::Member { object, property } => {
                if self.assign_identifier_backed_member(object, property, &value) {
                    return Ok(());
                }
                let object = self.evaluate_expression(object)?;
                self.set_member_value(object, property, value)
            }
            Expression::ComputedMember { object, property } => {
                let property_name = self.evaluate_expression(property)?.to_property_key();
                if self.assign_identifier_backed_member(object, property_name.as_str(), &value) {
                    return Ok(());
                }
                let object = self.evaluate_expression(object)?;
                self.set_member_value(object, property_name.as_str(), value)
            }
            _ => Err("Unsupported assignment target".to_owned()),
        }
    }

    fn assign_identifier_backed_member(
        &mut self,
        object: &Expression,
        property: &str,
        value: &Value,
    ) -> bool {
        let Expression::Identifier(name) = object else {
            return false;
        };

        let Some(existing) = self.lookup_binding(name) else {
            return false;
        };
        let Value::DetachedElement(_) = existing else {
            return false;
        };

        if let Some(scope) = self.find_scope_containing(name)
            && let Some(bound) = scope.borrow_mut().bindings.get_mut(name)
        {
            return set_detached_member_value(bound, property, value);
        }

        false
    }

    fn call_member(
        &mut self,
        receiver: Value,
        property: &str,
        arguments: &[Value],
    ) -> Result<Value, String> {
        match receiver.clone() {
            Value::Document => self.call_document_method(property, arguments),
            Value::LiveElement(node_id) => self.call_element_method(node_id, property, arguments),
            Value::ClassList(node_id) => self.call_class_list_method(node_id, property, arguments),
            Value::String(text) => self.call_string_method(text.as_str(), property, arguments),
            Value::Array(items) => self.call_array_method(items, property, arguments),
            Value::JQueryCollection(ids) => self.call_jquery_method(ids, property, arguments),
            _ => {
                let callee = self.get_member_value(receiver.clone(), property)?;
                self.call_value(callee, receiver, arguments)
            }
        }
    }

    fn call_value(
        &mut self,
        callee: Value,
        this_value: Value,
        arguments: &[Value],
    ) -> Result<Value, String> {
        let Value::Function(function) = callee else {
            return Err("Unsupported JS call target".to_owned());
        };

        match function.kind.clone() {
            FunctionKind::Native(kind) => self.call_native_function(kind, this_value, arguments),
            FunctionKind::User(function) => {
                let previous_scope = self.scope.clone();
                let next_scope = Scope::new(Some(function.env.clone()));
                {
                    let mut bindings = next_scope.borrow_mut();
                    bindings.bindings.insert("this".to_owned(), this_value);
                    for (index, param) in function.params.iter().enumerate() {
                        bindings.bindings.insert(
                            param.clone(),
                            arguments.get(index).cloned().unwrap_or(Value::Undefined),
                        );
                    }
                }
                self.scope = next_scope;
                let result = match self.execute_statements(&function.body)? {
                    ControlFlow::Continue(_) => Value::Undefined,
                    ControlFlow::Return(value) => value,
                };
                self.scope = previous_scope;
                Ok(result)
            }
        }
    }

    fn construct_value(&mut self, callee: Value, arguments: &[Value]) -> Result<Value, String> {
        let Value::Function(function) = callee else {
            return Err("Unsupported constructor".to_owned());
        };

        match function.kind.clone() {
            FunctionKind::Native(kind) => self.call_native_function(kind, Value::Undefined, arguments),
            FunctionKind::User(_) => {
                let instance = self.create_plain_object();
                let value = self.call_value(Value::Function(function), instance.clone(), arguments)?;
                Ok(match value {
                    Value::Undefined => instance,
                    other => other,
                })
            }
        }
    }

    fn call_native_function(
        &mut self,
        kind: NativeFunction,
        _this_value: Value,
        arguments: &[Value],
    ) -> Result<Value, String> {
        match kind {
            NativeFunction::Dollar => {
                let selector = first_string_argument(arguments)?;
                Ok(Value::JQueryCollection(
                    self.document.query_selector_all_ids(selector.as_str()),
                ))
            }
            NativeFunction::ArrayIsArray => {
                Ok(Value::Bool(matches!(arguments.first(), Some(Value::Array(_)))))
            }
            NativeFunction::RegExpConstructor => {
                let source = arguments
                    .first()
                    .map(regex_source)
                    .unwrap_or_default();
                let flags = arguments
                    .get(1)
                    .map(Value::to_string_value)
                    .unwrap_or_default();
                Ok(Value::Regex(RegexValue { source, flags }))
            }
            NativeFunction::FunctionConstructor => {
                Ok(self.create_native_function(NativeFunction::Noop))
            }
            NativeFunction::Noop => Ok(Value::Undefined),
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

    fn call_string_method(
        &mut self,
        text: &str,
        property: &str,
        arguments: &[Value],
    ) -> Result<Value, String> {
        match property {
            "split" => {
                let separator = first_string_argument(arguments)?;
                let parts = if separator.is_empty() {
                    text.chars().map(|ch| Value::String(ch.to_string())).collect()
                } else {
                    text.split(separator.as_str())
                        .map(|part| Value::String(part.to_owned()))
                        .collect()
                };
                Ok(Value::Array(Rc::new(RefCell::new(parts))))
            }
            "match" => {
                let pattern = arguments.first().cloned().unwrap_or(Value::Undefined);
                Ok(simple_match(text, pattern))
            }
            "replace" => {
                let pattern = arguments.first().cloned().unwrap_or(Value::Undefined);
                let replacement = arguments
                    .get(1)
                    .cloned()
                    .unwrap_or(Value::String(String::new()))
                    .to_string_value();
                Ok(Value::String(simple_replace(text, pattern, replacement.as_str())))
            }
            _ => Err(format!("Unsupported string method: {property}")),
        }
    }

    fn call_array_method(
        &mut self,
        items: ArrayRef,
        property: &str,
        arguments: &[Value],
    ) -> Result<Value, String> {
        match property {
            "forEach" => {
                let callback = arguments.first().cloned().unwrap_or(Value::Undefined);
                let snapshot = items.borrow().clone();
                for (index, item) in snapshot.into_iter().enumerate() {
                    let array_value = Value::Array(items.clone());
                    let args = [item, Value::Number(index as f64), array_value];
                    let _ = self.call_value(callback.clone(), Value::Undefined, &args)?;
                }
                Ok(Value::Undefined)
            }
            _ => Err(format!("Unsupported array method: {property}")),
        }
    }

    fn call_jquery_method(
        &mut self,
        ids: Vec<ElementId>,
        property: &str,
        arguments: &[Value],
    ) -> Result<Value, String> {
        match property {
            "addClass" => {
                let tokens = collect_class_tokens(arguments);
                for id in ids {
                    if let Some(element) = self.document.find_element_by_node_id_mut(id) {
                        for token in &tokens {
                            if !element.attributes.has_class(token.as_str()) {
                                element.attributes.classes.push(token.clone());
                            }
                        }
                    }
                }
                Ok(Value::Undefined)
            }
            "removeClass" => {
                let tokens = collect_class_tokens(arguments);
                for id in ids {
                    if let Some(element) = self.document.find_element_by_node_id_mut(id) {
                        element
                            .attributes
                            .classes
                            .retain(|existing| !tokens.iter().any(|token| token == existing));
                    }
                }
                Ok(Value::Undefined)
            }
            _ => Err(format!("Unsupported jQuery method: {property}")),
        }
    }

    fn get_member_value(&mut self, object: Value, property: &str) -> Result<Value, String> {
        match object {
            Value::Window => match property {
                "document" => Ok(Value::Document),
                "window" => Ok(Value::Window),
                "localStorage" => Ok(self.create_plain_object()),
                _ => Ok(self.lookup_binding(property).unwrap_or(Value::Undefined)),
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
                "head" => Ok(self
                    .document
                    .find_first_element_by_name("head")
                    .map(|element| Value::LiveElement(element.node_id))
                    .unwrap_or(Value::Undefined)),
                "cookie" => Ok(Value::String(String::new())),
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
            Value::NodeList(ids) => {
                if property == "length" {
                    return Ok(Value::Number(ids.len() as f64));
                }
                if let Some(index) = parse_array_index(property) {
                    return Ok(ids
                        .get(index)
                        .copied()
                        .map(Value::LiveElement)
                        .unwrap_or(Value::Undefined));
                }
                Ok(Value::Undefined)
            }
            Value::JQueryCollection(ids) => {
                if property == "length" {
                    return Ok(Value::Number(ids.len() as f64));
                }
                if let Some(index) = parse_array_index(property) {
                    return Ok(ids
                        .get(index)
                        .copied()
                        .map(Value::LiveElement)
                        .unwrap_or(Value::Undefined));
                }
                Ok(Value::Undefined)
            }
            Value::Array(items) => {
                if property == "length" {
                    return Ok(Value::Number(items.borrow().len() as f64));
                }
                if let Some(index) = parse_array_index(property) {
                    return Ok(items
                        .borrow()
                        .get(index)
                        .cloned()
                        .unwrap_or(Value::Undefined));
                }
                Ok(Value::Undefined)
            }
            Value::Object(properties) => {
                Ok(properties
                    .borrow()
                    .get(property)
                    .cloned()
                    .unwrap_or(Value::Undefined))
            }
            Value::Function(function) => {
                Ok(function
                    .properties
                    .borrow()
                    .get(property)
                    .cloned()
                    .unwrap_or(Value::Undefined))
            }
            Value::String(text) => {
                if property == "length" {
                    return Ok(Value::Number(text.chars().count() as f64));
                }
                if let Some(index) = parse_array_index(property) {
                    return Ok(text
                        .chars()
                        .nth(index)
                        .map(|ch| Value::String(ch.to_string()))
                        .unwrap_or(Value::Undefined));
                }
                Ok(Value::Undefined)
            }
            Value::Regex(regex) => match property {
                "source" => Ok(Value::String(regex.source)),
                "flags" => Ok(Value::String(regex.flags)),
                _ => Ok(Value::Undefined),
            },
            _ => Ok(Value::Undefined),
        }
    }

    fn has_property(&mut self, object: Value, property: &str) -> bool {
        match object {
            Value::Window => matches!(property, "document" | "window" | "localStorage")
                || self.lookup_binding(property).is_some(),
            Value::Document => matches!(
                property,
                "documentElement"
                    | "body"
                    | "head"
                    | "cookie"
                    | "getElementById"
                    | "querySelector"
                    | "querySelectorAll"
                    | "createElement"
                    | "createTextNode"
            ),
            Value::Array(items) => property == "length"
                || parse_array_index(property).is_some_and(|index| index < items.borrow().len()),
            Value::NodeList(ids) | Value::JQueryCollection(ids) => {
                property == "length"
                    || parse_array_index(property).is_some_and(|index| index < ids.len())
            }
            Value::Object(properties) => properties.borrow().contains_key(property),
            Value::Function(function) => function.properties.borrow().contains_key(property),
            Value::String(text) => {
                property == "length"
                    || parse_array_index(property).is_some_and(|index| index < text.chars().count())
            }
            Value::Regex(regex) => matches!(property, "source" | "flags")
                || !regex.source.is_empty() && property == "constructor",
            Value::LiveElement(_) => matches!(
                property,
                "classList" | "className" | "textContent" | "id" | "checked" | "disabled" | "value"
            ),
            _ => false,
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
            Value::Object(properties) => {
                properties.borrow_mut().insert(property.to_owned(), value);
                Ok(())
            }
            Value::Function(function) => {
                function.properties.borrow_mut().insert(property.to_owned(), value);
                Ok(())
            }
            Value::Array(items) => {
                if let Some(index) = parse_array_index(property) {
                    let mut items = items.borrow_mut();
                    if index >= items.len() {
                        items.resize(index + 1, Value::Undefined);
                    }
                    items[index] = value;
                    return Ok(());
                }
                Err(format!("Unsupported array property assignment: {property}"))
            }
            _ => Err(format!("Unsupported assignment through property {property}")),
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
                let _ = match position {
                    InsertPosition::Append => self.document.append_child_to(parent_id, Node::Text(text)),
                    InsertPosition::Prepend => self.document.prepend_child_to(parent_id, Node::Text(text)),
                };
                Ok(Value::Undefined)
            }
            Value::LiveElement(existing_id) => {
                let Some(existing) = self.document.find_element_by_node_id(existing_id).cloned() else {
                    return Ok(Value::Undefined);
                };
                let inserted = match position {
                    InsertPosition::Append => {
                        self.document.append_child_to(parent_id, Node::Element(existing))
                    }
                    InsertPosition::Prepend => {
                        self.document.prepend_child_to(parent_id, Node::Element(existing))
                    }
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

    fn declare_global(&mut self, name: &str, value: Value) {
        self.global.borrow_mut().bindings.insert(name.to_owned(), value);
    }

    fn declare_binding(&mut self, name: &str, value: Value) {
        self.scope.borrow_mut().bindings.insert(name.to_owned(), value);
    }

    fn lookup_binding(&self, name: &str) -> Option<Value> {
        lookup_scope(&self.scope, name)
    }

    fn assign_binding(&mut self, name: &str, value: Value) {
        if let Some(scope) = self.find_scope_containing(name) {
            scope.borrow_mut().bindings.insert(name.to_owned(), value);
            return;
        }
        self.global.borrow_mut().bindings.insert(name.to_owned(), value);
    }

    fn find_scope_containing(&self, name: &str) -> Option<ScopeRef> {
        find_scope_containing(&self.scope, name)
    }

    fn create_plain_object(&self) -> Value {
        Value::Object(Rc::new(RefCell::new(HashMap::new())))
    }

    fn create_native_function(&self, kind: NativeFunction) -> Value {
        Value::Function(Rc::new(FunctionValue {
            kind: FunctionKind::Native(kind),
            properties: Rc::new(RefCell::new(HashMap::new())),
        }))
    }

    fn create_user_function(&self, params: Vec<String>, body: Vec<Statement>) -> Value {
        Value::Function(Rc::new(FunctionValue {
            kind: FunctionKind::User(UserFunction {
                params,
                body,
                env: self.scope.clone(),
            }),
            properties: Rc::new(RefCell::new(HashMap::new())),
        }))
    }

    fn set_property_value(
        &self,
        target: Value,
        property: &str,
        value: Value,
    ) -> Result<(), String> {
        match target {
            Value::Object(properties) => {
                properties.borrow_mut().insert(property.to_owned(), value);
                Ok(())
            }
            Value::Function(function) => {
                function.properties.borrow_mut().insert(property.to_owned(), value);
                Ok(())
            }
            _ => Err(format!("Unsupported property target: {property}")),
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
            Value::Null => "null".to_owned(),
            Value::String(value) => value.clone(),
            Value::Bool(value) => {
                if *value {
                    "true".to_owned()
                } else {
                    "false".to_owned()
                }
            }
            Value::Number(value) => number_to_string(*value),
            Value::DetachedText(value) => value.clone(),
            Value::Regex(regex) => format!("/{}/{}", regex.source, regex.flags),
            _ => String::new(),
        }
    }

    fn to_number_value(&self) -> f64 {
        match self {
            Value::Number(value) => *value,
            Value::Bool(true) => 1.0,
            Value::Bool(false) => 0.0,
            Value::String(value) => value.parse::<f64>().unwrap_or(0.0),
            Value::Null => 0.0,
            Value::Undefined => 0.0,
            _ => 0.0,
        }
    }

    fn to_property_key(&self) -> String {
        match self {
            Value::Number(value) => number_to_string(*value),
            Value::String(value) => value.clone(),
            Value::Bool(value) => {
                if *value {
                    "true".to_owned()
                } else {
                    "false".to_owned()
                }
            }
            Value::Null => "null".to_owned(),
            Value::Undefined => "undefined".to_owned(),
            _ => self.to_string_value(),
        }
    }

    fn is_truthy(&self) -> bool {
        match self {
            Value::Undefined | Value::Null => false,
            Value::String(value) => !value.is_empty(),
            Value::Bool(value) => *value,
            Value::Number(value) => *value != 0.0,
            _ => true,
        }
    }

    fn type_name(&self) -> String {
        match self {
            Value::Undefined => "undefined".to_owned(),
            Value::Null => "object".to_owned(),
            Value::String(_) => "string".to_owned(),
            Value::Bool(_) => "boolean".to_owned(),
            Value::Number(_) => "number".to_owned(),
            Value::Function(_) => "function".to_owned(),
            _ => "object".to_owned(),
        }
    }
}

fn lookup_scope(scope: &ScopeRef, name: &str) -> Option<Value> {
    if let Some(value) = scope.borrow().bindings.get(name) {
        return Some(value.clone());
    }
    let parent = scope.borrow().parent.clone();
    parent.and_then(|parent| lookup_scope(&parent, name))
}

fn find_scope_containing(scope: &ScopeRef, name: &str) -> Option<ScopeRef> {
    if scope.borrow().bindings.contains_key(name) {
        return Some(scope.clone());
    }
    let parent = scope.borrow().parent.clone();
    parent.and_then(|parent| find_scope_containing(&parent, name))
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

fn object_key_to_string(key: &ObjectKey) -> String {
    match key {
        ObjectKey::Identifier(value) | ObjectKey::String(value) => value.clone(),
        ObjectKey::Number(value) => number_to_string(*value),
    }
}

fn parse_array_index(property: &str) -> Option<usize> {
    if property.is_empty() || !property.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    property.parse::<usize>().ok()
}

fn number_to_string(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
}

fn strict_equals(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Undefined, Value::Undefined) | (Value::Null, Value::Null) => true,
        (Value::String(left), Value::String(right)) => left == right,
        (Value::Bool(left), Value::Bool(right)) => left == right,
        (Value::Number(left), Value::Number(right)) => left == right,
        (Value::Window, Value::Window) | (Value::Document, Value::Document) => true,
        (Value::LiveElement(left), Value::LiveElement(right)) => left == right,
        (Value::ClassList(left), Value::ClassList(right)) => left == right,
        (Value::Regex(left), Value::Regex(right)) => {
            left.source == right.source && left.flags == right.flags
        }
        (Value::Array(left), Value::Array(right)) => Rc::ptr_eq(left, right),
        (Value::Object(left), Value::Object(right)) => Rc::ptr_eq(left, right),
        (Value::Function(left), Value::Function(right)) => Rc::ptr_eq(left, right),
        (Value::JQueryCollection(left), Value::JQueryCollection(right)) => left == right,
        (Value::NodeList(left), Value::NodeList(right)) => left == right,
        _ => false,
    }
}

fn loose_equals(left: &Value, right: &Value) -> bool {
    if matches!((left, right), (Value::Null, Value::Undefined) | (Value::Undefined, Value::Null)) {
        return true;
    }
    strict_equals(left, right)
}

fn regex_source(value: &Value) -> String {
    match value {
        Value::Regex(regex) => regex.source.clone(),
        _ => value.to_string_value(),
    }
}

fn simple_match(text: &str, pattern: Value) -> Value {
    if text.is_empty() {
        return Value::Null;
    }
    match pattern {
        Value::String(needle) => {
            if text.contains(needle.as_str()) {
                Value::Array(Rc::new(RefCell::new(vec![Value::String(needle)])))
            } else {
                Value::Null
            }
        }
        Value::Regex(regex) => {
            if regex.source == "." {
                let value = text
                    .chars()
                    .next()
                    .map(|ch| Value::String(ch.to_string()))
                    .unwrap_or(Value::Undefined);
                return Value::Array(Rc::new(RefCell::new(vec![value])));
            }
            if !regex
                .source
                .chars()
                .any(|ch| matches!(ch, '[' | ']' | '(' | ')' | '?' | '+' | '*' | '|' | '^' | '$' | '\\'))
                && text.contains(regex.source.as_str())
            {
                return Value::Array(Rc::new(RefCell::new(vec![Value::String(regex.source)])));
            }
            Value::Null
        }
        _ => Value::Null,
    }
}

fn simple_replace(text: &str, pattern: Value, replacement: &str) -> String {
    match pattern {
        Value::String(needle) => text.replacen(needle.as_str(), replacement, 1),
        Value::Regex(regex) => {
            if regex.source == "." {
                if let Some((index, ch)) = text.char_indices().next() {
                    let end = index + ch.len_utf8();
                    let mut out = String::new();
                    out.push_str(replacement);
                    out.push_str(&text[end..]);
                    return out;
                }
            }
            text.to_owned()
        }
        _ => text.to_owned(),
    }
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch == '$' || ch.is_ascii_alphabetic()
}

fn is_identifier_continue(ch: char) -> bool {
    is_identifier_start(ch) || ch.is_ascii_digit()
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
            $('html').addClass('ve-available');
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
        assert!(html.attributes.has_class("ve-available"));
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

    #[test]
    fn supports_functions_arrays_objects_and_conditionals() {
        let mut document = crate::html::parse_document(r#"<div id="greeting">Welcome</div>"#);

        execute(
            &mut document,
            r#"
            (function() {
                var key = "text";
                var payload = { text: "Hello from JS", enabled: true, missing: null };
                var values = [payload[key]];
                if (payload.enabled && payload.missing === null) {
                    document.getElementById("greeting").textContent = values[0];
                }
            }());
            "#,
        )
        .expect("script should execute");

        let greeting = document
            .find_first_element_by_id("greeting")
            .expect("missing greeting");
        assert_eq!(greeting.children, vec![Node::Text("Hello from JS".to_owned())]);
    }

    #[test]
    fn executes_wikipedia_client_bootstrap_without_cookie() {
        let mut document = crate::html::parse_document(
            r#"<html class="client-nojs"><body></body></html>"#,
        );

        execute(
            &mut document,
            r#"
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
            "#,
        )
        .expect("script should execute");

        let html = document
            .find_first_element_by_name("html")
            .expect("missing html");
        assert!(html.attributes.has_class("client-js"));
        assert!(html.attributes.has_class("skin-vector"));
        assert!(!html.attributes.has_class("client-nojs"));
    }
}
