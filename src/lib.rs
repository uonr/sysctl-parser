pub mod schema;

use nom::{
    bytes::complete::take_till, character::complete::line_ending, multi::separated_list0, IResult,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConfigValue {
    Str(String),
    Table(BTreeMap<String, ConfigValue>),
}

/// A utility function to help insert a dotted key into our nested structure.
pub fn insert_nested_key(root: &mut BTreeMap<String, ConfigValue>, key: &str, val: &str) {
    let parts: Vec<&str> = key.split('.').collect();
    insert_recursive(root, &parts, val);
}

fn insert_recursive(current_map: &mut BTreeMap<String, ConfigValue>, parts: &[&str], val: &str) {
    if parts.len() == 1 {
        current_map.insert(parts[0].to_string(), ConfigValue::Str(val.to_string()));
        return;
    }

    let head = parts[0];
    let tail = &parts[1..];

    // Check if this key already exists
    if let Some(existing_value) = current_map.get_mut(head) {
        if let ConfigValue::Table(ref mut sub_map) = existing_value {
            insert_recursive(sub_map, tail, val);
            return;
        } else {
            // If it was previously a string, overwrite it
            let mut new_map = BTreeMap::new();
            insert_recursive(&mut new_map, tail, val);
            *existing_value = ConfigValue::Table(new_map);
            return;
        }
    } else {
        let mut new_map = BTreeMap::new();
        insert_recursive(&mut new_map, tail, val);
        current_map.insert(head.to_string(), ConfigValue::Table(new_map));
    }
}

#[derive(Debug, PartialEq)]
enum ParsedLine {
    /// Comment line (ignored later)
    Comment(String),
    /// Setting "key = value"
    Setting(String, String),
    /// Empty (whitespace-only) line
    Empty,
}

/// Parse a single line
fn parse_line_content(input: &str) -> IResult<&str, ParsedLine> {
    let trimmed = input.trim_start();
    if trimmed.is_empty() {
        return Ok(("", ParsedLine::Empty));
    }

    if trimmed.starts_with('#') {
        let comment = trimmed[1..].trim().to_string();
        return Ok(("", ParsedLine::Comment(comment)));
    }

    // Try to find '='
    if let Some(eq_pos) = trimmed.find('=') {
        let key = &trimmed[..eq_pos];
        let value = &trimmed[eq_pos + 1..];
        return Ok((
            "",
            ParsedLine::Setting(key.trim().to_string(), value.trim().to_string()),
        ));
    }

    Ok(("", ParsedLine::Empty))
}

/// Parse exactly one line up to newline.
fn parse_line(input: &str) -> IResult<&str, ParsedLine> {
    // Take everything until '\r' or '\n'
    let (remaining, line_str) = take_till(|c| c == '\r' || c == '\n')(input)?;
    // Then parse that content
    let (_, parsed) = parse_line_content(line_str)?;
    Ok((remaining, parsed))
}

fn parse_config_lines(input: &str) -> IResult<&str, Vec<(String, String)>> {
    let (remaining, lines) = separated_list0(
        // we separate by line_ending
        line_ending,
        parse_line,
    )(input)?;

    // Filter to keep only Settings
    let mut settings = Vec::new();
    for line in lines {
        if let ParsedLine::Setting(k, v) = line {
            settings.push((k, v));
        }
    }

    // Optionally consume any trailing newline or whitespace
    let (remaining, _) = nom::combinator::opt(line_ending)(remaining)?;

    Ok((remaining, settings))
}

pub fn parse_sysctl_conf_to_nested(input: &str) -> Result<BTreeMap<String, ConfigValue>, String> {
    match parse_config_lines(input) {
        Ok((_, kvs)) => {
            let mut root = BTreeMap::new();
            for (k, v) in kvs {
                insert_nested_key(&mut root, &k, &v);
            }
            Ok(root)
        }
        Err(e) => Err(format!("Parse error: {:?}", e)),
    }
}

pub fn to_json_string(map: &BTreeMap<String, ConfigValue>) -> String {
    serde_json::to_string_pretty(map).unwrap()
}

pub fn parse_to_map(input: &str) -> BTreeMap<String, ConfigValue> {
    parse_sysctl_conf_to_nested(input).unwrap()
}
