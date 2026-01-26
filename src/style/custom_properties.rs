use std::borrow::Cow;
use std::collections::HashMap;
use std::rc::Rc;

use super::{CascadePriority, Cascaded};

const MAX_VAR_RECURSION_DEPTH: usize = 32;

#[derive(Clone, Debug)]
pub struct CustomProperties {
    values: Rc<HashMap<String, String>>,
}

impl Default for CustomProperties {
    fn default() -> Self {
        Self {
            values: Rc::new(HashMap::new()),
        }
    }
}

impl CustomProperties {
    pub fn get(&self, name: &str) -> Option<&str> {
        let name = name.trim().to_ascii_lowercase();
        self.values.get(&name).map(String::as_str)
    }

    pub(super) fn merge(
        inherited: &CustomProperties,
        declared: &HashMap<String, Cascaded<String>>,
    ) -> CustomProperties {
        if declared.is_empty() {
            return inherited.clone();
        }

        let mut merged = if inherited.values.is_empty() {
            HashMap::new()
        } else {
            inherited.values.as_ref().clone()
        };

        for (name, cascaded) in declared {
            merged.insert(name.clone(), cascaded.value.clone());
        }

        CustomProperties {
            values: Rc::new(merged),
        }
    }

    pub(super) fn resolve_vars<'a>(&'a self, input: &'a str) -> Option<Cow<'a, str>> {
        if !contains_var_function(input) {
            return Some(Cow::Borrowed(input));
        }
        let mut stack = Vec::new();
        let resolved = self.resolve_vars_with_stack(input, &mut stack, 0)?;
        Some(Cow::Owned(resolved))
    }

    fn resolve_vars_with_stack(
        &self,
        input: &str,
        stack: &mut Vec<String>,
        depth: usize,
    ) -> Option<String> {
        if depth > MAX_VAR_RECURSION_DEPTH {
            return None;
        }

        if !contains_var_function(input) {
            return Some(input.to_owned());
        }

        let bytes = input.as_bytes();
        let mut out = String::new();
        let mut last = 0usize;
        let mut idx = 0usize;

        while idx + 4 <= bytes.len() {
            if !is_var_at(bytes, idx) {
                idx += 1;
                continue;
            }

            out.push_str(&input[last..idx]);

            let args_start = idx + 4;
            let (args, consumed) = split_balanced_parens(&input[args_start..])?;
            let replacement = self.resolve_var_args(args, stack, depth + 1)?;
            out.push_str(&replacement);

            idx = args_start + consumed;
            last = idx;
        }

        out.push_str(&input[last..]);
        Some(out)
    }

    fn resolve_var_args(
        &self,
        args: &str,
        stack: &mut Vec<String>,
        depth: usize,
    ) -> Option<String> {
        let (name, fallback) = split_var_arguments(args);
        let name = name.trim();
        let fallback = fallback.map(str::trim).filter(|value| !value.is_empty());

        if !name.starts_with("--") {
            return fallback
                .and_then(|fallback| self.resolve_vars_with_stack(fallback, stack, depth));
        }

        let name = name.to_ascii_lowercase();
        if stack.iter().any(|entry| entry == &name) {
            return fallback
                .and_then(|fallback| self.resolve_vars_with_stack(fallback, stack, depth));
        }

        if let Some(raw) = self.values.get(&name) {
            stack.push(name);
            let resolved = self
                .resolve_vars_with_stack(raw, stack, depth + 1)
                .or_else(|| {
                    fallback.and_then(|fallback| self.resolve_vars_with_stack(fallback, stack, depth))
                });
            stack.pop();
            return resolved;
        }

        fallback.and_then(|fallback| self.resolve_vars_with_stack(fallback, stack, depth))
    }
}

pub(super) fn apply_custom_property_declaration(
    declared: &mut HashMap<String, Cascaded<String>>,
    name: &str,
    value: &str,
    priority: CascadePriority,
) {
    let name = name.trim().to_ascii_lowercase();
    if name.is_empty() {
        return;
    }

    let should_set = match declared.get(&name) {
        Some(existing) => priority >= existing.priority,
        None => true,
    };
    if should_set {
        declared.insert(
            name,
            Cascaded {
                value: value.trim().to_owned(),
                priority,
            },
        );
    }
}

fn contains_var_function(input: &str) -> bool {
    let bytes = input.as_bytes();
    let mut idx = 0usize;
    while idx + 4 <= bytes.len() {
        if is_var_at(bytes, idx) {
            return true;
        }
        idx += 1;
    }
    false
}

fn is_var_at(bytes: &[u8], idx: usize) -> bool {
    let Some(tail) = bytes.get(idx..idx + 4) else {
        return false;
    };
    (tail[0] | 0x20) == b'v' && (tail[1] | 0x20) == b'a' && (tail[2] | 0x20) == b'r' && tail[3] == b'('
}

fn split_balanced_parens(input: &str) -> Option<(&str, usize)> {
    let bytes = input.as_bytes();
    let mut depth = 1usize;
    let mut idx = 0usize;

    while idx < bytes.len() {
        match bytes[idx] {
            b'(' => depth = depth.saturating_add(1),
            b')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some((&input[..idx], idx + 1));
                }
            }
            _ => {}
        }
        idx += 1;
    }

    None
}

fn split_var_arguments(args: &str) -> (&str, Option<&str>) {
    let bytes = args.as_bytes();
    let mut depth = 0usize;
    let mut idx = 0usize;

    while idx < bytes.len() {
        match bytes[idx] {
            b'(' => depth = depth.saturating_add(1),
            b')' => depth = depth.saturating_sub(1),
            b',' if depth == 0 => return (&args[..idx], Some(&args[idx + 1..])),
            _ => {}
        }
        idx += 1;
    }

    (args, None)
}

