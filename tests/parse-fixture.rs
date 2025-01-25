use sysctl_parser::*;

#[test]
fn test_2() {
    let input = include_str!("../fixtures/2.conf");

    let nested_map = parse_to_map(input);

    assert_eq!(nested_map.len(), 2);
    assert_eq!(
        nested_map.get("endpoint").unwrap(),
        &ConfigValue::Str("localhost:3000".to_string())
    );

}

#[test]
fn test_3() {
    let input = include_str!("../fixtures/3.conf");

    let nested_map = parse_to_map(input);

    assert_eq!(nested_map.len(), 2);
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
