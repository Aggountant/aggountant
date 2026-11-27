//! Travel allowances of an expense claim.
//!
//! Only the Finnish rules exist (ADR-11 section 2): [`fi`] computes the
//! scheduled tax-free allowances of a trip from the Tax Administration's
//! yearly decision. The claim itself, its lines, approval and status are not
//! here; when another country's rules arrive, the split is made along this
//! module boundary, not through a country parameter.

pub mod fi;
