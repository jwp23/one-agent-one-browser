use std::fmt;
use std::fmt::Write as _;
use std::io::{self, Write as IoWrite};
use std::sync::OnceLock;
use std::time::Instant;

const MAX_LINE_CHARS: usize = 120;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Level {
    fn tag(self) -> &'static str {
        match self {
            Level::Error => "ERR",
            Level::Warn => "WRN",
            Level::Info => "INF",
            Level::Debug => "DBG",
            Level::Trace => "TRC",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "error" | "err" => Some(Level::Error),
            "warn" | "wrn" | "warning" => Some(Level::Warn),
            "info" | "inf" => Some(Level::Info),
            "debug" | "dbg" => Some(Level::Debug),
            "trace" | "trc" => Some(Level::Trace),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Nav,
    Net,
    Css,
    Res,
    Layout,
    Render,
}

impl Target {
    fn tag(self) -> &'static str {
        match self {
            Target::Nav => "NAV",
            Target::Net => "NET",
            Target::Css => "CSS",
            Target::Res => "RES",
            Target::Layout => "LYT",
            Target::Render => "RND",
        }
    }

    const fn mask(self) -> u64 {
        match self {
            Target::Nav => 1 << 0,
            Target::Net => 1 << 1,
            Target::Css => 1 << 2,
            Target::Res => 1 << 3,
            Target::Layout => 1 << 4,
            Target::Render => 1 << 5,
        }
    }
}

const ALL_TARGETS: u64 = Target::Nav.mask()
    | Target::Net.mask()
    | Target::Css.mask()
    | Target::Res.mask()
    | Target::Layout.mask()
    | Target::Render.mask();

struct Config {
    targets: u64,
    max_level: Level,
    start: Instant,
}

static CONFIG: OnceLock<Config> = OnceLock::new();

fn config() -> &'static Config {
    CONFIG.get_or_init(|| {
        let targets = match std::env::var("OAB_LOG") {
            Ok(value) => parse_targets_env(Some(value.as_str())),
            Err(std::env::VarError::NotPresent) => ALL_TARGETS,
            Err(_) => 0,
        };
        let max_level =
            std::env::var("OAB_LOG_LEVEL").ok().and_then(|s| Level::parse(&s)).unwrap_or(Level::Info);
        Config {
            targets,
            max_level,
            start: Instant::now(),
        }
    })
}

fn parse_targets_env(value: Option<&str>) -> u64 {
    let Some(value) = value else {
        return 0;
    };

    let value = value.trim();
    if value.is_empty() {
        return 0;
    }

    match value.to_ascii_lowercase().as_str() {
        "0" | "false" | "off" | "none" => return 0,
        "1" | "true" | "on" | "*" | "all" => return ALL_TARGETS,
        _ => {}
    };

    let mut mask = 0u64;
    for token in value.split(|c: char| c == ',' || c.is_whitespace() || c == ';') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        match token.to_ascii_lowercase().as_str() {
            "*" | "all" => return ALL_TARGETS,
            "nav" => mask |= Target::Nav.mask(),
            "net" => mask |= Target::Net.mask(),
            "css" => mask |= Target::Css.mask(),
            "res" | "resources" => mask |= Target::Res.mask(),
            "layout" | "lyt" => mask |= Target::Layout.mask(),
            "render" | "rnd" => mask |= Target::Render.mask(),
            _ => {}
        }
    }
    mask
}

pub fn enabled(target: Target, level: Level) -> bool {
    let cfg = config();
    (cfg.targets & target.mask()) != 0 && level <= cfg.max_level
}

pub fn log(target: Target, level: Level, message: fmt::Arguments<'_>) {
    if !enabled(target, level) {
        return;
    }

    let cfg = config();
    let elapsed_ms: u64 = cfg
        .start
        .elapsed()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX);

    let mut msg = String::new();
    let _ = fmt::write(&mut msg, message);
    sanitize_in_place(&mut msg);
    let msg = msg.trim();

    let mut line = String::new();
    let _ = write!(
        &mut line,
        "t={elapsed_ms:>6} {} {} {msg}",
        level.tag(),
        target.tag()
    );
    truncate_in_place(&mut line, MAX_LINE_CHARS);

    let mut out = io::stdout().lock();
    let _ = writeln!(out, "{line}");
    let _ = IoWrite::flush(&mut out);
}

pub fn shorten<'a>(value: &'a str, max_chars: usize) -> std::borrow::Cow<'a, str> {
    let value = value.trim();
    if max_chars == 0 {
        return std::borrow::Cow::Borrowed("");
    }

    let mut count = 0usize;
    let mut cut_byte_idx: Option<usize> = None;
    for (byte_idx, _) in value.char_indices() {
        if count == max_chars {
            cut_byte_idx = Some(byte_idx);
            break;
        }
        count = count.saturating_add(1);
    }

    let Some(cut_byte_idx) = cut_byte_idx else {
        return std::borrow::Cow::Borrowed(value);
    };

    if max_chars == 1 {
        return std::borrow::Cow::Borrowed("…");
    }

    let mut out = String::new();
    for (byte_idx, ch) in value.char_indices() {
        if byte_idx >= cut_byte_idx {
            break;
        }
        out.push(ch);
    }
    out.pop();
    out.push('…');
    std::borrow::Cow::Owned(out)
}

fn sanitize_in_place(value: &mut String) {
    if !value.as_bytes().iter().any(|&b| b == b'\n' || b == b'\r' || b == b'\t') {
        return;
    }

    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\n' | '\r' | '\t' => out.push(' '),
            _ => out.push(ch),
        }
    }
    *value = out;
}

fn truncate_in_place(value: &mut String, max_chars: usize) {
    if max_chars == 0 {
        value.clear();
        return;
    }

    let mut count = 0usize;
    let mut cut_byte_idx: Option<usize> = None;
    for (byte_idx, _) in value.char_indices() {
        if count == max_chars {
            cut_byte_idx = Some(byte_idx);
            break;
        }
        count = count.saturating_add(1);
    }

    let Some(cut_byte_idx) = cut_byte_idx else {
        return;
    };

    value.truncate(cut_byte_idx);
    if max_chars == 1 {
        value.clear();
        value.push('…');
        return;
    }
    value.pop();
    value.push('…');
}
