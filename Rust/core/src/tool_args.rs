use std::{env, path::PathBuf};

use crate::{GooseError, GooseResult};

pub fn args() -> Vec<String> {
    env::args().skip(1).collect()
}

pub fn flag(args: &[String], name: &str) -> bool {
    args.iter().any(|arg| arg == name)
}

pub fn value(args: &[String], name: &str) -> GooseResult<Option<String>> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == name {
            let Some(value) = iter.next() else {
                return Err(GooseError::message(format!("missing value for {name}")));
            };
            return Ok(Some(value.clone()));
        }
    }
    Ok(None)
}

pub fn path_value(args: &[String], name: &str) -> GooseResult<Option<PathBuf>> {
    Ok(value(args, name)?.map(PathBuf::from))
}

pub fn default_path(args: &[String], name: &str, default: &str) -> GooseResult<PathBuf> {
    Ok(path_value(args, name)?.unwrap_or_else(|| PathBuf::from(default)))
}

/// Error-message wording for the optional integer helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidWording {
    /// `invalid {name}: {source}` — the terse form.
    Terse,
    /// `invalid {name} value {raw}: {error}` — echoes the rejected value.
    WithValue,
}

fn parse_optional<T>(
    raw: Option<String>,
    name: &str,
    wording: InvalidWording,
    parse: impl FnOnce(&str) -> Result<T, std::num::ParseIntError>,
) -> GooseResult<Option<T>> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    parse(&raw).map(Some).map_err(|error| {
        GooseError::message(match wording {
            InvalidWording::Terse => format!("invalid {name}: {error}"),
            InvalidWording::WithValue => format!("invalid {name} value {raw}: {error}"),
        })
    })
}

pub fn optional_usize(
    args: &[String],
    name: &str,
    wording: InvalidWording,
) -> GooseResult<Option<usize>> {
    parse_optional(value(args, name)?, name, wording, |raw| {
        raw.parse::<usize>()
    })
}

pub fn optional_u64(
    args: &[String],
    name: &str,
    wording: InvalidWording,
) -> GooseResult<Option<u64>> {
    parse_optional(value(args, name)?, name, wording, |raw| raw.parse::<u64>())
}

pub fn optional_mib(
    args: &[String],
    name: &str,
    wording: InvalidWording,
) -> GooseResult<Option<u64>> {
    Ok(optional_u64(args, name, wording)?.map(|value| value * 1024 * 1024))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(pairs: &[&str]) -> Vec<String> {
        pairs.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn optional_int_helpers_parse_and_keep_both_wordings() {
        let bad = argv(&["--limit", "abc"]);
        let err = optional_usize(&bad, "--limit", InvalidWording::Terse).unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid --limit: invalid digit found in string"
        );
        let err = optional_usize(&bad, "--limit", InvalidWording::WithValue).unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid --limit value abc: invalid digit found in string"
        );

        let good = argv(&["--limit", "42"]);
        assert_eq!(
            optional_usize(&good, "--limit", InvalidWording::Terse).unwrap(),
            Some(42)
        );
        assert_eq!(
            optional_usize(&good, "--limit", InvalidWording::WithValue).unwrap(),
            Some(42)
        );
        assert_eq!(
            optional_usize(&[], "--limit", InvalidWording::Terse).unwrap(),
            None
        );
        assert_eq!(
            optional_u64(&good, "--limit", InvalidWording::Terse).unwrap(),
            Some(42)
        );
        assert_eq!(
            optional_mib(&argv(&["--mem", "2"]), "--mem", InvalidWording::Terse).unwrap(),
            Some(2 * 1024 * 1024)
        );
    }
}
