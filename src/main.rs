use nom::{
    bytes::complete::take_till, character::complete::line_ending, multi::separated_list0, IResult,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io::Read as _};

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

/// Parse the entire input as multiple lines, ignoring empty or comment lines,
/// returning a vector of (key, value) for the Setting lines.
fn parse_config_lines(input: &str) -> IResult<&str, Vec<(String, String)>> {
    // We'll separate the input by line endings, parse each line,
    // and collect the Setting lines.
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;

    let nested_map =
        parse_sysctl_conf_to_nested(&input).map_err(|e| format!("Failed to parse input: {}", e))?;

    let json_str = to_json_string(&nested_map);
    println!("{}", json_str);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper function to parse input and return the resulting nested map.
    /// Panics if parsing fails.
    fn parse_to_map(input: &str) -> BTreeMap<String, ConfigValue> {
        parse_sysctl_conf_to_nested(input).unwrap()
    }

    /// Test a typical multi-line input with comments, dotted keys, etc.
    #[test]
    fn test_parse_basic() {
        let input = r#"
endpoint = localhost:3000
# debug = true
log.file = /var/log/console.log
log.name = default.log

log.level = info
"#;

        let nested_map = parse_to_map(input);

        // We expect a structure like:
        // {
        //   "endpoint": "localhost:3000",
        //   "log": {
        //     "file": "/var/log/console.log",
        //     "level": "info",
        //     "name": "default.log"
        //   }
        // }
        assert_eq!(nested_map.len(), 2); // "endpoint" and "log"

        assert_eq!(
            nested_map.get("endpoint").unwrap(),
            &ConfigValue::Str("localhost:3000".to_string())
        );

        if let Some(ConfigValue::Table(log_map)) = nested_map.get("log") {
            assert_eq!(
                log_map.get("file").unwrap(),
                &ConfigValue::Str("/var/log/console.log".to_string())
            );
            assert_eq!(
                log_map.get("name").unwrap(),
                &ConfigValue::Str("default.log".to_string())
            );
            assert_eq!(
                log_map.get("level").unwrap(),
                &ConfigValue::Str("info".to_string())
            );
        } else {
            panic!("expected 'log' to be a Table");
        }
    }

    /// Test that empty lines and whitespace-only lines are safely ignored.
    #[test]
    fn test_empty_and_whitespace_lines() {
        let input = r#"

   
key = value
  
"#;
        let nested_map = parse_to_map(input);
        // Only one setting line "key = value"
        assert_eq!(nested_map.len(), 1);
        assert_eq!(
            nested_map.get("key").unwrap(),
            &ConfigValue::Str("value".to_string())
        );
    }

    #[test]
    fn test_comment_only() {
        let input = r#"
# This is a comment
# Another comment
"#;
        let nested_map = parse_to_map(input);
        // No settings
        assert!(nested_map.is_empty());
    }

    #[test]
    fn test_no_space_around_equal() {
        let input = "username=admin\npassword=secret";
        let nested_map = parse_to_map(input);
        assert_eq!(nested_map.len(), 2);
        assert_eq!(
            nested_map.get("username").unwrap(),
            &ConfigValue::Str("admin".to_string())
        );
        assert_eq!(
            nested_map.get("password").unwrap(),
            &ConfigValue::Str("secret".to_string())
        );
    }

    #[test]
    fn test_multiple_dots() {
        let input = "a.b.c.d = final\n";
        let nested_map = parse_to_map(input);

        // structure: { "a": { "b": { "c": { "d": "final" }}}}
        if let Some(ConfigValue::Table(a_map)) = nested_map.get("a") {
            if let Some(ConfigValue::Table(b_map)) = a_map.get("b") {
                if let Some(ConfigValue::Table(c_map)) = b_map.get("c") {
                    assert_eq!(
                        c_map.get("d").unwrap(),
                        &ConfigValue::Str("final".to_string())
                    );
                } else {
                    panic!("missing c map");
                }
            } else {
                panic!("missing b map");
            }
        } else {
            panic!("missing a map");
        }
    }


    #[test]
    fn test_completely_empty_input() {
        let input = "";
        let nested_map = parse_to_map(input);
        assert!(nested_map.is_empty());
    }

    #[test]
    fn test_line_with_extra_spaces() {
        let input = "   foo.bar    =    hello world   \n";
        let nested_map = parse_to_map(input);

        if let Some(ConfigValue::Table(foo_map)) = nested_map.get("foo") {
            assert_eq!(
                foo_map.get("bar").unwrap(),
                &ConfigValue::Str("hello world".to_string())
            );
        } else {
            panic!("expected 'foo' to be a Table");
        }
    }
}
