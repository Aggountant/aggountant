# Public, synthetic wage-channel test PKI

These RSA-2048 test keys and certificates are intentionally public. Never use
or trust them outside the loopback mock. They identify fictional parties, not
any company or person. Created with OpenSSL on 2026-10-24: a self-signed test CA,
a server CSR with `serverAuth` + SAN `localhost,127.0.0.1`, and a client CSR with
`clientAuth` + `digitalSignature`. Certificates expire in 2036. The disposable
CA private key was not committed; changing fixtures requires regenerating the
whole test PKI and signed golden messages together. `.fixture` prevents build
capture from confusing these public inputs with operator credential files.

Golden messages are byte-compared against the Rust serializer and RSA signer.
`issues/tulorekisteri-wage-report/verify_xml.py` independently verifies them
against official XSDs and with signxml (including extracted SOAP namespace context).
