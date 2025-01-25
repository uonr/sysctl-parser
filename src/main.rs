use clap::Parser;
use sysctl_parser::{parse_sysctl_conf_to_nested, to_json_string};

use std::io::Read as _;

/// Simple program to greet a person
#[derive(Parser, Debug)]
struct Args {
    /// The path to the schema file
    #[arg(long)]
    schema: Option<String>,
    /// The path to the input file
    path: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut input = String::new();
    if let Some(path) = args.path {
        std::fs::File::open(path)?.read_to_string(&mut input)?;
    } else {
        std::io::stdin().read_to_string(&mut input)?;
    }
    let nested_map =
        parse_sysctl_conf_to_nested(&input).map_err(|e| format!("Failed to parse input: {}", e))?;

    if let Some(schema_path) = args.schema {
        let mut schema = String::new();
        std::fs::File::open(schema_path)?.read_to_string(&mut schema)?;
        let schema_fields = sysctl_parser::schema::parse(&*schema);
        sysctl_parser::schema::validate_config(&schema_fields, &nested_map)
            .map_err(|e| format!("Failed to validate config: {}", e))?;
    }
    let json_str = to_json_string(&nested_map);
    println!("{}", json_str);

    Ok(())
}
