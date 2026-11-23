# Result

`Cargo.lock`: `url` 2.5.8 → 2.5.9 (`cargo update -p url --precise 2.5.9`).
It also moved `form_urlencoded` 1.2.2 → 1.2.3, which url 2.5.9 requires;
nothing else changed. `cargo deny check advisories bans licenses sources`
passes, and `scripts/ci-local.sh` passes on stable and MSRV 1.89.
