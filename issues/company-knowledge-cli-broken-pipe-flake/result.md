# Result: company-knowledge-cli-broken-pipe-flake

Fixed together with `employer-report-test-stdin-flake`; the `invoke` helper
in `company_knowledge_cli.rs` now writes stdin through
`crates/cli/tests/common/mod.rs::feed_stdin`, which tolerates `BrokenPipe`
when the CLI refuses before reading. The result, review and assessment for
both issues are in `issues/employer-report-test-stdin-flake/`.
