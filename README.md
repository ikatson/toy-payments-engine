# Toy payments engine

## Code organization
- main.rs - read the input file and process it
- amount.rs - decimal parsing
- error.rs - errors
- accounts.rs - business logic
- parser.rs - parsing CSV

## Dependencies and reasoning
atoi - for efficient parsing of integer values from byte input. Stdlib (stable) can only parse strings.
memchr - for efficient splitting of input rows with comma separator
thiserror - error deriving
tracing and tracing_subscriber - logging errors

## Implementation notes

- The parser and the code deal with ASCII bytes. We don't check utf-8 as it's an unnecessary perf loss.
