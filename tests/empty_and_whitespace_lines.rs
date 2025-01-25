use sysctl_parser::*;

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
