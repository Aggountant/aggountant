# Security Policy

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues, discussions, or
pull requests.**

Report privately using **GitHub's [Private Vulnerability Reporting](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability)**:
open the repository's **Security** tab → **Report a vulnerability**, or go directly to
<https://github.com/aggountant/aggountant/security/advisories/new>.

Include, as far as you can: the affected version or commit, the component and threat surface
(for example the Vero.fi client, a statement parser, the credential store), reproduction steps
or a proof-of-concept, and the impact you observed. Use synthetic data; never send real
bookkeeping data or credentials.

aggountant is a local command-line tool for one trusted user on one machine. The surfaces
most worth a researcher's attention are:

- **the Vero.fi client**, which submits VAT returns over HTTPS (rustls) with the user's own
  software registration and certificate, and the outbox that decides whether an uncertain
  submission is reconciled instead of resent;
- **credential handling**: the Vero.fi credential bundle read from a local file, and the
  in-memory vault that must keep secrets out of logs, provenance and the outbox;
- **parsers for user-supplied files**: camt.053/camt.052 bank statements (XML), receipts and
  invoices captured as evidence, and the JSON accepted on standard input;
- **personal data at rest**: the encrypted employee store used for payroll and Tulorekisteri
  reports, and the integrity of sealed vouchers and their content hashes;
- **the prebuilt release binaries** and the shell installer published on GitHub Releases.

Findings that only matter when several users or processes share one data home, or when the
local user is hostile, are outside the current design; report them anyway if you think the
impact is real, and say so.

<!-- shipshape-security:supported-versions-start -->
## Supported Versions

aggountant is in 0.x. Only the latest release receives security fixes.

| Version | Supported |
|---------|-----------|
| latest  | ✅        |
<!-- shipshape-security:supported-versions-end -->

## What to Expect

- We will acknowledge your report as soon as we can; this is a small project maintained by one
  person.
- We will confirm the issue and determine its severity, and keep you informed of progress.
- We ask that you give us a reasonable window to release a fix before any public disclosure;
  we practice coordinated disclosure and will credit you unless you prefer to remain anonymous.

## Safe Harbor

We consider good-faith security research conducted under this policy to be authorized. We will
not pursue or support legal action against researchers who act in good faith, avoid privacy
violations and service disruption, and give us a reasonable time to respond before disclosure.

This safe harbor covers only assets this project controls: its source code, this repository,
and the artifacts it publishes. It does not authorize testing of GitHub, package registries,
Vero.fi, Tulorekisteri, banks, or other third-party services, whose own policies continue to
apply.
