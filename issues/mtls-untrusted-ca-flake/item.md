---
created: 2026-11-10
updated: 2026-11-12
type: bug
status: fixed
priority: normal
lane: test-flakes
lane_seq: 1
collision: [crates/vero-vat-api-mock]
closed: 2026-11-12
---

# Flaky mTLS mock test: untrusted-CA handshake sometimes ends in Broken pipe

## Problem

`handshake_refuses_a_client_signed_by_an_untrusted_ca`
(`crates/vero-vat-api-mock/tests/mtls.rs`) is flaky. It expects the client's
error to contain `UnknownCA`, but sometimes the server closes the connection
first and the client sees `Broken pipe (os error 32)` instead. On Rust 1.88,
`cargo test --locked -p aggountant-vero-vat-api-mock --test mtls` failed in 2
of 6 consecutive runs.

`scripts/ci-local.sh` runs `cargo test` without `--no-fail-fast`, so the flake
also hides every test binary after it and turns the local gate red at random.

## Expected

The test accepts either outcome that proves the handshake was refused (an
`UnknownCA` alert, or a reset or broken pipe with no request recorded by the
mock server), or the mock sends its alert before closing. The assertion that
the server recorded no request stays.

## Acceptance criteria

- [ ] 50 consecutive runs of the `mtls` test pass on stable and on the MSRV.
- [ ] The test still fails if the mock accepts the untrusted client.

## Resolution

### 2026-11-12T07:24:46Z · @issuectl

Fixed in the preceding commit: 2/300 failures before, 0/300 after; the no-certificate test had the same cause.
