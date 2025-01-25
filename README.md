# sysctl-parser

This is a simple parser for sysctl.conf files.

## Usage

Basic usage, output the JSON representation.

```
cargo run < fixtures/2.conf

# or

cargo run -- fixtures/2.conf
```

With `--schema` flag, check the file against the schema.

```bash
# Pass
cargo run -- --schema fixtures/schema.txt fixtures/1.conf

# Fail
cargo run -- --schema fixtures/schema.txt fixtures/invaild.conf
```