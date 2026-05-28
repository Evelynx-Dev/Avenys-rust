# Error Tests for Avenys Compiler

This directory contains test files to verify the error reporting system of the Avenys compiler. Each file should produce a specific type of error.

## Test Files

### Lexer Errors (01-02)
- `01_lexer_unexpected_char.mire` - Unexpected character '@' in source
- `02_lexer_unterminated_string.mire` - Unclosed string literal

### Parser Errors (03, 14)
- `03_parser_missing_brace.mire` - Missing brace after if condition
- `14_parser_else_without_brace.mire` - Missing brace after else

### Type Errors (04-05, 07, 09-11, 13)
- `04_type_undefined_variable.mire` - Using undefined variable
- `05_type_incompatible_types.mire` - Adding i64 and str
- `07_type_not_callable.mire` - Calling a non-function value
- `09_type_unknown_field.mire` - Accessing undefined struct field
- `10_type_and_operand_mismatch.mire` - Using 'and' with incompatible types
- `11_type_array_plus_scalar.mire` - Adding array and scalar
- `13_type_wrong_arg_count.mire` - Wrong number of function arguments

### Runtime Errors (06, 08)
- `06_runtime_out_of_bounds.mire` - Array index out of bounds
- `08_runtime_division_by_zero.mire` - Division by zero

### Ownership Errors (12)
- `12_ownership_double_borrow.mire` - Double mutable borrow of same variable

## Expected Error Types

| File | Expected Error Type |
|------|-------------------|
| 01_lexer_unexpected_char.mire | lexer |
| 02_lexer_unterminated_string.mire | lexer |
| 03_parser_missing_brace.mire | parser |
| 04_type_undefined_variable.mire | type |
| 05_type_incompatible_types.mire | type |
| 06_runtime_out_of_bounds.mire | runtime |
| 07_type_not_callable.mire | type |
| 08_runtime_division_by_zero.mire | runtime |
| 09_type_unknown_field.mire | type |
| 10_type_and_operand_mismatch.mire | type |
| 11_type_array_plus_scalar.mire | type |
| 12_ownership_double_borrow.mire | ownership |
| 13_type_wrong_arg_count.mire | type |
| 14_parser_else_without_brace.mire | parser |

## Running Tests

To test individual files:
```bash
cargo run -- run tests/error/01_lexer_unexpected_char.mire
```

To test all error files:
```bash
for f in tests/error/*.mire; do cargo run -- run "$f" 2>&1; done
```