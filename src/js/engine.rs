use crate::dom::{Attributes, Document, Element, ElementId, Node};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

type ScopeRef = Rc<RefCell<Scope>>;
type ObjectRef = Rc<RefCell<HashMap<String, Value>>>;
type ArrayRef = Rc<RefCell<ArrayValue>>;
type SetRef = Rc<RefCell<Vec<Value>>>;

#[derive(Clone, Default)]
struct ArrayValue {
    items: Vec<Value>,
    properties: HashMap<String, Value>,
}

pub(crate) struct Runtime {
    global: ScopeRef,
}

#[cfg(test)]
pub fn execute(document: &mut Document, source: &str) -> Result<(), String> {
    let mut runtime = Runtime::new(document);
    runtime.execute(document, source)
}

impl Runtime {
    pub(crate) fn new(document: &mut Document) -> Self {
        let executor = Executor::new(document);
        Self {
            global: executor.global.clone(),
        }
    }

    pub(crate) fn execute(&mut self, document: &mut Document, source: &str) -> Result<(), String> {
        let tokens = Lexer::new(source).tokenize()?;
        let program = Parser::new(tokens).parse_program()?;
        Executor::with_global(document, self.global.clone()).execute_program(&program)
    }
}

#[derive(Clone, Debug)]
enum Statement {
    VariableDeclaration {
        declarations: Vec<VariableDeclarator>,
    },
    FunctionDeclaration {
        name: String,
        params: Vec<Parameter>,
        body: Vec<Statement>,
    },
    Return(Option<Expression>),
    Throw(Expression),
    Break,
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
    While {
        test: Expression,
        body: Box<Statement>,
    },
    Switch {
        discriminant: Expression,
        cases: Vec<SwitchCase>,
    },
    For {
        init: Option<ForInit>,
        test: Option<Expression>,
        update: Option<Expression>,
        body: Box<Statement>,
    },
    ForEach {
        binding: ForEachBinding,
        operator: ForEachOperator,
        iterable: Expression,
        body: Box<Statement>,
    },
    Expression(Expression),
}

#[derive(Clone, Debug)]
struct VariableDeclarator {
    pattern: BindingPattern,
    init: Option<Expression>,
}

#[derive(Clone, Debug)]
enum BindingPattern {
    Identifier(String),
    Object(Vec<ObjectBindingProperty>),
}

#[derive(Clone, Debug)]
struct ObjectBindingProperty {
    key: String,
    binding: String,
}

#[derive(Clone, Debug)]
struct Parameter {
    name: String,
    default: Option<Expression>,
}

#[derive(Clone, Debug)]
struct SwitchCase {
    test: Option<Expression>,
    consequent: Vec<Statement>,
}

#[derive(Clone, Debug)]
enum ForInit {
    VariableDeclaration(Vec<VariableDeclarator>),
    Expression(Expression),
}

#[derive(Clone, Debug)]
enum ForEachBinding {
    Declaration(VariableDeclarator),
    Target(Expression),
}

#[derive(Clone, Copy, Debug)]
enum ForEachOperator {
    In,
    Of,
}

#[derive(Clone, Debug)]
enum ObjectKey {
    Identifier(String),
    String(String),
    Number(f64),
}

#[derive(Clone, Debug)]
enum ObjectProperty {
    KeyValue(ObjectKey, Expression),
    Computed(Expression, Expression),
    Shorthand(String),
}

#[derive(Clone, Debug)]
enum ArrayElement {
    Expression(Expression),
    Spread(Expression),
}

#[derive(Clone, Debug)]
enum TemplateSegment {
    String(String),
    Expression(Expression),
}

#[derive(Clone, Copy, Debug)]
enum UnaryOperator {
    Not,
    Plus,
    Minus,
    Typeof,
    Delete,
}

#[derive(Clone, Copy, Debug)]
enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    LeftShift,
    RightShift,
    UnsignedRightShift,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    In,
    Instanceof,
    Equal,
    NotEqual,
    StrictEqual,
    StrictNotEqual,
    BitXor,
    LogicalAnd,
    LogicalOr,
}

#[derive(Clone, Copy, Debug)]
enum UpdateOperator {
    Increment,
    Decrement,
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
    TemplateLiteral(Vec<TemplateSegment>),
    Array(Vec<ArrayElement>),
    Object(Vec<ObjectProperty>),
    Function {
        params: Vec<Parameter>,
        body: Vec<Statement>,
        lexical_this: bool,
        constructible: bool,
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
    Update {
        target: Box<Expression>,
        operator: UpdateOperator,
        prefix: bool,
    },
    Unary {
        operator: UnaryOperator,
        argument: Box<Expression>,
    },
    Conditional {
        test: Box<Expression>,
        consequent: Box<Expression>,
        alternate: Box<Expression>,
    },
    Sequence(Vec<Expression>),
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
    TemplateLiteral(TemplateLiteralToken),
    KeywordVar,
    KeywordLet,
    KeywordConst,
    KeywordFunction,
    KeywordReturn,
    KeywordThrow,
    KeywordBreak,
    KeywordSwitch,
    KeywordCase,
    KeywordDefault,
    KeywordIf,
    KeywordElse,
    KeywordTrue,
    KeywordFalse,
    KeywordNull,
    KeywordThis,
    KeywordNew,
    KeywordTypeof,
    KeywordDelete,
    KeywordTry,
    KeywordCatch,
    KeywordWhile,
    KeywordFor,
    KeywordIn,
    KeywordOf,
    KeywordInstanceof,
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
    Question,
    Arrow,
    Equal,
    EqualEqual,
    EqualEqualEqual,
    Bang,
    BangEqual,
    BangEqualEqual,
    Plus,
    PlusPlus,
    PlusEqual,
    Minus,
    MinusMinus,
    MinusEqual,
    Star,
    Slash,
    Percent,
    Ellipsis,
    LessLess,
    Less,
    LessEqual,
    GreaterGreaterGreater,
    GreaterGreater,
    Greater,
    GreaterEqual,
    Caret,
    CaretEqual,
    AndAnd,
    OrOr,
    Eof,
}

#[derive(Clone, Debug, PartialEq)]
struct TemplateLiteralToken {
    parts: Vec<TemplateLiteralPartToken>,
}

#[derive(Clone, Debug, PartialEq)]
enum TemplateLiteralPartToken {
    String(String),
    Expression(String),
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
                '/' if should_parse_regex(out.last()) => out.push(self.consume_regex_literal()?),
                '/' => {
                    self.advance_char();
                    out.push(Token::Slash);
                }
                '*' => {
                    self.advance_char();
                    out.push(Token::Star);
                }
                '%' => {
                    self.advance_char();
                    out.push(Token::Percent);
                }
                '`' => out.push(Token::TemplateLiteral(self.consume_template_literal()?)),
                '.' if self.starts_with("...") => {
                    self.cursor += 3;
                    out.push(Token::Ellipsis);
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
                '?' => {
                    self.advance_char();
                    out.push(Token::Question);
                }
                '=' if self.starts_with("=>") => {
                    self.cursor += 2;
                    out.push(Token::Arrow);
                }
                '+' if self.starts_with("++") => {
                    self.cursor += 2;
                    out.push(Token::PlusPlus);
                }
                '+' if self.starts_with("+=") => {
                    self.cursor += 2;
                    out.push(Token::PlusEqual);
                }
                '+' => {
                    self.advance_char();
                    out.push(Token::Plus);
                }
                '-' if self.starts_with("--") => {
                    self.cursor += 2;
                    out.push(Token::MinusMinus);
                }
                '-' if self.starts_with("-=") => {
                    self.cursor += 2;
                    out.push(Token::MinusEqual);
                }
                '-' => {
                    self.advance_char();
                    out.push(Token::Minus);
                }
                '<' if self.starts_with("<<") => {
                    self.cursor += 2;
                    out.push(Token::LessLess);
                }
                '<' if self.starts_with("<=") => {
                    self.cursor += 2;
                    out.push(Token::LessEqual);
                }
                '<' => {
                    self.advance_char();
                    out.push(Token::Less);
                }
                '>' if self.starts_with(">>>") => {
                    self.cursor += 3;
                    out.push(Token::GreaterGreaterGreater);
                }
                '>' if self.starts_with(">>") => {
                    self.cursor += 2;
                    out.push(Token::GreaterGreater);
                }
                '>' if self.starts_with(">=") => {
                    self.cursor += 2;
                    out.push(Token::GreaterEqual);
                }
                '>' => {
                    self.advance_char();
                    out.push(Token::Greater);
                }
                '^' if self.starts_with("^=") => {
                    self.cursor += 2;
                    out.push(Token::CaretEqual);
                }
                '^' => {
                    self.advance_char();
                    out.push(Token::Caret);
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
                        "throw" => Token::KeywordThrow,
                        "break" => Token::KeywordBreak,
                        "switch" => Token::KeywordSwitch,
                        "case" => Token::KeywordCase,
                        "default" => Token::KeywordDefault,
                        "if" => Token::KeywordIf,
                        "else" => Token::KeywordElse,
                        "true" => Token::KeywordTrue,
                        "false" => Token::KeywordFalse,
                        "null" => Token::KeywordNull,
                        "this" => Token::KeywordThis,
                        "new" => Token::KeywordNew,
                        "typeof" => Token::KeywordTypeof,
                        "delete" => Token::KeywordDelete,
                        "try" => Token::KeywordTry,
                        "catch" => Token::KeywordCatch,
                        "while" => Token::KeywordWhile,
                        "for" => Token::KeywordFor,
                        "in" => Token::KeywordIn,
                        "of" => Token::KeywordOf,
                        "instanceof" => Token::KeywordInstanceof,
                        _ => Token::Identifier(identifier),
                    });
                }
                _ => {
                    return Err(format!(
                        "Unsupported token in JS source near byte {}",
                        self.cursor
                    ));
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

    fn consume_template_literal(&mut self) -> Result<TemplateLiteralToken, String> {
        self.advance_char();
        let mut parts = Vec::new();
        let mut current = String::new();

        while let Some(ch) = self.advance_char() {
            match ch {
                '`' => {
                    parts.push(TemplateLiteralPartToken::String(current));
                    return Ok(TemplateLiteralToken { parts });
                }
                '\\' => {
                    let escaped = self
                        .advance_char()
                        .ok_or_else(|| "Unterminated template escape".to_owned())?;
                    match escaped {
                        'n' => current.push('\n'),
                        'r' => current.push('\r'),
                        't' => current.push('\t'),
                        '\\' => current.push('\\'),
                        '\'' => current.push('\''),
                        '"' => current.push('"'),
                        '`' => current.push('`'),
                        '$' => current.push('$'),
                        other => current.push(other),
                    }
                }
                '$' if self.peek_char() == Some('{') => {
                    self.advance_char();
                    parts.push(TemplateLiteralPartToken::String(std::mem::take(&mut current)));
                    let source = self.consume_template_expression_source()?;
                    parts.push(TemplateLiteralPartToken::Expression(source));
                }
                _ => current.push(ch),
            }
        }

        Err("Unterminated template literal".to_owned())
    }

    fn consume_template_expression_source(&mut self) -> Result<String, String> {
        let mut out = String::new();
        let mut depth = 1;

        while let Some(ch) = self.advance_char() {
            match ch {
                '\'' | '"' => {
                    out.push(ch);
                    self.consume_string_source(ch, &mut out)?;
                }
                '`' => {
                    out.push(ch);
                    self.consume_nested_template_source(&mut out)?;
                }
                '{' => {
                    depth += 1;
                    out.push(ch);
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(out);
                    }
                    out.push(ch);
                }
                _ => out.push(ch),
            }
        }

        Err("Unterminated template expression".to_owned())
    }

    fn consume_string_source(&mut self, quote: char, out: &mut String) -> Result<(), String> {
        while let Some(ch) = self.advance_char() {
            out.push(ch);
            if ch == '\\' {
                let escaped = self
                    .advance_char()
                    .ok_or_else(|| "Unterminated string escape".to_owned())?;
                out.push(escaped);
                continue;
            }
            if ch == quote {
                return Ok(());
            }
        }

        Err("Unterminated string literal".to_owned())
    }

