# crates/wire — rules the code does not show

The canonical wire format is the preimage of the voucher hash chain. A sealed
voucher must hash to the same value on any machine, toolchain or future
release, for as long as the books must be kept, so this crate changes far more
rarely than its size suggests.

- **A released `wire_version` is frozen.** Any change to canonicalization or the
  profile, even a bug fix, is a new `wire_version` with its own golden
  directory; old goldens stay and are verified forever. Goldens are regenerated
  only locally with `--features regen-goldens`; CI only verifies.
- **The profile is RFC 8785 (JCS) with every domain value as a string.**
  Decimals are strings with one canonical grammar and preserved scale
  (`"1234.50"` stays `"1234.50"`, so equal numbers with different scale hash
  differently and the Model normalises scale where equality matters). `Money`
  always carries its currency, dates are `YYYY-MM-DD`, timestamps RFC 3339 UTC
  with `Z`, hashes lowercase hex. JSON numbers are allowed only for small
  non-negative counts and indices. No generic maps in hashed structures.
- **aggountant owns the encoder.** The hashing path has no runtime dependency on
  an outside JCS crate (one may be used as a test oracle), and `rust_decimal`
  never appears in a `Serialize` implementation: decimals reach the wire only
  through the string codec. Hashes are always computed from the wire encoder,
  never from `serde_json::to_vec`.
- **The read path is strict.** Reject duplicate keys, unknown fields, invalid
  UTF-8 and trailing bytes, rebuild the Model types, re-encode with the frozen
  encoder for that record's version and require byte equality
  (`NON_CANONICAL_WIRE`). The chain is verified against the stored bytes, never
  against reconstructed ones, and non-canonical aliases are rejected rather than
  normalised.
