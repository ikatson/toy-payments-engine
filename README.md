# Toy payments engine

## Code organization

- main.rs - read the input file and process it
- amount.rs - decimal parsing
- error.rs - errors
- accounts.rs - business logic
- parser.rs - parsing CSV

## Dependencies and reasoning behind using them

- atoi - for efficient parsing of integer values from byte input. Stdlib (stable) can only parse strings.
  We could implement it ourselves, but I used the dep to reduce the surface area.
- memchr - for efficient splitting of input rows with comma separator
- thiserror - error deriving
- tracing and tracing_subscriber - logging errors

## Implementation notes
- The decimal amount stored is represented as u64, the last 4 places are taken by the fraction part.
  The max number that can be represented is 1_844_674_407_370_955.1615
- The parser and the code deal with ASCII bytes. We don't check utf-8 as it's an unnecessary perf loss.
- The parses assumes a fixed CSV format with a header and at least 4 columns exactly in this order:
  type, client, tx, amount

  This lets us make the parsing very simple and efficient.
  Using "csv" crate would make more sense if we knew the fields can be reordered, strings could contain quotes etc.
  But as the spec doesn't require it, we optimize for the given case.

## Assumptions not stated in the spec
- The CSV input contains only the columns specified exactly in the order specified. It MAY contain extra columns at the end, we ignore them.
- The CSV strings don't contain quotes (or more specifically, quoted commas or newlines that would break parsing).
- Only deposits can be disputed. This seems to be implicit in the spec.
- If a chargeback would bring the account total into negative, we set it to zero instead for simplicity, as the account is frozen anyway, and there's no way to unfreeze it.
- "held" can become greater than "total" if a transaction is disputed, but some money were withdrawn. This is considered OK as long as the dispute is resolved. This sets amount available for withdrawal to 0.
