# Synthetic mock PKI fixtures

These certificates and keys exist only for the local Vero VAT API mock and its
mTLS tests. They were generated for this public repository, chain to the
`aggountant-vero-mock-test-ca` root that nothing else trusts, and authenticate
nothing outside the tests. Real Vero credentials never live in the source tree.

The `.pem.fixture` suffix keeps credential-suffix heuristics, such as the shared
build runner's source capture, from dropping these files: those heuristics
cannot tell a public fixture from a secret, and these are not secrets.
