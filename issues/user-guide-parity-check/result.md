# Result

`scripts/check-user-guide.sh` is the gate. `scripts/ci-local.sh` runs it last
as `== user guide ==` (stable toolchain), and `.github/workflows/ci.yml` runs
it as the `user-guide` job on ubuntu-latest. It checks:

1. every `docs/user/fi/` page has an `en/` page at the same path and vice
   versa, and each pair has the same heading levels in the same order;
2. every relative link in any `docs/user/**/*.md` resolves, including a
   `#anchor` against the target page's GitHub heading slugs (links inside
   code fences and inline code are skipped, external URLs are not fetched);
3. `docs/user/practice/run-practice-month.sh` passes against the freshly
   built `aggountant` binary in a throwaway home (or a binary given as `$1`).

## Verification

- Current guide: 32 page pairs pass; full `scripts/ci-local.sh` green.
- An extra `fi/` page without an `en/` partner, a link to a missing page, a
  link to a missing anchor, an extra heading in one language, and a camt
  statement with an unsupported currency each made the script exit 1 and
  name the file and line (the practice failure prints the CLI's
  `INVALID_CAMT053` error).

## Simplicity

Reuses `run-practice-month.sh` unchanged for the practice part instead of a
second runner. The structural checks are one embedded Python block with no
dependencies; jq was already required by the practice script. Left out:
external URL checks (network-dependent, flaky), comparison of translated
heading text, a separate binary-download path.

## Decisions for later review

- Heading parity checks levels and order only, not text, because the text is
  translated. `docs/user/AGENTS.md` already states the rule; the gate makes it
  enforceable. Anchor checks use GitHub's slug rules (lower-case, keep
  letters, digits, marks, `-`, `_`; spaces to hyphens; `-1` suffixes for
  repeats), which also matches the guide's stated anchor convention.
- The link check covers all of `docs/user/`, including `AGENTS.md` files and
  `practice/README.md`, not only the two language trees.
- Hosted CI gets its own job rather than a step in `test`, so it runs once,
  not on four matrix cells. `ci.yml` carries a `shipshape-ci:managed` header;
  a regeneration by `/shipshape-ci` would drop the job, so whoever regenerates
  it must carry the `user-guide` job over (or teach the generator about it).
- The practice run's stdout is discarded in the gate; its failures print to
  stderr, which is what a reader needs. Run the practice script directly for
  the full transcript.
- No model review was run for this slice; the script was checked by the
  negative tests above.
