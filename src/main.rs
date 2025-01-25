use sysctl_parser::{parse_sysctl_conf_to_nested, to_json_string};

use std::io::Read as _;

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
    use std::collections::BTreeMap;

    use sysctl_parser::ConfigValue;

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
