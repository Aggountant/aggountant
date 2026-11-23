# Result

`crates/cli/tests/explain_cli.rs` no longer pins the wording. A helper,
`guide_what_to_do`, reads the "Mitä tehdään" cell of the `ALREADY_INITIALIZED`
row from `docs/user/fi/reference/error-codes.md` at test time, and the test
asserts that the error JSON `hint` equals that cell. The guide's prose can now
change without breaking the test, and the test still checks that the hint
comes from the guide. The guide text was not touched.
