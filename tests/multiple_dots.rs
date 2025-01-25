use sysctl_parser::*;

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