    fn consume_nested_template_source(&mut self, out: &mut String) -> Result<(), String> {
        while let Some(ch) = self.advance_char() {
            out.push(ch);
            match ch {
                '\\' => {
                    let escaped = self
                        .advance_char()
                        .ok_or_else(|| "Unterminated template escape".to_owned())?;
                    out.push(escaped);
                }
                '`' => return Ok(()),
                '$' if self.peek_char() == Some('{') => {
                    self.advance_char();
                    out.push('{');
                    let source = self.consume_template_expression_source()?;
                    out.push_str(source.as_str());
                    out.push('}');
                }
                _ => {}
            }
        }

        Err("Unterminated nested template literal".to_owned())
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
            let declarations = self.parse_variable_declaration_list()?;
            self.match_token(&Token::Semicolon);
            return Ok(Statement::VariableDeclaration { declarations });
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

        if self.match_token(&Token::KeywordThrow) {
            let expression = self.parse_expression()?;
            self.match_token(&Token::Semicolon);
            return Ok(Statement::Throw(expression));
        }

        if self.match_token(&Token::KeywordBreak) {
            self.match_token(&Token::Semicolon);
            return Ok(Statement::Break);
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

        if self.match_token(&Token::KeywordWhile) {
            self.expect(&Token::LeftParen)?;
            let test = self.parse_expression()?;
            self.expect(&Token::RightParen)?;
            let body = Box::new(self.parse_statement()?);
            return Ok(Statement::While { test, body });
        }

        if self.match_token(&Token::KeywordSwitch) {
            self.expect(&Token::LeftParen)?;
            let discriminant = self.parse_expression()?;
            self.expect(&Token::RightParen)?;
            self.expect(&Token::LeftBrace)?;
            let mut cases = Vec::new();
            while !self.check(&Token::RightBrace) && !self.is_at_end() {
                let test = if self.match_token(&Token::KeywordCase) {
                    let test = self.parse_expression()?;
                    self.expect(&Token::Colon)?;
                    Some(test)
                } else if self.match_token(&Token::KeywordDefault) {
                    self.expect(&Token::Colon)?;
                    None
                } else {
                    return Err(format!("Unexpected token in JS switch: {:?}", self.peek()));
                };
                let mut consequent = Vec::new();
                while !self.check(&Token::RightBrace)
                    && !self.check(&Token::KeywordCase)
                    && !self.check(&Token::KeywordDefault)
                {
                    while self.match_token(&Token::Semicolon) {}
                    if self.check(&Token::RightBrace)
                        || self.check(&Token::KeywordCase)
                        || self.check(&Token::KeywordDefault)
                    {
                        break;
                    }
                    consequent.push(self.parse_statement()?);
                }
                cases.push(SwitchCase { test, consequent });
            }
            self.expect(&Token::RightBrace)?;
            return Ok(Statement::Switch { discriminant, cases });
        }

        if self.match_token(&Token::KeywordFor) {
            self.expect(&Token::LeftParen)?;
            let init = if self.match_token(&Token::Semicolon) {
                None
            } else if self.match_any(&[Token::KeywordVar, Token::KeywordLet, Token::KeywordConst]) {
                let first = self.parse_variable_declarator(true)?;
                let operator = if self.match_token(&Token::KeywordIn) {
                    Some(ForEachOperator::In)
                } else if self.match_token(&Token::KeywordOf) {
                    Some(ForEachOperator::Of)
                } else {
                    None
                };

                if let Some(operator) = operator {
                    let iterable = self.parse_expression()?;
                    self.expect(&Token::RightParen)?;
                    let body = Box::new(self.parse_statement()?);
                    return Ok(Statement::ForEach {
                        binding: ForEachBinding::Declaration(first),
                        operator,
                        iterable,
                        body,
                    });
                }

                let mut declarations = vec![first];
                while self.match_token(&Token::Comma) {
                    declarations.push(self.parse_variable_declarator(true)?);
                }
                self.expect(&Token::Semicolon)?;
                Some(ForInit::VariableDeclaration(declarations))
            } else {
                let for_each_cursor = self.cursor;
                if let Ok(target) = self.parse_member_expression() {
                    let operator = if self.match_token(&Token::KeywordIn) {
                        Some(ForEachOperator::In)
                    } else if self.match_token(&Token::KeywordOf) {
                        Some(ForEachOperator::Of)
                    } else {
                        None
                    };

                    if let Some(operator) = operator {
                        let iterable = self.parse_expression()?;
                        self.expect(&Token::RightParen)?;
                        let body = Box::new(self.parse_statement()?);
                        return Ok(Statement::ForEach {
                            binding: ForEachBinding::Target(target),
                            operator,
                            iterable,
                            body,
                        });
                    }
                }
                self.cursor = for_each_cursor;
                let expression = self.parse_expression()?;
                self.expect(&Token::Semicolon)?;
                Some(ForInit::Expression(expression))
            };
            let test = if self.check(&Token::Semicolon) {
                None
            } else {
                Some(self.parse_expression()?)
            };
            self.expect(&Token::Semicolon)?;
            let update = if self.check(&Token::RightParen) {
                None
            } else {
                Some(self.parse_expression()?)
            };
            self.expect(&Token::RightParen)?;
            let body = Box::new(self.parse_statement()?);
            return Ok(Statement::For {
                init,
                test,
                update,
                body,
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

    fn parse_function_signature_and_body(
        &mut self,
    ) -> Result<(Vec<Parameter>, Vec<Statement>), String> {
        self.expect(&Token::LeftParen)?;
        let params = self.parse_parameter_list_contents()?;
        self.expect(&Token::RightParen)?;
        let body = self.parse_block_statements()?;
        Ok((params, body))
    }

    fn parse_variable_declaration_list(&mut self) -> Result<Vec<VariableDeclarator>, String> {
        let mut declarations = Vec::new();
        loop {
            declarations.push(self.parse_variable_declarator(true)?);
            if !self.match_token(&Token::Comma) {
                break;
            }
        }
        Ok(declarations)
    }

    fn parse_expression(&mut self) -> Result<Expression, String> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expression, String> {
        if self.is_arrow_function_start()? {
            return self.parse_arrow_function();
        }

        let expression = self.parse_conditional()?;
        if self.match_token(&Token::Equal) {
            let value = self.parse_assignment()?;
            return Ok(Expression::Assignment {
                target: Box::new(expression),
                value: Box::new(value),
            });
        }
        if self.match_token(&Token::PlusEqual) {
            let value = self.parse_assignment()?;
            return Ok(Expression::Assignment {
                target: Box::new(expression.clone()),
                value: Box::new(Expression::Binary {
                    left: Box::new(expression),
                    operator: BinaryOperator::Add,
                    right: Box::new(value),
                }),
            });
        }
        if self.match_token(&Token::MinusEqual) {
            let value = self.parse_assignment()?;
            return Ok(Expression::Assignment {
                target: Box::new(expression.clone()),
                value: Box::new(Expression::Binary {
                    left: Box::new(expression),
                    operator: BinaryOperator::Subtract,
                    right: Box::new(value),
                }),
            });
        }
        if self.match_token(&Token::CaretEqual) {
            let value = self.parse_assignment()?;
            return Ok(Expression::Assignment {
                target: Box::new(expression.clone()),
                value: Box::new(Expression::Binary {
                    left: Box::new(expression),
                    operator: BinaryOperator::BitXor,
                    right: Box::new(value),
                }),
            });
        }
        Ok(expression)
    }

    fn parse_conditional(&mut self) -> Result<Expression, String> {
        let expression = self.parse_logical_or()?;
        if !self.match_token(&Token::Question) {
            return Ok(expression);
        }
        let consequent = self.parse_assignment()?;
        self.expect(&Token::Colon)?;
        let alternate = self.parse_assignment()?;
        Ok(Expression::Conditional {
            test: Box::new(expression),
            consequent: Box::new(consequent),
            alternate: Box::new(alternate),
        })
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
        let mut expression = self.parse_relational()?;
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
            let right = self.parse_relational()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                operator,
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn parse_relational(&mut self) -> Result<Expression, String> {
        let mut expression = self.parse_shift()?;
        loop {
            let operator = if self.match_token(&Token::KeywordIn) {
                Some(BinaryOperator::In)
            } else if self.match_token(&Token::KeywordInstanceof) {
                Some(BinaryOperator::Instanceof)
            } else if self.match_token(&Token::Less) {
                Some(BinaryOperator::Less)
            } else if self.match_token(&Token::LessEqual) {
                Some(BinaryOperator::LessEqual)
            } else if self.match_token(&Token::Greater) {
                Some(BinaryOperator::Greater)
            } else if self.match_token(&Token::GreaterEqual) {
                Some(BinaryOperator::GreaterEqual)
            } else {
                None
            };
            let Some(operator) = operator else {
                break;
            };
            let right = self.parse_shift()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                operator,
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn parse_shift(&mut self) -> Result<Expression, String> {
        let mut expression = self.parse_additive()?;
        loop {
            let operator = if self.match_token(&Token::LessLess) {
                Some(BinaryOperator::LeftShift)
            } else if self.match_token(&Token::GreaterGreaterGreater) {
                Some(BinaryOperator::UnsignedRightShift)
            } else if self.match_token(&Token::GreaterGreater) {
                Some(BinaryOperator::RightShift)
            } else {
                None
            };
            let Some(operator) = operator else {
                break;
            };
            let right = self.parse_additive()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                operator,
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn parse_additive(&mut self) -> Result<Expression, String> {
        let mut expression = self.parse_multiplicative()?;
        loop {
            let operator = if self.match_token(&Token::Plus) {
                Some(BinaryOperator::Add)
            } else if self.match_token(&Token::Minus) {
                Some(BinaryOperator::Subtract)
            } else if self.match_token(&Token::Caret) {
                Some(BinaryOperator::BitXor)
            } else {
                None
            };
            let Some(operator) = operator else {
                break;
            };
            let right = self.parse_multiplicative()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                operator,
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn parse_multiplicative(&mut self) -> Result<Expression, String> {
        let mut expression = self.parse_unary()?;
        loop {
            let operator = if self.match_token(&Token::Star) {
                Some(BinaryOperator::Multiply)
            } else if self.match_token(&Token::Slash) {
                Some(BinaryOperator::Divide)
            } else if self.match_token(&Token::Percent) {
                Some(BinaryOperator::Modulo)
            } else {
                None
            };
            let Some(operator) = operator else {
                break;
            };
            let right = self.parse_unary()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                operator,
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn parse_unary(&mut self) -> Result<Expression, String> {
        if self.match_token(&Token::PlusPlus) {
            return Ok(Expression::Update {
                target: Box::new(self.parse_unary()?),
                operator: UpdateOperator::Increment,
                prefix: true,
            });
        }
        if self.match_token(&Token::MinusMinus) {
            return Ok(Expression::Update {
                target: Box::new(self.parse_unary()?),
                operator: UpdateOperator::Decrement,
                prefix: true,
            });
        }
        if self.match_token(&Token::Plus) {
            return Ok(Expression::Unary {
                operator: UnaryOperator::Plus,
                argument: Box::new(self.parse_unary()?),
            });
        }
        if self.match_token(&Token::Minus) {
            return Ok(Expression::Unary {
                operator: UnaryOperator::Minus,
                argument: Box::new(self.parse_unary()?),
            });
        }
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
        if self.match_token(&Token::KeywordDelete) {
            return Ok(Expression::Unary {
                operator: UnaryOperator::Delete,
                argument: Box::new(self.parse_unary()?),
            });
        }
        if self.match_token(&Token::KeywordNew) {
            let callee = self.parse_member_expression()?;
            let arguments = if self.match_token(&Token::LeftParen) {
                self.parse_call_arguments()?
            } else {
                Vec::new()
            };
            return self.finish_call_member(Expression::New {
                callee: Box::new(callee),
                arguments,
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expression, String> {
        let expression = self.parse_call_member()?;
        if self.match_token(&Token::PlusPlus) {
            return Ok(Expression::Update {
                target: Box::new(expression),
                operator: UpdateOperator::Increment,
                prefix: false,
            });
        }
        if self.match_token(&Token::MinusMinus) {
            return Ok(Expression::Update {
                target: Box::new(expression),
                operator: UpdateOperator::Decrement,
                prefix: false,
            });
        }
        Ok(expression)
    }

    fn parse_call_member(&mut self) -> Result<Expression, String> {
        let expression = self.parse_member_expression()?;
        self.finish_call_member(expression)
    }

    fn parse_member_expression(&mut self) -> Result<Expression, String> {
        let mut expression = self.parse_primary()?;

        loop {
            if self.match_token(&Token::Dot) {
                let property = self.expect_property_identifier()?;
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

    fn finish_call_member(&mut self, mut expression: Expression) -> Result<Expression, String> {
        loop {
            if self.match_token(&Token::LeftParen) {
                let arguments = self.parse_call_arguments()?;
                expression = Expression::Call {
                    callee: Box::new(expression),
                    arguments,
                };
                continue;
            }

            if self.match_token(&Token::Dot) {
                let property = self.expect_property_identifier()?;
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

            break;
        }

        Ok(expression)
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
            Token::TemplateLiteral(template) => self.parse_template_literal(template),
            Token::LeftParen => {
                let mut expressions = vec![self.parse_assignment()?];
                while self.match_token(&Token::Comma) {
                    expressions.push(self.parse_assignment()?);
                }
                self.expect(&Token::RightParen)?;
                Ok(if expressions.len() == 1 {
                    expressions.pop().unwrap_or(Expression::Null)
                } else {
                    Expression::Sequence(expressions)
                })
            }
            Token::LeftBracket => {
                let mut items = Vec::new();
                if !self.check(&Token::RightBracket) {
                    loop {
                        if self.match_token(&Token::Ellipsis) {
                            items.push(ArrayElement::Spread(self.parse_expression()?));
                        } else {
                            items.push(ArrayElement::Expression(self.parse_expression()?));
                        }
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
                        if self.match_token(&Token::LeftBracket) {
                            let key = self.parse_expression()?;
                            self.expect(&Token::RightBracket)?;
                            self.expect(&Token::Colon)?;
                            let value = self.parse_expression()?;
                            properties.push(ObjectProperty::Computed(key, value));
                            if !self.match_token(&Token::Comma) {
                                break;
                            }
                            if self.check(&Token::RightBrace) {
                                break;
                            }
                            continue;
                        }
                        let key = match self.advance() {
                            Token::Identifier(name) => {
                                if self.match_token(&Token::Colon) {
                                    let key = ObjectKey::Identifier(name);
                                    let value = self.parse_expression()?;
                                    properties.push(ObjectProperty::KeyValue(key, value));
                                } else {
                                    properties.push(ObjectProperty::Shorthand(name));
                                }
                                if !self.match_token(&Token::Comma) {
                                    break;
                                }
                                if self.check(&Token::RightBrace) {
                                    break;
                                }
                                continue;
                            }
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
                        properties.push(ObjectProperty::KeyValue(key, value));
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
                Ok(Expression::Function {
                    params,
                    body,
                    lexical_this: false,
                    constructible: true,
                })
            }
            other => Err(format!("Unexpected token in JS expression: {other:?}")),
        }
    }

    fn parse_variable_declarator(
        &mut self,
        allow_initializer: bool,
    ) -> Result<VariableDeclarator, String> {
        let pattern = self.parse_binding_pattern()?;
        let init = if self.match_token(&Token::Equal) {
            if !allow_initializer {
                return Err("Unexpected initializer in JS for-in/of binding".to_owned());
            }
            Some(self.parse_expression()?)
        } else {
            None
        };
        Ok(VariableDeclarator { pattern, init })
    }

    fn parse_binding_pattern(&mut self) -> Result<BindingPattern, String> {
        if self.match_token(&Token::LeftBrace) {
            let mut properties = Vec::new();
            if !self.check(&Token::RightBrace) {
                loop {
                    let key = self.expect_property_identifier()?;
                    let binding = if self.match_token(&Token::Colon) {
                        self.expect_identifier()?
                    } else {
                        key.clone()
                    };
                    properties.push(ObjectBindingProperty { key, binding });
                    if !self.match_token(&Token::Comma) {
                        break;
                    }
                    if self.check(&Token::RightBrace) {
                        break;
                    }
                }
            }
            self.expect(&Token::RightBrace)?;
            return Ok(BindingPattern::Object(properties));
        }

        Ok(BindingPattern::Identifier(self.expect_identifier()?))
    }

    fn parse_parameter_list_contents(&mut self) -> Result<Vec<Parameter>, String> {
        let mut params = Vec::new();
        if !self.check(&Token::RightParen) {
            loop {
                params.push(self.parse_parameter()?);
                if !self.match_token(&Token::Comma) {
                    break;
                }
                if self.check(&Token::RightParen) {
                    break;
                }
            }
        }
        Ok(params)
    }

    fn parse_parameter(&mut self) -> Result<Parameter, String> {
        let name = self.expect_identifier()?;
        let default = if self.match_token(&Token::Equal) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        Ok(Parameter { name, default })
    }

    fn is_arrow_function_start(&mut self) -> Result<bool, String> {
        let saved = self.cursor;
        let result = if matches!(self.peek(), Token::Identifier(_))
            && matches!(self.peek_next(), Token::Arrow)
        {
            true
        } else if self.match_token(&Token::LeftParen) {
            let params_ok = if self.check(&Token::RightParen) {
                true
            } else {
                self.parse_parameter_list_contents().is_ok()
            };
            params_ok && self.match_token(&Token::RightParen) && self.check(&Token::Arrow)
        } else {
            false
        };
        self.cursor = saved;
        Ok(result)
    }

    fn parse_arrow_function(&mut self) -> Result<Expression, String> {
        let params = if matches!(self.peek(), Token::Identifier(_))
            && matches!(self.peek_next(), Token::Arrow)
        {
            vec![Parameter {
                name: self.expect_identifier()?,
                default: None,
            }]
        } else {
            self.expect(&Token::LeftParen)?;
            let params = self.parse_parameter_list_contents()?;
            self.expect(&Token::RightParen)?;
            params
        };
        self.expect(&Token::Arrow)?;
        let body = if self.check(&Token::LeftBrace) {
            self.parse_block_statements()?
        } else {
            vec![Statement::Return(Some(self.parse_expression()?))]
        };
        Ok(Expression::Function {
            params,
            body,
            lexical_this: true,
            constructible: false,
        })
    }

    fn parse_template_literal(
        &self,
        template: TemplateLiteralToken,
    ) -> Result<Expression, String> {
        let mut segments = Vec::new();
        for part in template.parts {
            match part {
                TemplateLiteralPartToken::String(value) => {
                    segments.push(TemplateSegment::String(value));
                }
                TemplateLiteralPartToken::Expression(source) => {
                    let expression = Parser::new(Lexer::new(source.as_str()).tokenize()?)
                        .parse_expression_only()?;
                    segments.push(TemplateSegment::Expression(expression));
                }
            }
        }
        Ok(Expression::TemplateLiteral(segments))
    }

    fn parse_expression_only(mut self) -> Result<Expression, String> {
        let expression = self.parse_expression()?;
        if !self.is_at_end() {
            return Err(format!(
                "Unexpected trailing token in JS expression: {:?}",
                self.peek()
            ));
        }
        Ok(expression)
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek(), Token::Eof)
    }

    fn peek(&self) -> Token {
        self.tokens.get(self.cursor).cloned().unwrap_or(Token::Eof)
    }

    fn peek_next(&self) -> Token {
        self.tokens
            .get(self.cursor + 1)
            .cloned()
            .unwrap_or(Token::Eof)
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

    fn expect_property_identifier(&mut self) -> Result<String, String> {
        match self.advance() {
            Token::Identifier(name) => Ok(name),
            Token::KeywordVar => Ok("var".to_owned()),
            Token::KeywordLet => Ok("let".to_owned()),
            Token::KeywordConst => Ok("const".to_owned()),
            Token::KeywordFunction => Ok("function".to_owned()),
            Token::KeywordReturn => Ok("return".to_owned()),
            Token::KeywordThrow => Ok("throw".to_owned()),
            Token::KeywordBreak => Ok("break".to_owned()),
            Token::KeywordSwitch => Ok("switch".to_owned()),
            Token::KeywordCase => Ok("case".to_owned()),
            Token::KeywordDefault => Ok("default".to_owned()),
            Token::KeywordIf => Ok("if".to_owned()),
            Token::KeywordElse => Ok("else".to_owned()),
            Token::KeywordTrue => Ok("true".to_owned()),
            Token::KeywordFalse => Ok("false".to_owned()),
            Token::KeywordNull => Ok("null".to_owned()),
            Token::KeywordThis => Ok("this".to_owned()),
            Token::KeywordNew => Ok("new".to_owned()),
            Token::KeywordTypeof => Ok("typeof".to_owned()),
            Token::KeywordDelete => Ok("delete".to_owned()),
            Token::KeywordTry => Ok("try".to_owned()),
            Token::KeywordCatch => Ok("catch".to_owned()),
            Token::KeywordWhile => Ok("while".to_owned()),
            Token::KeywordFor => Ok("for".to_owned()),
            Token::KeywordIn => Ok("in".to_owned()),
            Token::KeywordOf => Ok("of".to_owned()),
            Token::KeywordInstanceof => Ok("instanceof".to_owned()),
            other => Err(format!("Expected property identifier, found {other:?}")),
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
    Set(SetRef),
    Function(Rc<FunctionValue>),
    BoundFunction(Rc<BoundFunctionValue>),
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
struct BoundFunctionValue {
    target: Value,
    this_value: Value,
    preset_arguments: Vec<Value>,
    properties: ObjectRef,
}

#[derive(Clone)]
struct UserFunction {
    params: Vec<Parameter>,
    body: Vec<Statement>,
    env: ScopeRef,
    lexical_this: bool,
    constructible: bool,
}

#[derive(Clone, Copy)]
enum NativeFunction {
    Dollar,
    ArrayIsArray,
    ObjectAssign,
    ObjectCreate,
    ObjectKeys,
    ObjectHasOwnProperty,
    ErrorConstructor,
    SetConstructor,
    RegExpConstructor,
    FunctionConstructor,
    FunctionCall,
    FunctionApply,
    FunctionBind,
    ArrayForEach,
    ArrayJoin,
    ArrayPush,
    ArrayMap,
    ArrayFilter,
    ArraySlice,
    ArraySort,
    ArrayReduce,
    SetTimeout,
    ClearTimeout,
    RequestIdleCallback,
    CancelIdleCallback,
    ReturnZero,
    DateNow,
    MathMax,
    MathMin,
    MathRound,
    MathCeil,
    EncodeURIComponent,
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
    Break,
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
            "setTimeout",
            executor.create_native_function(NativeFunction::SetTimeout),
        );
        executor.declare_global(
            "clearTimeout",
            executor.create_native_function(NativeFunction::ClearTimeout),
        );
        executor.declare_global(
            "requestIdleCallback",
            executor.create_native_function(NativeFunction::RequestIdleCallback),
        );
        executor.declare_global(
            "cancelIdleCallback",
            executor.create_native_function(NativeFunction::CancelIdleCallback),
        );
        executor.declare_global(
            "encodeURIComponent",
            executor.create_native_function(NativeFunction::EncodeURIComponent),
        );
        let console = executor.create_plain_object();
        let _ = executor.set_property_value(
            console.clone(),
            "log",
            executor.create_native_function(NativeFunction::Noop),
        );
        let _ = executor.set_property_value(
            console.clone(),
            "warn",
            executor.create_native_function(NativeFunction::Noop),
        );
        executor.declare_global("console", console);

        let local_storage = executor.create_plain_object();
        let _ = executor.set_property_value(
            local_storage.clone(),
            "getItem",
            executor.create_native_function(NativeFunction::Noop),
        );
        let _ = executor.set_property_value(
            local_storage.clone(),
            "setItem",
            executor.create_native_function(NativeFunction::Noop),
        );
        let _ = executor.set_property_value(
            local_storage.clone(),
            "removeItem",
            executor.create_native_function(NativeFunction::Noop),
        );
        executor.declare_global("localStorage", local_storage);

        let performance = executor.create_plain_object();
        let timing = executor.create_plain_object();
        let _ = executor.set_property_value(timing.clone(), "navigationStart", Value::Number(0.0));
        let _ = executor.set_property_value(
            performance.clone(),
            "mark",
            executor.create_native_function(NativeFunction::Noop),
        );
        let _ = executor.set_property_value(
            performance.clone(),
            "now",
            executor.create_native_function(NativeFunction::ReturnZero),
        );
        let _ = executor.set_property_value(performance.clone(), "timing", timing);
        executor.declare_global("performance", performance);

        let date = executor.create_plain_object();
        let _ = executor.set_property_value(
            date.clone(),
            "now",
            executor.create_native_function(NativeFunction::DateNow),
        );
        executor.declare_global("Date", date);

        let math = executor.create_plain_object();
        let _ = executor.set_property_value(
            math.clone(),
            "max",
            executor.create_native_function(NativeFunction::MathMax),
        );
        let _ = executor.set_property_value(
            math.clone(),
            "min",
            executor.create_native_function(NativeFunction::MathMin),
        );
        let _ = executor.set_property_value(
            math.clone(),
            "round",
            executor.create_native_function(NativeFunction::MathRound),
        );
        let _ = executor.set_property_value(
            math.clone(),
            "ceil",
            executor.create_native_function(NativeFunction::MathCeil),
        );
        executor.declare_global("Math", math);

        let object_ctor = executor.create_native_function(NativeFunction::Noop);
        let _ = executor.set_property_value(
            object_ctor.clone(),
            "assign",
            executor.create_native_function(NativeFunction::ObjectAssign),
        );
        let _ = executor.set_property_value(
            object_ctor.clone(),
            "create",
            executor.create_native_function(NativeFunction::ObjectCreate),
        );
        let _ = executor.set_property_value(
            object_ctor.clone(),
            "keys",
            executor.create_native_function(NativeFunction::ObjectKeys),
        );
        let _ = executor.set_property_value(
            object_ctor.clone(),
            "hasOwnProperty",
            executor.create_native_function(NativeFunction::ObjectHasOwnProperty),
        );
        executor.declare_global("Object", object_ctor);

        let error_ctor = executor.create_native_function(NativeFunction::ErrorConstructor);
        executor.declare_global("Error", error_ctor);

        executor.declare_global(
            "RegExp",
            executor.create_native_function(NativeFunction::RegExpConstructor),
        );
        let function_ctor = executor.create_native_function(NativeFunction::FunctionConstructor);
        let function_proto = executor.create_plain_object();
        let _ = executor.set_property_value(
            function_proto.clone(),
            "call",
            executor.create_native_function(NativeFunction::FunctionCall),
        );
        let _ = executor.set_property_value(
            function_proto.clone(),
            "apply",
            executor.create_native_function(NativeFunction::FunctionApply),
        );
        let _ = executor.set_property_value(
            function_proto.clone(),
            "bind",
            executor.create_native_function(NativeFunction::FunctionBind),
        );
        let _ = executor.set_property_value(function_ctor.clone(), "prototype", function_proto);
        executor.declare_global("Function", function_ctor);

        let array_ctor = executor.create_native_function(NativeFunction::Noop);
        let array_proto = executor.create_plain_object();
        let _ = executor.set_property_value(
            array_proto.clone(),
            "forEach",
            executor.create_native_function(NativeFunction::ArrayForEach),
        );
        let _ = executor.set_property_value(
            array_proto.clone(),
            "join",
            executor.create_native_function(NativeFunction::ArrayJoin),
        );
        let _ = executor.set_property_value(
            array_proto.clone(),
            "push",
            executor.create_native_function(NativeFunction::ArrayPush),
        );
        let _ = executor.set_property_value(
            array_proto.clone(),
            "map",
            executor.create_native_function(NativeFunction::ArrayMap),
        );
        let _ = executor.set_property_value(
            array_proto.clone(),
            "filter",
            executor.create_native_function(NativeFunction::ArrayFilter),
        );
        let _ = executor.set_property_value(
            array_proto.clone(),
            "slice",
            executor.create_native_function(NativeFunction::ArraySlice),
        );
        let _ = executor.set_property_value(
            array_proto.clone(),
            "sort",
            executor.create_native_function(NativeFunction::ArraySort),
        );
        let _ = executor.set_property_value(
            array_proto.clone(),
            "reduce",
            executor.create_native_function(NativeFunction::ArrayReduce),
        );
        let _ = executor.set_property_value(
            array_ctor.clone(),
            "isArray",
            executor.create_native_function(NativeFunction::ArrayIsArray),
        );
        let _ = executor.set_property_value(array_ctor.clone(), "prototype", array_proto);
        executor.declare_global("Array", array_ctor);

        executor.declare_global(
            "Set",
            executor.create_native_function(NativeFunction::SetConstructor),
        );

        let element_ctor = executor.create_native_function(NativeFunction::Noop);
        executor.declare_global("Element", element_ctor.clone());
        executor.declare_global("HTMLElement", element_ctor);

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

    fn with_global(document: &'a mut Document, global: ScopeRef) -> Self {
        Self {
            document,
            global: global.clone(),
            scope: global,
        }
    }

    fn execute_program(&mut self, statements: &[Statement]) -> Result<(), String> {
        match self.execute_statements(statements)? {
            ControlFlow::Continue(_) | ControlFlow::Return(_) | ControlFlow::Break => Ok(()),
        }
    }

    fn execute_statements(&mut self, statements: &[Statement]) -> Result<ControlFlow, String> {
        let mut last = Value::Undefined;
        for statement in statements {
            match self.execute_statement(statement)? {
                ControlFlow::Continue(value) => last = value,
                ControlFlow::Return(value) => return Ok(ControlFlow::Return(value)),
                ControlFlow::Break => return Ok(ControlFlow::Break),
            }
        }
        Ok(ControlFlow::Continue(last))
    }

    fn execute_statement(&mut self, statement: &Statement) -> Result<ControlFlow, String> {
        match statement {
            Statement::VariableDeclaration { declarations } => {
                let mut last = Value::Undefined;
                for declaration in declarations {
                    let value = if let Some(init) = &declaration.init {
                        self.evaluate_expression(init)?
                    } else {
                        Value::Undefined
                    };
                    self.declare_pattern(&declaration.pattern, value.clone())?;
                    last = value;
                }
                Ok(ControlFlow::Continue(last))
            }
            Statement::FunctionDeclaration { name, params, body } => {
                let function =
                    self.create_user_function(params.clone(), body.clone(), false, true);
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
            Statement::Throw(expression) => {
                let value = self.evaluate_expression(expression)?;
                Err(self.error_to_string(&value))
            }
            Statement::Break => Ok(ControlFlow::Break),
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
            Statement::While { test, body } => {
                let mut last = Value::Undefined;
                while self.evaluate_expression(test)?.is_truthy() {
                    match self.execute_statement(body)? {
                        ControlFlow::Continue(value) => last = value,
                        ControlFlow::Return(value) => return Ok(ControlFlow::Return(value)),
                        ControlFlow::Break => break,
                    }
                }
                Ok(ControlFlow::Continue(last))
            }
            Statement::Switch { discriminant, cases } => {
                let value = self.evaluate_expression(discriminant)?;
                let default_index = cases.iter().position(|case| case.test.is_none());
                let mut index = cases
                    .iter()
                    .position(|case| {
                        case.test.as_ref().is_some_and(|test| {
                            self.evaluate_expression(test)
                                .is_ok_and(|candidate| strict_equals(&value, &candidate))
                        })
                    })
                    .or(default_index);
                let mut last = Value::Undefined;
                while let Some(current) = index {
                    match self.execute_statements(&cases[current].consequent)? {
                        ControlFlow::Continue(value) => last = value,
                        ControlFlow::Return(value) => return Ok(ControlFlow::Return(value)),
                        ControlFlow::Break => return Ok(ControlFlow::Continue(last)),
                    }
                    index = current
                        .checked_add(1)
                        .filter(|next| *next < cases.len());
                }
                Ok(ControlFlow::Continue(last))
            }
            Statement::For {
                init,
                test,
                update,
                body,
            } => {
                let mut last = Value::Undefined;
                if let Some(init) = init {
                    match init {
                        ForInit::VariableDeclaration(declarations) => {
                            for declaration in declarations {
                                let value = if let Some(init) = &declaration.init {
                                    self.evaluate_expression(init)?
                                } else {
                                    Value::Undefined
                                };
                                self.declare_pattern(&declaration.pattern, value)?;
                            }
                        }
                        ForInit::Expression(expression) => {
                            last = self.evaluate_expression(expression)?;
                        }
                    }
                }
                loop {
                    if let Some(test) = test
                        && !self.evaluate_expression(test)?.is_truthy()
                    {
                        break;
                    }
                    match self.execute_statement(body)? {
                        ControlFlow::Continue(value) => last = value,
                        ControlFlow::Return(value) => return Ok(ControlFlow::Return(value)),
                        ControlFlow::Break => break,
                    }
                    if let Some(update) = update {
                        last = self.evaluate_expression(update)?;
                    }
                }
                Ok(ControlFlow::Continue(last))
            }
            Statement::ForEach {
                binding,
                operator,
                iterable,
                body,
            } => {
                let values = match operator {
                    ForEachOperator::In => own_property_names(&self.evaluate_expression(iterable)?)?
                        .into_iter()
                        .map(Value::String)
                        .collect::<Vec<_>>(),
                    ForEachOperator::Of => array_like_values(&self.evaluate_expression(iterable)?),
                };
                let mut last = Value::Undefined;
                for value in values {
                    match binding {
                        ForEachBinding::Declaration(binding) => {
                            self.declare_pattern(&binding.pattern, value)?;
                        }
                        ForEachBinding::Target(target) => {
                            self.assign_target(target, value)?;
                        }
                    }
                    match self.execute_statement(body)? {
                        ControlFlow::Continue(next) => last = next,
                        ControlFlow::Return(value) => return Ok(ControlFlow::Return(value)),
                        ControlFlow::Break => break,
                    }
                }
                Ok(ControlFlow::Continue(last))
            }
            Statement::Expression(expression) => {
                Ok(ControlFlow::Continue(self.evaluate_expression(expression)?))
            }
        }
    }

    fn evaluate_expression(&mut self, expression: &Expression) -> Result<Value, String> {
        match expression {
            Expression::Identifier(name) => {
                Ok(self.lookup_binding(name).unwrap_or(Value::Undefined))
            }
            Expression::This => Ok(self.lookup_binding("this").unwrap_or(Value::Window)),
            Expression::String(value) => Ok(Value::String(value.clone())),
            Expression::Number(value) => Ok(Value::Number(*value)),
            Expression::Bool(value) => Ok(Value::Bool(*value)),
            Expression::Null => Ok(Value::Null),
            Expression::Regex { source, flags } => Ok(Value::Regex(RegexValue {
                source: source.clone(),
                flags: flags.clone(),
            })),
            Expression::TemplateLiteral(segments) => {
                let mut out = String::new();
                for segment in segments {
                    match segment {
                        TemplateSegment::String(value) => out.push_str(value.as_str()),
                        TemplateSegment::Expression(expression) => out.push_str(
                            self.evaluate_expression(expression)?.to_string_value().as_str(),
                        ),
                    }
                }
                Ok(Value::String(out))
            }
            Expression::Array(items) => {
                let mut values = Vec::with_capacity(items.len());
                for item in items {
                    match item {
                        ArrayElement::Expression(expression) => {
                            values.push(self.evaluate_expression(expression)?);
                        }
                        ArrayElement::Spread(expression) => values
                            .extend(array_like_values(&self.evaluate_expression(expression)?)),
                    }
                }
                Ok(new_array_value(values))
            }
            Expression::Object(properties) => {
                let object = self.create_plain_object();
                for property in properties {
                    match property {
                        ObjectProperty::KeyValue(key, value) => {
                            let key = object_key_to_string(key);
                            let value = self.evaluate_expression(value)?;
                            self.set_property_value(object.clone(), key.as_str(), value)?;
                        }
                        ObjectProperty::Computed(key, value) => {
                            let key = self.evaluate_expression(key)?.to_property_key();
                            let value = self.evaluate_expression(value)?;
                            self.set_property_value(object.clone(), key.as_str(), value)?;
                        }
                        ObjectProperty::Shorthand(name) => {
                            let value = self.lookup_binding(name).unwrap_or(Value::Undefined);
                            self.set_property_value(object.clone(), name.as_str(), value)?;
                        }
                    }
                }
                Ok(object)
            }
            Expression::Function {
                params,
                body,
                lexical_this,
                constructible,
            } => Ok(self.create_user_function(
                params.clone(),
                body.clone(),
                *lexical_this,
                *constructible,
            )),
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
            Expression::Update {
                target,
                operator,
                prefix,
            } => self.evaluate_update(target, *operator, *prefix),
            Expression::Unary { operator, argument } => self.evaluate_unary(*operator, argument),
            Expression::Conditional {
                test,
                consequent,
                alternate,
            } => {
                if self.evaluate_expression(test)?.is_truthy() {
                    self.evaluate_expression(consequent)
                } else {
                    self.evaluate_expression(alternate)
                }
            }
            Expression::Sequence(expressions) => {
                let mut last = Value::Undefined;
                for expression in expressions {
                    last = self.evaluate_expression(expression)?;
                }
                Ok(last)
            }
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
            UnaryOperator::Not => Ok(Value::Bool(
                !self.evaluate_expression(argument)?.is_truthy(),
            )),
            UnaryOperator::Plus => Ok(Value::Number(
                self.evaluate_expression(argument)?.to_number_value(),
            )),
            UnaryOperator::Minus => Ok(Value::Number(
                -self.evaluate_expression(argument)?.to_number_value(),
            )),
            UnaryOperator::Typeof => Ok(Value::String(
                self.evaluate_expression(argument)?.type_name(),
            )),
            UnaryOperator::Delete => self.delete_target(argument),
        }
    }

    fn evaluate_update(
        &mut self,
        target: &Expression,
        operator: UpdateOperator,
        prefix: bool,
    ) -> Result<Value, String> {
        let current = self.read_target_value(target)?;
        let delta = match operator {
            UpdateOperator::Increment => 1.0,
            UpdateOperator::Decrement => -1.0,
        };
        let updated = Value::Number(current.to_number_value() + delta);
        self.assign_target(target, updated.clone())?;
        if prefix { Ok(updated) } else { Ok(current) }
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
                Ok(Value::Number(
                    left.to_number_value() + right.to_number_value(),
                ))
            }
            BinaryOperator::Subtract => {
                let left = self.evaluate_expression(left)?;
                let right = self.evaluate_expression(right)?;
                Ok(Value::Number(
                    left.to_number_value() - right.to_number_value(),
                ))
            }
            BinaryOperator::Multiply => {
                let left = self.evaluate_expression(left)?;
                let right = self.evaluate_expression(right)?;
                Ok(Value::Number(
                    left.to_number_value() * right.to_number_value(),
                ))
            }
            BinaryOperator::Divide => {
                let left = self.evaluate_expression(left)?;
                let right = self.evaluate_expression(right)?;
                Ok(Value::Number(
                    left.to_number_value() / right.to_number_value(),
                ))
            }
            BinaryOperator::Modulo => {
                let left = self.evaluate_expression(left)?;
                let right = self.evaluate_expression(right)?;
                Ok(Value::Number(
                    left.to_number_value() % right.to_number_value(),
                ))
            }
            BinaryOperator::LeftShift => {
                let left = to_js_int32(self.evaluate_expression(left)?.to_number_value());
                let right = shift_count(self.evaluate_expression(right)?.to_number_value());
                Ok(Value::Number((((left as u32) << right) as i32) as f64))
            }
            BinaryOperator::RightShift => {
                let left = to_js_int32(self.evaluate_expression(left)?.to_number_value());
                let right = shift_count(self.evaluate_expression(right)?.to_number_value());
                Ok(Value::Number((left >> right) as f64))
            }
            BinaryOperator::UnsignedRightShift => {
                let left = to_js_uint32(self.evaluate_expression(left)?.to_number_value());
                let right = shift_count(self.evaluate_expression(right)?.to_number_value());
                Ok(Value::Number((left >> right) as f64))
            }
            BinaryOperator::Less => {
                let left = self.evaluate_expression(left)?;
                let right = self.evaluate_expression(right)?;
                Ok(Value::Bool(compare_values(&left, &right) < 0))
            }
            BinaryOperator::LessEqual => {
                let left = self.evaluate_expression(left)?;
                let right = self.evaluate_expression(right)?;
                Ok(Value::Bool(compare_values(&left, &right) <= 0))
            }
            BinaryOperator::Greater => {
                let left = self.evaluate_expression(left)?;
                let right = self.evaluate_expression(right)?;
                Ok(Value::Bool(compare_values(&left, &right) > 0))
            }
            BinaryOperator::GreaterEqual => {
                let left = self.evaluate_expression(left)?;
                let right = self.evaluate_expression(right)?;
                Ok(Value::Bool(compare_values(&left, &right) >= 0))
            }
            BinaryOperator::In => {
                let property = self.evaluate_expression(left)?.to_property_key();
                let object = self.evaluate_expression(right)?;
                Ok(Value::Bool(self.has_property(object, property.as_str())))
            }
            BinaryOperator::Instanceof => {
                let left = self.evaluate_expression(left)?;
                let right = self.evaluate_expression(right)?;
                Ok(Value::Bool(self.instanceof_value(&left, &right)))
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
            BinaryOperator::BitXor => {
                let left = self.evaluate_expression(left)?;
                let right = self.evaluate_expression(right)?;
                Ok(Value::Number(
                    ((left.to_number_value() as i32) ^ (right.to_number_value() as i32)) as f64,
                ))
            }
        }
    }

    fn evaluate_call(
        &mut self,
        callee: &Expression,
        arguments: &[Expression],
    ) -> Result<Value, String> {
        let context = format!("call {}", expression_debug_label(callee));
        let evaluated_arguments = arguments
            .iter()
            .map(|argument| self.evaluate_expression(argument))
            .collect::<Result<Vec<_>, _>>()?;

        let result = match callee {
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
                let callee_value = self.evaluate_expression(callee)?;
                if matches!(callee_value, Value::Undefined) {
                    return Err(format!(
                        "Unsupported JS call target: {}",
                        expression_debug_label(callee)
                    ));
                }
                self.call_value(callee_value, Value::Window, &evaluated_arguments)
            }
        };

        self.annotate_trace_error(result, context.as_str())
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

    fn read_target_value(&mut self, target: &Expression) -> Result<Value, String> {
        match target {
            Expression::Identifier(name) => {
                Ok(self.lookup_binding(name).unwrap_or(Value::Undefined))
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
            _ => Err("Unsupported assignment target".to_owned()),
        }
    }

    fn delete_target(&mut self, target: &Expression) -> Result<Value, String> {
        match target {
            Expression::Identifier(_) => Ok(Value::Bool(true)),
            Expression::Member { object, property } => {
                let object = self.evaluate_expression(object)?;
                Ok(Value::Bool(self.delete_property(object, property)))
            }
            Expression::ComputedMember { object, property } => {
                let object = self.evaluate_expression(object)?;
                let property = self.evaluate_expression(property)?.to_property_key();
                Ok(Value::Bool(self.delete_property(object, property.as_str())))
            }
            _ => Ok(Value::Bool(true)),
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
        let context = format!("member {}.{}", receiver.trace_label(), property);
        let result = match receiver.clone() {
            Value::Document => self.call_document_method(property, arguments),
            Value::LiveElement(node_id) => self.call_element_method(node_id, property, arguments),
            Value::ClassList(node_id) => self.call_class_list_method(node_id, property, arguments),
            Value::String(text) => self.call_string_method(text.as_str(), property, arguments),
            Value::Number(number) => self.call_number_method(number, property, arguments),
            Value::Array(items) => self.call_array_method(items, property, arguments),
            Value::Set(items) => self.call_set_method(items, property, arguments),
            Value::JQueryCollection(ids) => self.call_jquery_method(ids, property, arguments),
            _ => {
                let callee = self.get_member_value(receiver.clone(), property)?;
                if matches!(callee, Value::Undefined) {
                    return Err(format!(
                        "Unsupported member call: {}.{}",
                        receiver.debug_label(),
                        property
                    ));
                }
                self.call_value(callee, receiver, arguments)
            }
        };

        self.annotate_trace_error(result, context.as_str())
    }

    fn call_value(
        &mut self,
        callee: Value,
        this_value: Value,
        arguments: &[Value],
    ) -> Result<Value, String> {
        let context = format!("invoke {}", callee.trace_label());
        let result = match callee {
            Value::Function(function) => match function.kind.clone() {
                FunctionKind::Native(kind) => {
                    self.call_native_function(kind, this_value, arguments)
                }
                FunctionKind::User(function) => {
                    let previous_scope = self.scope.clone();
                    let next_scope = Scope::new(Some(function.env.clone()));
                    {
                        let mut bindings = next_scope.borrow_mut();
                        if !function.lexical_this {
                            bindings.bindings.insert("this".to_owned(), this_value);
                        }
                    }
                    self.scope = next_scope;
                    for (index, param) in function.params.iter().enumerate() {
                        let argument = arguments.get(index).cloned().unwrap_or(Value::Undefined);
                        let value = if matches!(argument, Value::Undefined) {
                            if let Some(default) = &param.default {
                                self.evaluate_expression(default)?
                            } else {
                                Value::Undefined
                            }
                        } else {
                            argument
                        };
                        self.declare_binding(&param.name, value);
                    }
                    let result = match self.execute_statements(&function.body)? {
                        ControlFlow::Continue(_) => Value::Undefined,
                        ControlFlow::Return(value) => value,
                        ControlFlow::Break => Value::Undefined,
                    };
                    self.scope = previous_scope;
                    Ok(result)
                }
            },
            Value::BoundFunction(function) => {
                let mut bound_arguments = function.preset_arguments.clone();
                bound_arguments.extend_from_slice(arguments);
                self.call_value(
                    function.target.clone(),
                    function.this_value.clone(),
                    &bound_arguments,
                )
            }
            _ => Err(format!(
                "Unsupported JS call target: {}",
                callee.debug_label()
            )),
        };

        self.annotate_trace_error(result, context.as_str())
    }

    fn construct_value(&mut self, callee: Value, arguments: &[Value]) -> Result<Value, String> {
        let Value::Function(function) = callee else {
            return Err(format!(
                "Unsupported constructor: {}",
                callee.debug_label()
            ));
        };

        match function.kind.clone() {
            FunctionKind::Native(kind) => {
                self.call_native_function(kind, Value::Undefined, arguments)
            }
            FunctionKind::User(function_state) => {
                if !function_state.constructible {
                    return Err("Unsupported constructor".to_owned());
                }
                let prototype = function.properties.borrow().get("prototype").cloned();
                let instance = self.create_object_with_prototype(prototype);
                let value =
                    self.call_value(Value::Function(function), instance.clone(), arguments)?;
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
        this_value: Value,
        arguments: &[Value],
    ) -> Result<Value, String> {
        match kind {
            NativeFunction::Dollar => {
                let selector = first_string_argument(arguments)?;
                Ok(Value::JQueryCollection(
                    self.document.query_selector_all_ids(selector.as_str()),
                ))
            }
            NativeFunction::ArrayIsArray => Ok(Value::Bool(matches!(
                arguments.first(),
                Some(Value::Array(_))
            ))),
            NativeFunction::ObjectAssign => {
                let Some(target) = arguments.first().cloned() else {
                    return Ok(Value::Undefined);
                };
                for source in &arguments[1..] {
                    for (key, value) in own_property_entries(source)? {
                        self.set_property_value(target.clone(), key.as_str(), value)?;
                    }
                }
                Ok(target)
            }
            NativeFunction::ObjectCreate => {
                let prototype = arguments.first().cloned();
                Ok(self.create_object_with_prototype(prototype))
            }
            NativeFunction::ObjectKeys => {
                let Some(value) = arguments.first() else {
                    return Ok(new_array_value(Vec::new()));
                };
                let keys = own_property_names(value)?
                    .into_iter()
                    .map(Value::String)
                    .collect();
                Ok(new_array_value(keys))
            }
            NativeFunction::ObjectHasOwnProperty => {
                let property = first_string_argument(arguments)?;
                Ok(Value::Bool(
                    self.has_own_property(this_value, property.as_str()),
                ))
            }
            NativeFunction::ErrorConstructor => {
                let error = self.create_plain_object();
                let message = arguments
                    .first()
                    .cloned()
                    .unwrap_or(Value::String(String::new()));
                let _ = self.set_property_value(error.clone(), "message", message);
                let _ = self.set_property_value(
                    error.clone(),
                    "name",
                    Value::String("Error".to_owned()),
                );
                Ok(error)
            }
            NativeFunction::SetConstructor => {
                let set = Value::Set(Rc::new(RefCell::new(Vec::new())));
                if let Some(iterable) = arguments.first() {
                    for value in array_like_values(iterable) {
                        insert_set_value(&set, value);
                    }
                }
                Ok(set)
            }
            NativeFunction::RegExpConstructor => {
                let source = arguments.first().map(regex_source).unwrap_or_default();
                let flags = arguments
                    .get(1)
                    .map(Value::to_string_value)
                    .unwrap_or_default();
                Ok(Value::Regex(RegexValue { source, flags }))
            }
            NativeFunction::FunctionConstructor => {
                Ok(self.create_native_function(NativeFunction::Noop))
            }
            NativeFunction::FunctionCall => {
                let Some(this_arg) = arguments.first().cloned() else {
                    return Ok(Value::Undefined);
                };
                self.call_value(this_value, this_arg, &arguments[1..])
                    .map_err(|err| format!("Function.call: {err}"))
            }
            NativeFunction::FunctionApply => {
                let this_arg = arguments.first().cloned().unwrap_or(Value::Undefined);
                let applied_arguments = arguments
                    .get(1)
                    .map(array_like_values)
                    .unwrap_or_default();
                self.call_value(this_value, this_arg, &applied_arguments)
                    .map_err(|err| format!("Function.apply: {err}"))
            }
            NativeFunction::FunctionBind => {
                let this_arg = arguments.first().cloned().unwrap_or(Value::Undefined);
                Ok(self.create_bound_function(this_value, this_arg, arguments[1..].to_vec()))
            }
            NativeFunction::ArrayForEach => {
                self.call_array_prototype_method(this_value, "forEach", arguments)
            }
            NativeFunction::ArrayJoin => {
                self.call_array_prototype_method(this_value, "join", arguments)
            }
            NativeFunction::ArrayPush => {
                self.call_array_prototype_method(this_value, "push", arguments)
            }
            NativeFunction::ArrayMap => {
                self.call_array_prototype_method(this_value, "map", arguments)
            }
            NativeFunction::ArrayFilter => {
                self.call_array_prototype_method(this_value, "filter", arguments)
            }
            NativeFunction::ArraySlice => {
                self.call_array_prototype_method(this_value, "slice", arguments)
            }
            NativeFunction::ArraySort => {
                self.call_array_prototype_method(this_value, "sort", arguments)
            }
            NativeFunction::ArrayReduce => {
                self.call_array_prototype_method(this_value, "reduce", arguments)
            }
            NativeFunction::SetTimeout => self.call_timer(arguments),
            NativeFunction::ClearTimeout => Ok(Value::Undefined),
            NativeFunction::RequestIdleCallback => self.call_idle_callback(arguments),
            NativeFunction::CancelIdleCallback => Ok(Value::Undefined),
            NativeFunction::ReturnZero => Ok(Value::Number(0.0)),
            NativeFunction::DateNow => Ok(Value::Number(0.0)),
            NativeFunction::MathMax => Ok(Value::Number(
                arguments
                    .iter()
                    .map(Value::to_number_value)
                    .reduce(f64::max)
                    .unwrap_or(f64::NEG_INFINITY),
            )),
            NativeFunction::MathMin => Ok(Value::Number(
                arguments
                    .iter()
                    .map(Value::to_number_value)
                    .reduce(f64::min)
                    .unwrap_or(f64::INFINITY),
            )),
            NativeFunction::MathRound => Ok(Value::Number(
                arguments
                    .first()
                    .map(Value::to_number_value)
                    .unwrap_or(0.0)
                    .round(),
            )),
            NativeFunction::MathCeil => Ok(Value::Number(
                arguments
                    .first()
                    .map(Value::to_number_value)
                    .unwrap_or(0.0)
                    .ceil(),
            )),
            NativeFunction::EncodeURIComponent => Ok(Value::String(encode_uri_component(
                js_string_argument(arguments.first()).as_str(),
            ))),
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
                Ok(element_id
                    .map(Value::LiveElement)
                    .unwrap_or(Value::Undefined))
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
                    text.chars()
                        .map(|ch| Value::String(ch.to_string()))
                        .collect()
                } else {
                    text.split(separator.as_str())
                        .map(|part| Value::String(part.to_owned()))
                        .collect()
                };
                Ok(new_array_value(parts))
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
                Ok(Value::String(simple_replace(
                    text,
                    pattern,
                    replacement.as_str(),
                )))
            }
            "lastIndexOf" => {
                let needle = first_string_argument(arguments)?;
                let result = if needle.is_empty() {
                    text.len()
                } else {
                    text.rfind(needle.as_str()).unwrap_or(usize::MAX)
                };
                Ok(Value::Number(if result == usize::MAX {
                    -1.0
                } else {
                    result as f64
                }))
            }
            "slice" => {
                let chars: Vec<char> = text.chars().collect();
                let len = chars.len() as i32;
                let start = normalized_slice_index(arguments.first(), len);
                let end = normalized_slice_end(arguments.get(1), len);
                let value = if start >= end {
                    String::new()
                } else {
                    chars[start as usize..end as usize].iter().collect()
                };
                Ok(Value::String(value))
            }
            "charCodeAt" => {
                let index = normalized_slice_index(arguments.first(), text.chars().count() as i32);
                let value = text
                    .chars()
                    .nth(index as usize)
                    .map(|ch| ch as u32 as f64)
                    .unwrap_or(f64::NAN);
                Ok(Value::Number(value))
            }
            "startsWith" => {
                let prefix = first_string_argument(arguments)?;
                Ok(Value::Bool(text.starts_with(prefix.as_str())))
            }
            _ => Err(format!("Unsupported string method: {property}")),
        }
    }

    fn call_number_method(
        &mut self,
        number: f64,
        property: &str,
        arguments: &[Value],
    ) -> Result<Value, String> {
        match property {
            "toString" => {
                let radix = arguments
                    .first()
                    .map(Value::to_number_value)
                    .unwrap_or(10.0) as i32;
                Ok(Value::String(number_to_radix_string(number, radix)))
            }
            _ => Err(format!("Unsupported number method: {property}")),
        }
    }

    fn call_array_method(
        &mut self,
        items: ArrayRef,
        property: &str,
        arguments: &[Value],
    ) -> Result<Value, String> {
        if property == "length"
            || parse_array_index(property).is_some()
            || array_has_custom_property(&items, property)
        {
            let callee = lookup_array_member(&items, property)
                .unwrap_or_else(|| self.array_prototype_property(property));
            return self.call_value(callee, Value::Array(items), arguments);
        }

        match property {
            "shift" => Ok(items
                .borrow_mut()
                .items
                .drain(..1)
                .next()
                .unwrap_or(Value::Undefined)),
            "push" => {
                let mut items = items.borrow_mut();
                items.items.extend_from_slice(arguments);
                Ok(Value::Number(items.items.len() as f64))
            }
            "pop" => Ok(items.borrow_mut().items.pop().unwrap_or(Value::Undefined)),
            "includes" => {
                let needle = arguments.first().cloned().unwrap_or(Value::Undefined);
                Ok(Value::Bool(
                    items
                        .borrow()
                        .items
                        .iter()
                        .any(|value| strict_equals(value, &needle)),
                ))
            }
            "forEach" | "join" | "map" | "filter" | "slice" | "sort" | "reduce" => {
                self.call_array_prototype_method(Value::Array(items), property, arguments)
            }
            _ => {
                let callee = self.array_prototype_property(property);
                self.call_value(callee, Value::Array(items), arguments)
            }
        }
    }

    fn call_set_method(
        &mut self,
        items: SetRef,
        property: &str,
        arguments: &[Value],
    ) -> Result<Value, String> {
        match property {
            "add" => {
                if let Some(value) = arguments.first().cloned() {
                    insert_set_value(&Value::Set(items.clone()), value);
                }
                Ok(Value::Set(items))
            }
            "has" => {
                let needle = arguments.first().cloned().unwrap_or(Value::Undefined);
                Ok(Value::Bool(
                    items
                        .borrow()
                        .iter()
                        .any(|value| strict_equals(value, &needle)),
                ))
            }
            "delete" => {
                let needle = arguments.first().cloned().unwrap_or(Value::Undefined);
                let mut items = items.borrow_mut();
                if let Some(index) = items.iter().position(|value| strict_equals(value, &needle)) {
                    items.remove(index);
                    return Ok(Value::Bool(true));
                }
                Ok(Value::Bool(false))
            }
            "forEach" => {
                let callback = arguments.first().cloned().unwrap_or(Value::Undefined);
                let this_arg = arguments.get(1).cloned().unwrap_or(Value::Undefined);
                let snapshot = items.borrow().clone();
                let set_value = Value::Set(items);
                for value in snapshot {
                    let args = [value.clone(), value, set_value.clone()];
                    let callback_result =
                        self.call_value(callback.clone(), this_arg.clone(), &args);
                    let _ =
                        self.annotate_trace_error(callback_result, "Set.forEach callback")?;
                }
                Ok(Value::Undefined)
            }
            _ => Err(format!("Unsupported set method: {property}")),
        }
    }

    fn call_array_prototype_method(
        &mut self,
        this_value: Value,
        property: &str,
        arguments: &[Value],
    ) -> Result<Value, String> {
        match property {
            "forEach" => {
                let callback = arguments.first().cloned().unwrap_or(Value::Undefined);
                let this_arg = arguments.get(1).cloned().unwrap_or(Value::Undefined);
                let values = array_like_values(&this_value);
                for (index, item) in values.into_iter().enumerate() {
                    let args = [item, Value::Number(index as f64), this_value.clone()];
                    let callback_result =
                        self.call_value(callback.clone(), this_arg.clone(), &args);
                    let _ = self
                        .annotate_trace_error(callback_result, "Array.forEach callback")?;
                }
                Ok(Value::Undefined)
            }
            "join" => {
                let separator = arguments
                    .first()
                    .map(Value::to_string_value)
                    .unwrap_or_else(|| ",".to_owned());
                let joined = array_like_values(&this_value)
                    .into_iter()
                    .map(array_join_fragment)
                    .collect::<Vec<_>>()
                    .join(separator.as_str());
                Ok(Value::String(joined))
            }
            "push" => {
                if let Value::Array(items) = &this_value {
                    let mut items = items.borrow_mut();
                    items.items.extend_from_slice(arguments);
                    return Ok(Value::Number(items.items.len() as f64));
                }
                let length = array_like_values(&this_value).len() + arguments.len();
                Ok(Value::Number(length as f64))
            }
            "map" => {
                let callback = arguments.first().cloned().unwrap_or(Value::Undefined);
                let this_arg = arguments.get(1).cloned().unwrap_or(Value::Undefined);
                let values = array_like_values(&this_value);
                let mut mapped = Vec::with_capacity(values.len());
                for (index, item) in values.into_iter().enumerate() {
                    let args = [item, Value::Number(index as f64), this_value.clone()];
                    let callback_result =
                        self.call_value(callback.clone(), this_arg.clone(), &args);
                    mapped.push(self.annotate_trace_error(callback_result, "Array.map callback")?);
                }
                Ok(new_array_value(mapped))
            }
            "filter" => {
                let callback = arguments.first().cloned().unwrap_or(Value::Undefined);
                let this_arg = arguments.get(1).cloned().unwrap_or(Value::Undefined);
                let values = array_like_values(&this_value);
                let mut filtered = Vec::new();
                for (index, item) in values.into_iter().enumerate() {
                    let args = [
                        item.clone(),
                        Value::Number(index as f64),
                        this_value.clone(),
                    ];
                    let callback_result =
                        self.call_value(callback.clone(), this_arg.clone(), &args);
                    if self
                        .annotate_trace_error(callback_result, "Array.filter callback")?
                        .is_truthy()
                    {
                        filtered.push(item);
                    }
                }
                Ok(new_array_value(filtered))
            }
            "slice" => {
                let values = array_like_values(&this_value);
                let len = values.len() as i32;
                let start = normalized_slice_index(arguments.first(), len);
                let end = normalized_slice_end(arguments.get(1), len);
                let slice = if start >= end {
                    Vec::new()
                } else {
                    values[start as usize..end as usize].to_vec()
                };
                Ok(new_array_value(slice))
            }
            "sort" => {
                let comparator = arguments
                    .first()
                    .cloned()
                    .filter(|value| !matches!(value, Value::Undefined));
                let mut values = array_like_values(&this_value);
                sort_values(self, &mut values, comparator)?;
                if let Value::Array(items) = &this_value {
                    items.borrow_mut().items = values.clone();
                    return Ok(this_value);
                }
                Ok(new_array_value(values))
            }
            "reduce" => {
                let callback = arguments.first().cloned().unwrap_or(Value::Undefined);
                let values = array_like_values(&this_value);
                let mut iter = values.into_iter().enumerate();
                let mut accumulator = if let Some(initial) = arguments.get(1).cloned() {
                    initial
                } else if let Some((_, first)) = iter.next() {
                    first
                } else {
                    return Ok(Value::Undefined);
                };
                for (index, item) in iter {
                    let args = [
                        accumulator.clone(),
                        item,
                        Value::Number(index as f64),
                        this_value.clone(),
                    ];
                    let callback_result =
                        self.call_value(callback.clone(), Value::Undefined, &args);
                    accumulator =
                        self.annotate_trace_error(callback_result, "Array.reduce callback")?;
                }
                Ok(accumulator)
            }
            _ => Err(format!("Unsupported array prototype method: {property}")),
        }
    }

    fn array_prototype_property(&mut self, property: &str) -> Value {
        self.lookup_binding("Array")
            .and_then(|ctor| self.get_member_value(ctor, "prototype").ok())
            .and_then(|prototype| self.get_member_value(prototype, property).ok())
            .unwrap_or(Value::Undefined)
    }

    fn call_timer(&mut self, arguments: &[Value]) -> Result<Value, String> {
        let Some(callback) = arguments.first().cloned() else {
            return Ok(Value::Number(0.0));
        };
        let callback_result = self.call_value(callback, Value::Window, &arguments[2..]);
        let _ = self.annotate_trace_error(callback_result, "setTimeout callback")?;
        Ok(Value::Number(0.0))
    }

    fn call_idle_callback(&mut self, arguments: &[Value]) -> Result<Value, String> {
        let Some(callback) = arguments.first().cloned() else {
            return Ok(Value::Number(0.0));
        };
        let deadline = self.create_plain_object();
        let _ = self.set_property_value(deadline.clone(), "didTimeout", Value::Bool(false));
        let _ = self.set_property_value(
            deadline.clone(),
            "timeRemaining",
            self.create_native_function(NativeFunction::ReturnZero),
        );
        let callback_result = self.call_value(callback, Value::Window, &[deadline]);
        let _ = self.annotate_trace_error(callback_result, "requestIdleCallback callback")?;
        Ok(Value::Number(0.0))
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
                "localStorage" => Ok(self
                    .lookup_binding("localStorage")
                    .unwrap_or(Value::Undefined)),
                "performance" => Ok(self
                    .lookup_binding("performance")
                    .unwrap_or(Value::Undefined)),
                "console" => Ok(self.lookup_binding("console").unwrap_or(Value::Undefined)),
                "setTimeout" => Ok(self
                    .lookup_binding("setTimeout")
                    .unwrap_or(Value::Undefined)),
                "clearTimeout" => Ok(self
                    .lookup_binding("clearTimeout")
                    .unwrap_or(Value::Undefined)),
                "requestIdleCallback" => Ok(self
                    .lookup_binding("requestIdleCallback")
                    .unwrap_or(Value::Undefined)),
                "cancelIdleCallback" => Ok(self
                    .lookup_binding("cancelIdleCallback")
                    .unwrap_or(Value::Undefined)),
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
                    return Ok(Value::Number(items.borrow().items.len() as f64));
                }
                if let Some(index) = parse_array_index(property) {
                    return Ok(items
                        .borrow()
                        .items
                        .get(index)
                        .cloned()
                        .unwrap_or(Value::Undefined));
                }
                if let Some(value) = lookup_array_custom_property(&items, property) {
                    return Ok(value);
                }
                Ok(self.array_prototype_property(property))
            }
            Value::Set(items) => match property {
                "size" => Ok(Value::Number(items.borrow().len() as f64)),
                _ => Ok(Value::Undefined),
            },
            Value::Object(properties) => {
                Ok(lookup_object_property(&properties, property).unwrap_or(Value::Undefined))
            }
            Value::Function(function) => {
                if let Some(value) = function.properties.borrow().get(property).cloned() {
                    return Ok(value);
                }
                match property {
                    "call" => Ok(self.create_native_function(NativeFunction::FunctionCall)),
                    "apply" => Ok(self.create_native_function(NativeFunction::FunctionApply)),
                    "bind" => Ok(self.create_native_function(NativeFunction::FunctionBind)),
                    _ => Ok(Value::Undefined),
                }
            }
            Value::BoundFunction(function) => {
                if let Some(value) = function.properties.borrow().get(property).cloned() {
                    return Ok(value);
                }
                match property {
                    "call" => Ok(self.create_native_function(NativeFunction::FunctionCall)),
                    "apply" => Ok(self.create_native_function(NativeFunction::FunctionApply)),
                    "bind" => Ok(self.create_native_function(NativeFunction::FunctionBind)),
                    _ => Ok(Value::Undefined),
                }
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
            Value::Window => {
                matches!(
                    property,
                    "document"
                        | "window"
                        | "localStorage"
                        | "performance"
                        | "console"
                        | "setTimeout"
                        | "clearTimeout"
                        | "requestIdleCallback"
                        | "cancelIdleCallback"
                        | "encodeURIComponent"
                ) || self.lookup_binding(property).is_some()
            }
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
            Value::Array(items) => {
                property == "length"
                    || parse_array_index(property)
                        .is_some_and(|index| index < items.borrow().items.len())
                    || array_has_custom_property(&items, property)
                    || !matches!(self.array_prototype_property(property), Value::Undefined)
            }
            Value::Set(_) => matches!(property, "size" | "add" | "has" | "delete" | "forEach"),
            Value::NodeList(ids) | Value::JQueryCollection(ids) => {
                property == "length"
                    || parse_array_index(property).is_some_and(|index| index < ids.len())
            }
            Value::Object(properties) => lookup_object_property(&properties, property).is_some(),
            Value::Function(function) => {
                function.properties.borrow().contains_key(property)
                    || matches!(property, "call" | "apply" | "bind")
            }
            Value::BoundFunction(function) => {
                function.properties.borrow().contains_key(property)
                    || matches!(property, "call" | "apply" | "bind")
            }
            Value::String(text) => {
                property == "length"
                    || parse_array_index(property).is_some_and(|index| index < text.chars().count())
            }
            Value::Regex(regex) => {
                matches!(property, "source" | "flags")
                    || !regex.source.is_empty() && property == "constructor"
            }
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
            Value::Window => {
                self.global
                    .borrow_mut()
                    .bindings
                    .insert(property.to_owned(), value);
                Ok(())
            }
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
                    _ => Err(format!(
                        "Unsupported element property assignment: {property}"
                    )),
                }
            }
            Value::Object(properties) => {
                properties.borrow_mut().insert(property.to_owned(), value);
                Ok(())
            }
            Value::Function(function) => {
                function
                    .properties
                    .borrow_mut()
                    .insert(property.to_owned(), value);
                Ok(())
            }
            Value::BoundFunction(function) => {
                function
                    .properties
                    .borrow_mut()
                    .insert(property.to_owned(), value);
                Ok(())
            }
            Value::Array(items) => {
                if let Some(index) = parse_array_index(property) {
                    let mut items = items.borrow_mut();
                    if index >= items.items.len() {
                        items.items.resize(index + 1, Value::Undefined);
                    }
                    items.items[index] = value;
                    return Ok(());
                }
                items
                    .borrow_mut()
                    .properties
                    .insert(property.to_owned(), value);
                Ok(())
            }
            Value::Set(_) => Err(format!("Unsupported set property assignment: {property}")),
            _ => Err(format!(
                "Unsupported assignment through property {property}"
            )),
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
                    InsertPosition::Append => self
                        .document
                        .append_child_to(parent_id, Node::Element(element)),
                    InsertPosition::Prepend => self
                        .document
                        .prepend_child_to(parent_id, Node::Element(element)),
                };
                if inserted {
                    Ok(Value::LiveElement(inserted_id))
                } else {
                    Ok(Value::Undefined)
                }
            }
            Value::DetachedText(text) => {
                let _ = match position {
                    InsertPosition::Append => {
                        self.document.append_child_to(parent_id, Node::Text(text))
                    }
                    InsertPosition::Prepend => {
                        self.document.prepend_child_to(parent_id, Node::Text(text))
                    }
                };
                Ok(Value::Undefined)
            }
            Value::LiveElement(existing_id) => {
                let Some(existing) = self.document.find_element_by_node_id(existing_id).cloned()
                else {
                    return Ok(Value::Undefined);
                };
                let inserted = match position {
                    InsertPosition::Append => self
                        .document
                        .append_child_to(parent_id, Node::Element(existing)),
                    InsertPosition::Prepend => self
                        .document
                        .prepend_child_to(parent_id, Node::Element(existing)),
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

    fn replace_element(
        &mut self,
        node_id: ElementId,
        arguments: &[Value],
    ) -> Result<Value, String> {
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
                let Some(existing) = self.document.find_element_by_node_id(existing_id).cloned()
                else {
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
        self.global
            .borrow_mut()
            .bindings
            .insert(name.to_owned(), value);
    }

    fn declare_binding(&mut self, name: &str, value: Value) {
        self.scope
            .borrow_mut()
            .bindings
            .insert(name.to_owned(), value);
    }

    fn declare_pattern(&mut self, pattern: &BindingPattern, value: Value) -> Result<(), String> {
        match pattern {
            BindingPattern::Identifier(name) => {
                self.declare_binding(name, value);
                Ok(())
            }
            BindingPattern::Object(properties) => {
                for property in properties {
                    let member = self
                        .get_member_value(value.clone(), property.key.as_str())
                        .unwrap_or(Value::Undefined);
                    self.declare_binding(&property.binding, member);
                }
                Ok(())
            }
        }
    }

    fn lookup_binding(&self, name: &str) -> Option<Value> {
        lookup_scope(&self.scope, name)
    }

    fn assign_binding(&mut self, name: &str, value: Value) {
        if let Some(scope) = self.find_scope_containing(name) {
            scope.borrow_mut().bindings.insert(name.to_owned(), value);
            return;
        }
        self.global
            .borrow_mut()
            .bindings
            .insert(name.to_owned(), value);
    }

    fn find_scope_containing(&self, name: &str) -> Option<ScopeRef> {
        find_scope_containing(&self.scope, name)
    }

    fn create_plain_object(&self) -> Value {
        self.create_object_with_prototype(None)
    }

    fn create_object_with_prototype(&self, prototype: Option<Value>) -> Value {
        let mut properties = HashMap::new();
        if let Some(prototype) = prototype {
            properties.insert("__proto__".to_owned(), prototype);
        }
        Value::Object(Rc::new(RefCell::new(properties)))
    }

    fn create_native_function(&self, kind: NativeFunction) -> Value {
        Value::Function(Rc::new(FunctionValue {
            kind: FunctionKind::Native(kind),
            properties: Rc::new(RefCell::new(HashMap::new())),
        }))
    }

    fn create_user_function(
        &self,
        params: Vec<Parameter>,
        body: Vec<Statement>,
        lexical_this: bool,
        constructible: bool,
    ) -> Value {
        let function = Value::Function(Rc::new(FunctionValue {
            kind: FunctionKind::User(UserFunction {
                params,
                body,
                env: self.scope.clone(),
                lexical_this,
                constructible,
            }),
            properties: Rc::new(RefCell::new(HashMap::new())),
        }));
        if constructible {
            let prototype = self.create_plain_object();
            let _ = self.set_property_value(prototype.clone(), "constructor", function.clone());
            let _ = self.set_property_value(function.clone(), "prototype", prototype);
        }
        function
    }

    fn create_bound_function(
        &self,
        target: Value,
        this_value: Value,
        preset_arguments: Vec<Value>,
    ) -> Value {
        Value::BoundFunction(Rc::new(BoundFunctionValue {
            target,
            this_value,
            preset_arguments,
            properties: Rc::new(RefCell::new(HashMap::new())),
        }))
    }

    fn has_own_property(&self, object: Value, property: &str) -> bool {
        match object {
            Value::Object(properties) => properties.borrow().contains_key(property),
            Value::Function(function) => function.properties.borrow().contains_key(property),
            Value::BoundFunction(function) => function.properties.borrow().contains_key(property),
            Value::Array(items) => {
                property == "length"
                    || parse_array_index(property)
                        .is_some_and(|index| index < items.borrow().items.len())
                    || array_has_custom_property(&items, property)
            }
            Value::Set(_) => property == "size",
            Value::NodeList(ids) | Value::JQueryCollection(ids) => {
                property == "length"
                    || parse_array_index(property).is_some_and(|index| index < ids.len())
            }
            Value::String(text) => {
                property == "length"
                    || parse_array_index(property).is_some_and(|index| index < text.chars().count())
            }
            Value::Regex(_) => matches!(property, "source" | "flags"),
            Value::LiveElement(_) => matches!(
                property,
                "classList" | "className" | "textContent" | "id" | "checked" | "disabled" | "value"
            ),
            Value::Window => {
                matches!(
                    property,
                    "document"
                        | "window"
                        | "localStorage"
                        | "performance"
                        | "console"
                        | "setTimeout"
                        | "clearTimeout"
                        | "requestIdleCallback"
                        | "cancelIdleCallback"
                        | "encodeURIComponent"
                ) || self.lookup_binding(property).is_some()
            }
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
            _ => false,
        }
    }

    fn delete_property(&mut self, object: Value, property: &str) -> bool {
        match object {
            Value::Object(properties) => {
                properties.borrow_mut().remove(property);
                true
            }
            Value::Function(function) => {
                function.properties.borrow_mut().remove(property);
                true
            }
            Value::BoundFunction(function) => {
                function.properties.borrow_mut().remove(property);
                true
            }
            Value::Array(items) => {
                if let Some(index) = parse_array_index(property)
                    && let Some(slot) = items.borrow_mut().items.get_mut(index)
                {
                    *slot = Value::Undefined;
                }
                items.borrow_mut().properties.remove(property);
                true
            }
            _ => true,
        }
    }

    fn error_to_string(&mut self, value: &Value) -> String {
        if let Value::Object(_) = value {
            let name = self
                .get_member_value(value.clone(), "name")
                .unwrap_or(Value::Undefined)
                .to_string_value();
            let message = self
                .get_member_value(value.clone(), "message")
                .unwrap_or(Value::Undefined)
                .to_string_value();
            if !name.is_empty() && !message.is_empty() {
                return format!("{name}: {message}");
            }
            if !message.is_empty() {
                return message;
            }
        }
        value.to_string_value()
    }

    fn annotate_trace_error<T>(&self, result: Result<T, String>, context: &str) -> Result<T, String> {
        result.map_err(|err| trace_annotate_error(err, context))
    }

    fn instanceof_value(&self, left: &Value, right: &Value) -> bool {
        if self
            .lookup_binding("HTMLElement")
            .is_some_and(|value| strict_equals(&value, right))
            || self
                .lookup_binding("Element")
                .is_some_and(|value| strict_equals(&value, right))
        {
            return matches!(left, Value::LiveElement(_) | Value::DetachedElement(_));
        }

        if self
            .lookup_binding("Array")
            .is_some_and(|value| strict_equals(&value, right))
        {
            return matches!(left, Value::Array(_));
        }
        if self
            .lookup_binding("Set")
            .is_some_and(|value| strict_equals(&value, right))
        {
            return matches!(left, Value::Set(_));
        }
        if self
            .lookup_binding("RegExp")
            .is_some_and(|value| strict_equals(&value, right))
        {
            return matches!(left, Value::Regex(_));
        }
        if self
            .lookup_binding("Function")
            .is_some_and(|value| strict_equals(&value, right))
        {
            return matches!(left, Value::Function(_) | Value::BoundFunction(_));
        }

        let prototype = match right {
            Value::Function(function) => function.properties.borrow().get("prototype").cloned(),
            Value::BoundFunction(function) => function.properties.borrow().get("prototype").cloned(),
            _ => None,
        };

        let Some(prototype) = prototype else {
            return false;
        };

        let mut current = match left {
            Value::Object(properties) => properties.borrow().get("__proto__").cloned(),
            _ => None,
        };

        while let Some(value) = current {
            if strict_equals(&value, &prototype) {
                return true;
            }
            current = match value {
                Value::Object(properties) => properties.borrow().get("__proto__").cloned(),
                _ => None,
            };
        }

        false
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
                function
                    .properties
                    .borrow_mut()
                    .insert(property.to_owned(), value);
                Ok(())
            }
            Value::BoundFunction(function) => {
                function
                    .properties
                    .borrow_mut()
                    .insert(property.to_owned(), value);
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
    fn debug_label(&self) -> &'static str {
        match self {
            Value::Undefined => "undefined",
            Value::Null => "null",
            Value::String(_) => "string",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::Regex(_) => "regexp",
            Value::Object(_) => "object",
            Value::Array(_) => "array",
            Value::Set(_) => "set",
            Value::Function(_) => "function",
            Value::BoundFunction(_) => "bound-function",
            Value::Window => "window",
            Value::Document => "document",
            Value::LiveElement(_) => "element",
            Value::DetachedElement(_) => "detached-element",
            Value::DetachedText(_) => "text-node",
            Value::ClassList(_) => "class-list",
            Value::NodeList(_) => "node-list",
            Value::JQueryCollection(_) => "jquery-collection",
        }
    }

    fn trace_label(&self) -> String {
        match self {
            Value::Function(function) => match &function.kind {
                FunctionKind::Native(kind) => format!("native {}", kind.debug_name()),
                FunctionKind::User(_) => "function".to_owned(),
            },
            Value::BoundFunction(function) => {
                format!("bound {}", function.target.trace_label())
            }
            _ => self.debug_label().to_owned(),
        }
    }

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
            Value::BoundFunction(_) => "function".to_owned(),
            _ => "object".to_owned(),
        }
    }
}

impl NativeFunction {
    fn debug_name(&self) -> &'static str {
        match self {
            NativeFunction::Dollar => "$",
            NativeFunction::ArrayIsArray => "Array.isArray",
            NativeFunction::ObjectAssign => "Object.assign",
            NativeFunction::ObjectCreate => "Object.create",
            NativeFunction::ObjectKeys => "Object.keys",
            NativeFunction::ObjectHasOwnProperty => "Object.hasOwnProperty",
            NativeFunction::ErrorConstructor => "Error",
            NativeFunction::SetConstructor => "Set",
            NativeFunction::RegExpConstructor => "RegExp",
            NativeFunction::FunctionConstructor => "Function",
            NativeFunction::FunctionCall => "Function.call",
            NativeFunction::FunctionApply => "Function.apply",
            NativeFunction::FunctionBind => "Function.bind",
            NativeFunction::ArrayForEach => "Array.forEach",
            NativeFunction::ArrayJoin => "Array.join",
            NativeFunction::ArrayPush => "Array.push",
            NativeFunction::ArrayMap => "Array.map",
            NativeFunction::ArrayFilter => "Array.filter",
            NativeFunction::ArraySlice => "Array.slice",
            NativeFunction::ArraySort => "Array.sort",
            NativeFunction::ArrayReduce => "Array.reduce",
            NativeFunction::SetTimeout => "setTimeout",
            NativeFunction::ClearTimeout => "clearTimeout",
            NativeFunction::RequestIdleCallback => "requestIdleCallback",
            NativeFunction::CancelIdleCallback => "cancelIdleCallback",
            NativeFunction::ReturnZero => "returnZero",
            NativeFunction::DateNow => "Date.now",
            NativeFunction::MathMax => "Math.max",
            NativeFunction::MathMin => "Math.min",
            NativeFunction::MathRound => "Math.round",
            NativeFunction::MathCeil => "Math.ceil",
            NativeFunction::EncodeURIComponent => "encodeURIComponent",
            NativeFunction::Noop => "noop",
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

fn own_property_names(value: &Value) -> Result<Vec<String>, String> {
    match value {
        Value::Object(properties) => {
            let mut keys: Vec<String> = properties
                .borrow()
                .keys()
                .filter(|key| key.as_str() != "__proto__")
                .cloned()
                .collect();
            keys.sort();
            Ok(keys)
        }
        Value::Array(items) => {
            let items = items.borrow();
            let mut keys: Vec<String> =
                (0..items.items.len()).map(|index| index.to_string()).collect();
            let mut properties: Vec<String> = items.properties.keys().cloned().collect();
            properties.sort();
            keys.extend(properties);
            Ok(keys)
        }
        Value::Set(_) => Ok(vec!["size".to_owned()]),
        _ => Err("Unsupported Object.keys target".to_owned()),
    }
}

fn own_property_entries(value: &Value) -> Result<Vec<(String, Value)>, String> {
    match value {
        Value::Object(properties) => Ok(properties
            .borrow()
            .iter()
            .filter(|(key, _)| key.as_str() != "__proto__")
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()),
        Value::Array(items) => {
            let items = items.borrow();
            let mut entries: Vec<(String, Value)> = items
                .items
                .iter()
                .enumerate()
                .map(|(index, value)| (index.to_string(), value.clone()))
                .collect();
            let mut properties: Vec<(String, Value)> = items
                .properties
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();
            properties.sort_by(|left, right| left.0.cmp(&right.0));
            entries.extend(properties);
            Ok(entries)
        }
        _ => Err("Unsupported Object.assign source".to_owned()),
    }
}

fn object_key_to_string(key: &ObjectKey) -> String {
    match key {
        ObjectKey::Identifier(value) | ObjectKey::String(value) => value.clone(),
        ObjectKey::Number(value) => number_to_string(*value),
    }
}

fn new_array_ref(items: Vec<Value>) -> ArrayRef {
    Rc::new(RefCell::new(ArrayValue {
        items,
        properties: HashMap::new(),
    }))
}

fn new_array_value(items: Vec<Value>) -> Value {
    Value::Array(new_array_ref(items))
}

fn lookup_array_member(items: &ArrayRef, property: &str) -> Option<Value> {
    if property == "length" {
        return Some(Value::Number(items.borrow().items.len() as f64));
    }
    if let Some(index) = parse_array_index(property) {
        return Some(
            items.borrow()
                .items
                .get(index)
                .cloned()
                .unwrap_or(Value::Undefined),
        );
    }
    lookup_array_custom_property(items, property)
}

fn lookup_array_custom_property(items: &ArrayRef, property: &str) -> Option<Value> {
    items.borrow().properties.get(property).cloned()
}

fn array_has_custom_property(items: &ArrayRef, property: &str) -> bool {
    items.borrow().properties.contains_key(property)
}

fn array_like_values(value: &Value) -> Vec<Value> {
    match value {
        Value::Array(items) => items.borrow().items.clone(),
        Value::Set(items) => items.borrow().clone(),
        Value::NodeList(ids) | Value::JQueryCollection(ids) => {
            ids.iter().copied().map(Value::LiveElement).collect()
        }
        Value::String(text) => text
            .chars()
            .map(|ch| Value::String(ch.to_string()))
            .collect(),
        _ => Vec::new(),
    }
}

fn normalized_slice_index(argument: Option<&Value>, len: i32) -> i32 {
    let Some(argument) = argument else {
        return 0;
    };
    let index = argument.to_number_value() as i32;
    if index < 0 {
        (len + index).max(0).min(len)
    } else {
        index.min(len)
    }
}

fn normalized_slice_end(argument: Option<&Value>, len: i32) -> i32 {
    let Some(argument) = argument else {
        return len;
    };
    let index = argument.to_number_value() as i32;
    if index < 0 {
        (len + index).max(0).min(len)
    } else {
        index.min(len)
    }
}

fn parse_array_index(property: &str) -> Option<usize> {
    if property.is_empty() || !property.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    property.parse::<usize>().ok()
}

fn sort_values(
    executor: &mut Executor<'_>,
    values: &mut [Value],
    comparator: Option<Value>,
) -> Result<(), String> {
    let len = values.len();
    for i in 0..len {
        for j in 0..len.saturating_sub(i + 1) {
            let should_swap = if let Some(callback) = &comparator {
                let result = executor.call_value(
                    callback.clone(),
                    Value::Undefined,
                    &[values[j].clone(), values[j + 1].clone()],
                )?;
                result.to_number_value() > 0.0
            } else {
                compare_values(&values[j], &values[j + 1]) > 0
            };
            if should_swap {
                values.swap(j, j + 1);
            }
        }
    }
    Ok(())
}

fn insert_set_value(set: &Value, value: Value) {
    let Value::Set(items) = set else {
        return;
    };
    if !items
        .borrow()
        .iter()
        .any(|existing| strict_equals(existing, &value))
    {
        items.borrow_mut().push(value);
    }
}

fn number_to_string(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
}

fn number_to_radix_string(value: f64, radix: i32) -> String {
    if radix == 10 || !(2..=36).contains(&radix) || !value.is_finite() || value.fract() != 0.0 {
        return number_to_string(value);
    }

    let negative = value.is_sign_negative();
    let mut number = value.abs() as u64;
    if number == 0 {
        return "0".to_owned();
    }

    let mut digits = Vec::new();
    let radix = radix as u64;
    while number > 0 {
        let digit = (number % radix) as u8;
        digits.push(if digit < 10 {
            (b'0' + digit) as char
        } else {
            (b'a' + digit - 10) as char
        });
        number /= radix;
    }
    if negative {
        digits.push('-');
    }
    digits.iter().rev().collect()
}

fn compare_values(left: &Value, right: &Value) -> i32 {
    match (left, right) {
        (Value::String(left), Value::String(right)) => match left.cmp(right) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        },
        _ => {
            let left = left.to_number_value();
            let right = right.to_number_value();
            if left < right {
                -1
            } else if left > right {
                1
            } else {
                0
            }
        }
    }
}

fn to_js_uint32(value: f64) -> u32 {
    if !value.is_finite() || value == 0.0 {
        return 0;
    }
    value.trunc().rem_euclid(4294967296.0) as u32
}

fn to_js_int32(value: f64) -> i32 {
    to_js_uint32(value) as i32
}

fn shift_count(value: f64) -> u32 {
    to_js_uint32(value) & 31
}

fn lookup_object_property(properties: &ObjectRef, property: &str) -> Option<Value> {
    if let Some(value) = properties.borrow().get(property).cloned() {
        return Some(value);
    }
    let prototype = properties.borrow().get("__proto__").cloned();
    match prototype {
        Some(Value::Object(prototype)) => lookup_object_property(&prototype, property),
        Some(Value::Function(function)) => function.properties.borrow().get(property).cloned(),
        Some(Value::BoundFunction(function)) => function.properties.borrow().get(property).cloned(),
        _ => None,
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
        (Value::Set(left), Value::Set(right)) => Rc::ptr_eq(left, right),
        (Value::Object(left), Value::Object(right)) => Rc::ptr_eq(left, right),
        (Value::Function(left), Value::Function(right)) => Rc::ptr_eq(left, right),
        (Value::BoundFunction(left), Value::BoundFunction(right)) => Rc::ptr_eq(left, right),
        (Value::JQueryCollection(left), Value::JQueryCollection(right)) => left == right,
        (Value::NodeList(left), Value::NodeList(right)) => left == right,
        _ => false,
    }
}

fn loose_equals(left: &Value, right: &Value) -> bool {
    if matches!(
        (left, right),
        (Value::Null, Value::Undefined) | (Value::Undefined, Value::Null)
    ) {
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
                new_array_value(vec![Value::String(needle)])
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
                return new_array_value(vec![value]);
            }
            if !regex.source.chars().any(|ch| {
                matches!(
                    ch,
                    '[' | ']' | '(' | ')' | '?' | '+' | '*' | '|' | '^' | '$' | '\\'
                )
            }) && text.contains(regex.source.as_str())
            {
                return new_array_value(vec![Value::String(regex.source)]);
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

fn should_parse_regex(previous: Option<&Token>) -> bool {
    !matches!(
        previous,
        Some(
            Token::Identifier(_)
                | Token::String(_)
                | Token::Number(_)
                | Token::Regex { .. }
                | Token::TemplateLiteral(_)
                | Token::KeywordTrue
                | Token::KeywordFalse
                | Token::KeywordNull
                | Token::KeywordThis
                | Token::RightParen
                | Token::RightBracket
                | Token::RightBrace
                | Token::PlusPlus
                | Token::MinusMinus
        )
    )
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
        "src" => {
            let next = value.to_string_value();
            if next.is_empty() {
                element.attributes.remove("src");
            } else {
                element.attributes.insert("src".to_owned(), next);
            }
            true
        }
        "onload" | "onerror" => true,
        _ => false,
    }
}

fn expression_debug_label(expression: &Expression) -> String {
    match expression {
        Expression::Identifier(name) => name.clone(),
        Expression::This => "this".to_owned(),
        Expression::Member { object, property } => {
            format!("{}.{}", expression_debug_label(object), property)
        }
        Expression::ComputedMember { object, property } => {
            format!(
                "{}[{}]",
                expression_debug_label(object),
                computed_property_debug_label(property)
            )
        }
        Expression::Call { callee, .. } => format!("{}(...)", expression_debug_label(callee)),
        Expression::New { callee, .. } => format!("new {}", expression_debug_label(callee)),
        Expression::Sequence(_) => "sequence".to_owned(),
        Expression::Function { .. } => "function".to_owned(),
        Expression::TemplateLiteral(_) => "template".to_owned(),
        Expression::Array(_) => "array".to_owned(),
        Expression::Object(_) => "object".to_owned(),
        Expression::Assignment { .. } => "assignment".to_owned(),
        Expression::Update { .. } => "update".to_owned(),
        Expression::Unary { .. } => "unary".to_owned(),
        Expression::Conditional { .. } => "conditional".to_owned(),
        Expression::Binary { .. } => "binary".to_owned(),
        Expression::String(_) => "string".to_owned(),
        Expression::Number(_) => "number".to_owned(),
        Expression::Bool(_) => "bool".to_owned(),
        Expression::Null => "null".to_owned(),
        Expression::Regex { .. } => "regex".to_owned(),
    }
}

fn computed_property_debug_label(expression: &Expression) -> String {
    match expression {
        Expression::Identifier(name) => name.clone(),
        Expression::String(value) => format!("{value:?}"),
        Expression::Number(value) => number_to_string(*value),
        _ => "...".to_owned(),
    }
}

fn array_join_fragment(value: Value) -> String {
    match value {
        Value::Undefined | Value::Null => String::new(),
        other => other.to_string_value(),
    }
}

fn js_string_argument(value: Option<&Value>) -> String {
    match value {
        Some(Value::Undefined) | None => "undefined".to_owned(),
        Some(Value::Null) => "null".to_owned(),
        Some(Value::String(value)) => value.clone(),
        Some(Value::Bool(true)) => "true".to_owned(),
        Some(Value::Bool(false)) => "false".to_owned(),
        Some(Value::Number(value)) => number_to_string(*value),
        Some(other) => other.to_string_value(),
    }
}

fn encode_uri_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        if matches!(
            byte,
            b'A'..=b'Z'
                | b'a'..=b'z'
                | b'0'..=b'9'
                | b'-'
                | b'_'
                | b'.'
                | b'!'
                | b'~'
                | b'*'
                | b'\''
                | b'('
                | b')'
        ) {
            out.push(byte as char);
        } else {
            out.push('%');
            out.push(hex_digit(byte >> 4));
            out.push(hex_digit(byte & 0x0f));
        }
    }
    out
}

fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'A' + (value - 10)) as char,
        _ => unreachable!("hex digit out of range"),
    }
}

fn trace_annotate_error(err: String, context: &str) -> String {
    if std::env::var_os("OAB_TRACE_JS_ERRORS").is_some() {
        format!("{err} [via {context}]")
    } else {
        err
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
        assert_eq!(
            greeting.children,
            vec![Node::Text("Hello World!".to_owned())]
        );
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
        assert_eq!(
            greeting.children,
            vec![Node::Text("Hello from JS".to_owned())]
        );
    }

    #[test]
    fn executes_wikipedia_client_bootstrap_without_cookie() {
        let mut document =
            crate::html::parse_document(r#"<html class="client-nojs"><body></body></html>"#);

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

    #[test]
    fn supports_prototype_backed_constructors_and_startup_style_loops() {
        let mut document = crate::html::parse_document(r#"<html><body></body></html>"#);

        execute(
            &mut document,
            r#"
            function Map() {
                this.values = Object.create(null);
            }
            Map.prototype = {
                constructor: Map,
                set: function(selection, value) {
                    this.values[selection] = value;
                    return true;
                },
                get: function(selection, fallback) {
                    if (selection in this.values) {
                        return this.values[selection];
                    }
                    return fallback;
                }
            };

            var queue = [1, 2, 3], sum = 0;
            while (queue.length > 0) {
                sum += queue.shift();
            }

            for (var i = 0; i < 2; i++) {
                sum++;
            }

            var map = new Map();
            map.set("client-js", "ready");
            document.documentElement.textContent = map.get("client-js", "missing") + " " + sum;
            "#,
        )
        .expect("script should execute");

        let html = document
            .find_first_element_by_name("html")
            .expect("missing html element");
        assert_eq!(collect_text_content(html), "ready 8");
    }

    #[test]
    fn supports_startup_style_array_and_string_helpers() {
        let mut document = crate::html::parse_document(r#"<html><body></body></html>"#);

        execute(
            &mut document,
            r#"
            var prefixes = ["alpha"];
            prefixes.push("beta");
            var last = prefixes.pop();
            var containsAlpha = prefixes.includes("alpha");
            var title = "@import demo";
            if (containsAlpha && title.startsWith("@import")) {
                document.body.textContent = last;
            }
            "#,
        )
        .expect("script should execute");

        let body = document
            .find_first_element_by_name("body")
            .expect("missing body element");
        assert_eq!(collect_text_content(body), "beta");
    }

    #[test]
    fn supports_array_join_for_mediawiki_startup() {
        let mut document = crate::html::parse_document(r#"<html><body></body></html>"#);

        execute(
            &mut document,
            r#"
            var labels = ["alpha", null, undefined, "omega"];
            document.body.textContent =
                labels.join("|") + ":" + Array.prototype.join.call(["x", "y"], ".");
            "#,
        )
        .expect("script should execute");

        let body = document
            .find_first_element_by_name("body")
            .expect("missing body element");
        assert_eq!(collect_text_content(body), "alpha|||omega:x.y");
    }

    #[test]
    fn supports_array_push_apply_for_mediawiki_startup() {
        let mut document = crate::html::parse_document(r#"<html><body></body></html>"#);

        execute(
            &mut document,
            r#"
            var values = ["a"];
            Array.prototype.push.apply(values, ["b", "c"]);
            document.body.textContent = values.join("|");
            "#,
        )
        .expect("script should execute");

        let body = document
            .find_first_element_by_name("body")
            .expect("missing body element");
        assert_eq!(collect_text_content(body), "a|b|c");
    }

    #[test]
    fn supports_delete_set_and_startup_timers() {
        let mut document = crate::html::parse_document(r#"<html><body></body></html>"#);

        execute(
            &mut document,
            r#"
            var prefs = { theme: "dark", width: "wide" };
            var deletedTheme = delete prefs["theme"];
            var values = new Set([1, 2, 2]);
            values.add(3);
            values.delete(2);
            var sum = 0;
            values.forEach(function(value) {
                sum += value;
            });
            var idleState = "pending";
            requestIdleCallback(function(deadline) {
                if (deadline.didTimeout === false) {
                    idleState = "idle";
                }
            });
            setTimeout(function() {
                document.body.textContent =
                    deletedTheme + ":" + values.has(1) + ":" + values.has(2) + ":" + sum + ":" + values.size + ":" + idleState;
            }, 0);
            "#,
        )
        .expect("script should execute");

        let body = document
            .find_first_element_by_name("body")
            .expect("missing body element");
        assert_eq!(collect_text_content(body), "true:true:false:4:2:idle");
    }

    #[test]
    fn supports_array_prototype_helpers_and_object_runtime_methods() {
        let mut document = crate::html::parse_document(
            r#"<html><body><div class="item" id="a"></div><div class="item" id="b"></div></body></html>"#,
        );

        execute(
            &mut document,
            r#"
            var ids = [];
            Array.prototype.forEach.call(document.querySelectorAll(".item"), function(node) {
                ids.push(node.id);
            });
            var mapped = ids.map(function(id) {
                return id + id;
            });
            var filtered = mapped.filter(function(id) {
                return id !== "bb";
            });
            var sliced = filtered.slice(0, 1);
            var sorted = [3, 1, 2];
            sorted.sort(function(a, b) {
                return a - b;
            });
            var reduced = sorted.reduce(function(acc, value) {
                return acc * 10 + value;
            }, 0);
            var config = {};
            Object.assign(config, { ready: true, items: sliced.length });
            var keys = Object.keys(config);
            document.body.textContent =
                sliced[0] + ":" + reduced + ":" + config.ready + ":" + config.items + ":" + keys.length;
            "#,
        )
        .expect("script should execute");

        let body = document
            .find_first_element_by_name("body")
            .expect("missing body element");
        assert_eq!(collect_text_content(body), "aa:123:true:1:2");
    }

    #[test]
    fn supports_bundle_style_syntax_features() {
        let mut document = crate::html::parse_document(r#"<html><body></body></html>"#);

        execute(
            &mut document,
            r#"
            const suffixes = ["b", "c"];
            const labelFor = (prefix = "a") => `${prefix}${suffixes[0]}`;
            const labels = ["a", ...suffixes];
            const payload = { label: labelFor(), labels };
            const { label, labels: copiedLabels } = payload;
            let joined = "";
            for (const part of copiedLabels) {
                joined += part;
            }
            let keySummary = "";
            for (const key in { first: 1, second: 2 }) {
                keySummary += key[0];
            }
            let thrown = "";
            try {
                throw new Error("boom");
            } catch (error) {
                thrown = error;
            }
            document.body.textContent =
                (document.body instanceof HTMLElement)
                    ? `${label}:${joined}:${keySummary}:${+true}:${thrown}`
                    : "bad";
            "#,
        )
        .expect("script should execute");

        let body = document
            .find_first_element_by_name("body")
            .expect("missing body element");
        assert_eq!(collect_text_content(body), "ab:abc:fs:1:Error: boom");
    }

    #[test]
    fn parses_captured_riki_bundle_syntax() {
        let source =
            std::fs::read_to_string("tmp/riki-load.php.js").expect("missing captured bundle");
        let tokens = Lexer::new(source.as_str())
            .tokenize()
            .expect("bundle should tokenize");
        let mut parser = Parser::new(tokens);
        while !parser.is_at_end() {
            while parser.match_token(&Token::Semicolon) {}
            if parser.is_at_end() {
                break;
            }
            if let Err(err) = parser.parse_statement() {
                let start = parser.cursor.saturating_sub(5);
                let end = (parser.cursor + 5).min(parser.tokens.len());
                panic!(
                    "bundle should parse: {err}; around tokens {:?}",
                    &parser.tokens[start..end]
                );
            }
        }
    }

    #[test]
    fn reports_missing_member_calls_with_receiver_context() {
        let mut document = crate::html::parse_document(r#"<html><body></body></html>"#);

        let err = execute(&mut document, r#"({}).closest("main");"#)
            .expect_err("script should fail");

        assert_eq!(err, "Unsupported member call: object.closest");
    }

    #[test]
    fn supports_shift_operators_needed_by_mediawiki_startup() {
        let mut document = crate::html::parse_document(r#"<html><body></body></html>"#);

        execute(
            &mut document,
            r#"
            var left = 1 << 5;
            var signed = -16 >> 2;
            var unsigned = -1 >>> 0;
            document.body.textContent = left + ":" + signed + ":" + unsigned;
            "#,
        )
        .expect("script should execute");

        let body = document
            .find_first_element_by_name("body")
            .expect("missing body element");
        assert_eq!(collect_text_content(body), "32:-4:4294967295");
    }

    #[test]
    fn supports_for_in_with_existing_binding_targets() {
        let mut document = crate::html::parse_document(r#"<html><body></body></html>"#);

        execute(
            &mut document,
            r#"
            var source = { alpha: 1, beta: 2 };
            var key = "";
            for (key in source) {
                document.body.textContent += key[0];
            }
            "#,
        )
        .expect("script should execute");

        let body = document
            .find_first_element_by_name("body")
            .expect("missing body element");
        assert_eq!(collect_text_content(body), "ab");
    }

    #[test]
    fn supports_parenthesized_sequence_expressions() {
        let mut document = crate::html::parse_document(r#"<html><body></body></html>"#);

        execute(
            &mut document,
            r#"
            document.body.textContent = (1, 2, 3);
            "#,
        )
        .expect("script should execute");

        let body = document
            .find_first_element_by_name("body")
            .expect("missing body element");
        assert_eq!(collect_text_content(body), "3");
    }

    #[test]
    fn supports_window_property_assignment_for_globals() {
        let mut document = crate::html::parse_document(r#"<html><body></body></html>"#);

        execute(
            &mut document,
            r#"
            window.mediaWiki = { ready: true };
            document.body.textContent = window.mediaWiki.ready;
            "#,
        )
        .expect("script should execute");

        let body = document
            .find_first_element_by_name("body")
            .expect("missing body element");
        assert_eq!(collect_text_content(body), "true");
    }

    #[test]
    fn supports_function_apply() {
        let mut document = crate::html::parse_document(r#"<html><body></body></html>"#);

        execute(
            &mut document,
            r#"
            function describe(a, b) {
                document.body.textContent = this.label + ":" + a + ":" + b;
            }
            describe.apply({ label: "ctx" }, [1, 2]);
            "#,
        )
        .expect("script should execute");

        let body = document
            .find_first_element_by_name("body")
            .expect("missing body element");
        assert_eq!(collect_text_content(body), "ctx:1:2");
    }

    #[test]
    fn supports_string_last_index_of() {
        let mut document = crate::html::parse_document(r#"<html><body></body></html>"#);

        execute(
            &mut document,
            r#"
            document.body.textContent = "alpha@beta@gamma".lastIndexOf("@");
            "#,
        )
        .expect("script should execute");

        let body = document
            .find_first_element_by_name("body")
            .expect("missing body element");
        assert_eq!(collect_text_content(body), "10");
    }

    #[test]
    fn supports_math_and_date_statics_used_by_mediawiki() {
        let mut document = crate::html::parse_document(r#"<html><body></body></html>"#);

        execute(
            &mut document,
            r#"
            document.body.textContent =
                Math.max(1, 5, 3) + ":" +
                Math.min(1, 5, 3) + ":" +
                Math.round(2.4) + ":" +
                Math.ceil(2.1) + ":" +
                Date.now();
            "#,
        )
        .expect("script should execute");

        let body = document
            .find_first_element_by_name("body")
            .expect("missing body element");
        assert_eq!(collect_text_content(body), "5:1:2:3:0");
    }

    #[test]
    fn supports_encode_uri_component_for_mediawiki_startup() {
        let mut document = crate::html::parse_document(r#"<html><body></body></html>"#);

        execute(
            &mut document,
            r#"
            document.body.textContent = encodeURIComponent("lang=en&title=Riki LeCotey");
            "#,
        )
        .expect("script should execute");

        let body = document
            .find_first_element_by_name("body")
            .expect("missing body element");
        assert_eq!(
            collect_text_content(body),
            "lang%3Den%26title%3DRiki%20LeCotey"
        );
    }

    #[test]
    fn supports_script_src_assignment_before_append() {
        let mut document = crate::html::parse_document(r#"<html><head></head><body></body></html>"#);

        execute(
            &mut document,
            r#"
            var script = document.createElement("script");
            script.src = "/w/load.php?modules=startup";
            document.head.appendChild(script);
            "#,
        )
        .expect("script should execute");

        let head = document
            .find_first_element_by_name("head")
            .expect("missing head element");
        let inserted = head
            .children
            .iter()
            .find_map(|child| match child {
                Node::Element(element) if element.name == "script" => Some(element),
                _ => None,
            })
            .expect("missing inserted script");
        assert_eq!(
            inserted.attributes.get("src"),
            Some("/w/load.php?modules=startup")
        );
    }

    #[test]
    fn supports_script_event_handler_assignment_before_append() {
        let mut document = crate::html::parse_document(r#"<html><head></head><body></body></html>"#);

        execute(
            &mut document,
            r#"
            var script = document.createElement("script");
            script.onload = function() {};
            script.onerror = function() {};
            document.head.appendChild(script);
            document.body.textContent = "ok";
            "#,
        )
        .expect("script should execute");

        let body = document
            .find_first_element_by_name("body")
            .expect("missing body element");
        assert_eq!(collect_text_content(body), "ok");
    }

    #[test]
    fn supports_mediawiki_hash_string_and_number_helpers() {
        let mut document = crate::html::parse_document(r#"<html><body></body></html>"#);

        execute(
            &mut document,
            r#"
            var hash = (255).toString(36).slice(0, 2);
            var code = "AZ".charCodeAt(1);
            document.body.textContent = hash + ":" + code;
            "#,
        )
        .expect("script should execute");

        let body = document
            .find_first_element_by_name("body")
            .expect("missing body element");
        assert_eq!(collect_text_content(body), "73:90");
    }
}
