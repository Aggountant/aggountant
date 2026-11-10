---
created: 2026-11-10
updated: 2026-11-10
type: bug
reporter: conductor
status: fixed
priority: normal
closed: 2026-11-10
---

# Vero mock server drops TLS connections on macOS

## Description

Hosted CI on macOS fails explicit_test_verification_uses_mtls_read_and_persists_only_evidence on every toolchain with ADAPTER_AMBIGUOUS: the mock's listener is non-blocking, and on macOS accepted sockets inherit that mode, so the TLS handshake sees WouldBlock and the mock drops the connection. Linux does not inherit the flag.

## Resolution

### 2026-11-10T13:19:17Z · @issuectl

Fix in the preceding commit; verified by hosted macOS CI.
