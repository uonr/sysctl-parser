use std::collections::BTreeMap;

use nom::{
    IResult,
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{line_ending, space0},
    combinator::map,
    multi::separated_list0,
};

use super::ConfigValue;

#[derive(Debug, Clone, PartialEq)]
pub enum SchemeType {
    Bool,
    String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SchemeField {
    pub name: String,
    pub field_type: SchemeType,
}

fn parse_field_type(input: &str) -> IResult<&str, SchemeType> {
    alt((
        map(tag("bool"), |_| SchemeType::Bool),
        map(tag("string"), |_| SchemeType::String),
    ))(input)
}

fn parse_identifier(input: &str) -> IResult<&str, String> {
    let (input, ident) = take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)?;
    Ok((input, ident.to_string()))
}

fn parse_line(input: &str) -> IResult<&str, SchemeField> {
    let (input, name) = parse_identifier(input)?;

    let (input, _) = space0(input)?;
    let (input, _) = tag("->")(input)?;
    let (input, _) = space0(input)?;

    let (input, field_type) = parse_field_type(input)?;

    Ok((input, SchemeField { name, field_type }))
}

pub fn parse_scheme(input: &str) -> IResult<&str, Vec<SchemeField>> {
    separated_list0(line_ending, parse_line)(input)
}

pub fn parse(input: &str) -> Vec<SchemeField> {
    let (_, fields)=parse_scheme(input).expect("Failed to parse scheme");
    fields
}


/// Validate a config against a schema
pub fn validate_config(
    schema: &[SchemeField],
    config: &BTreeMap<String, ConfigValue>
) -> Result<(), String> {
    for field in schema {
        let field_name = &field.name;
        let field_type = &field.field_type;

        // Check presence
        let value = match config.get(field_name) {
            Some(val) => val,
            None => {
                return Err(format!("Missing required field: '{}'", field_name));
            }
        };

        // Check type
        match field_type {
            SchemeType::Bool => {
                match value {
                    ConfigValue::Str(s) => {
                        // Must be strictly "true" or "false"
                        if s != "true" && s != "false" {
                            return Err(format!(
                                "Field '{}' must be a bool ('true'/'false'), got: '{}'",
                                field_name, s
                            ));
                        }
                    }
                    ConfigValue::Table(_) => {
                        return Err(format!(
                            "Field '{}' must be a bool, but found a nested table.",
                            field_name
                        ));
                    }
                }
            }
            SchemeType::String => {
                match value {
                    ConfigValue::Str(_) => {
                        // any string is OK
                    }
                    ConfigValue::Table(_) => {
                        return Err(format!(
                            "Field '{}' must be a string, but found a nested table.",
                            field_name
                        ));
                    }
                }
            }
        }
    }


    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bool_line() {
        let input = "is_active -> bool";
        let (remain, parsed) = parse_line(input).unwrap();
        assert!(remain.is_empty());
        assert_eq!(
            parsed,
            SchemeField {
                name: "is_active".into(),
                field_type: SchemeType::Bool,
            }
        );
    }

    #[test]
    fn test_parse_string_line() {
        let input = "username->string";
        let (remain, parsed) = parse_line(input).unwrap();
        assert!(remain.is_empty());
        assert_eq!(
            parsed,
            SchemeField {
                name: "username".into(),
                field_type: SchemeType::String,
            }
        );
    }

    #[test]
    fn test_parse_scheme_multiple_lines() {
        let input = r#"enable_feature -> bool
display_name -> string
another_flag->bool
"#;
        let (_, fields) = parse_scheme(input).expect("Failed to parse scheme");
        assert_eq!(
            fields,
            vec![
                SchemeField { name: "enable_feature".into(), field_type: SchemeType::Bool },
                SchemeField { name: "display_name".into(), field_type: SchemeType::String },
                SchemeField { name: "another_flag".into(), field_type: SchemeType::Bool },
            ]
        );
    }

    #[test]
    fn test_parse_error_line() {
        let input = "invalid line";
        let result = parse_line(input);
        assert!(result.is_err());
    }



    #[test]
    fn test_validation_ok() {
        // schema
        let schema = vec![
            SchemeField { name: "is_active".into(), field_type: SchemeType::Bool },
            SchemeField { name: "username".into(), field_type: SchemeType::String },
        ];

        // config
        let mut config = BTreeMap::new();
        config.insert("is_active".to_string(), ConfigValue::Str("true".to_string()));
        config.insert("username".to_string(), ConfigValue::Str("Alice".to_string()));

        // should pass
        let result = validate_config(&schema, &config);
        assert!(result.is_ok(), "Expected validation to succeed, got {:?}", result);
    }

    #[test]
    fn test_validation_missing_field() {
        // schema
        let schema = vec![
            SchemeField { name: "is_active".into(), field_type: SchemeType::Bool },
            SchemeField { name: "username".into(), field_type: SchemeType::String },
        ];

        // config: only "is_active" is present, "username" missing
        let mut config = BTreeMap::new();
        config.insert("is_active".to_string(), ConfigValue::Str("false".to_string()));

        // should fail because "username" is missing
        let result = validate_config(&schema, &config);
        assert!(result.is_err(), "Expected validation to fail, got {:?}", result);
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("Missing required field"), "Error message mismatch: {}", err_msg);
    }
}
