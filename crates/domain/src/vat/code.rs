use super::*;

/// One dated edition of a VAT rate class, valid over `[valid_from, valid_to]`
/// (`valid_to = None` = current). `percent` is a canonical decimal string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VatRateEdition {
    pub rate_class: String,
    pub valid_from: Date,
    pub valid_to: Option<Date>,
    pub percent: String,
}

/// The bundled dated VAT rate table (all editions of all rate classes).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VatRateTable {
    pub table: String,
    pub editions: Vec<VatRateEdition>,
}

/// The aggregated VAT figures **derived from the sealed vouchers** of a period —
/// the ledger-authoritative counterpart to a [`VatFilingInput`], for the file-vat
/// cross-check. Each total is the **signed** EUR sum of `round(base × rate)` over
/// the period's VAT-coded postings (a sales credit note / purchase return reduces
/// its total), so a period dominated by corrections can be negative.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DerivedVatFigures {
    pub period: PeriodId,
    pub sales_vat: Money,
    pub purchase_vat: Money,
    /// Category-specific return fields derivable from sealed treatment provenance.
    /// Fields 307/308 are computed only when building the final filing.
    pub fields: VatReturnFields,
}

/// The direction a VAT code contributes to: output tax (on sales) or deductible
/// input tax (on purchases). Parsed from the code's first segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VatDirection {
    Sales,
    Purchase,
}

impl VatDirection {
    /// The lower-case label used in the code prefix and in structural diagnostics.
    pub(super) fn label(self) -> &'static str {
        match self {
            VatDirection::Sales => "sales",
            VatDirection::Purchase => "purchase",
        }
    }
}

/// The deductible fraction of input VAT a `purchase-` code carries under **AVL
/// 117 §** (partial deductibility — osittain vähennyskelpoinen). An acquisition
/// serving both deductible and non-deductible use (mixed-use real estate apportioned
/// by floor area, an asset with a documented private-use share, the edustus/mainos
/// boundary) deducts only the deductible fraction of its input VAT. Encoded as a
/// trailing `-deduct<NN>` marker on the VAT code (`purchase-domestic-standard-deduct50`
/// = 50 % deductible), so it needs no model-layer field and stays inside the semantic-
/// code family (`candidate.md §3`).
///
/// [`Full`](Deductibility::Full) is the default (no marker → 100 % deductible).
/// `NN` is a canonical `1..=99`: **100 % is not a marker** (use the plain code) and
/// **0 % is not a marker either** (a wholly non-deductible acquisition books the VAT
/// into cost with *no* deductible VAT posting at all — there is nothing for a code to
/// tag). The marker is refused on `sales-` codes: output VAT on a sale is never
/// fractional (issue `vat-partial-deductibility`).
///
/// **The fraction is an input, not a determination.** The domain does not adjudicate
/// *whether* partial deduction applies or *what* ratio is lawful (a passenger car,
/// say, is often wholly non-deductible, not proportionally deductible) — `NN` must be
/// an externally-established figure with its own basis. **Scope limits, honestly
/// stated (both deferred to a model-layer field, since the code string is a stopgap
/// encoding, not the durable home for a numeric parameter):**
/// * only **integer** percent is expressible; a ratio like 63,5 % is *refused* (a
///   loud `VAT_CODE`, never silently rounded) rather than approximated — see the
///   `vat-partial-deductibility-model-field` follow-up for an exact fraction + its
///   provenance;
/// * the domain derives only the *deductible* figure from `base × rate × fraction`;
///   it does **not** verify the operator booked the non-deductible remainder to cost
///   (that needs chart/account-role metadata — the `vat-partial-deductibility-\
///   completeness` follow-up), the same derive-from-base boundary every VAT figure
///   here already sits behind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Deductibility {
    Full,
    Partial(u32),
}

impl Deductibility {
    /// The scaling factor in `(0, 1]` applied to the deductible input VAT (and thus
    /// to its ALV-ilmoitus field-307 contribution). `Full` scales by 1 (a no-op), so
    /// a code without a marker derives exactly as before.
    pub(super) fn factor(self) -> Decimal {
        match self {
            Deductibility::Full => Decimal::ONE,
            Deductibility::Partial(pct) => Decimal::from(pct) / Decimal::from(100),
        }
    }
}

/// The **taxation scheme** a VAT code selects, parsed as a `margin-<category>` infix
/// on the rate-class segment. [`Normal`](VatScheme::Normal) is the default: output
/// VAT is levied *on top of* the base (`base × rate`). [`Margin`](VatScheme::Margin)
/// is the second-hand margin scheme (marginaaliverotusmenettely, **AVL 79 a–k §**):
/// the tagged posting's amount is the VAT-**inclusive** profit margin (myyntihinta −
/// ostohinta) and the output VAT is extracted from *within* it (`margin × rate /
/// (100 + rate)`, AVL 79 k §), not added on top. Margin is a **sales-only** scheme (a
/// reseller's output VAT); a `purchase-margin-*` code, or a margin code that also
/// carries a partial-deductibility marker, is refused in [`parse_vat_code`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VatScheme {
    Normal,
    Margin(MarginCategory),
}

/// The category of goods a margin-scheme code covers (**AVL 79 a §** — used goods,
/// works of art, collectors' items, antiques, and — the common special case —
/// second-hand vehicles). All Finnish margin-scheme categories are taxed at the
/// **standard** domestic rate on the margin (the reduced art rate applies to an
/// artist's own direct sale / to imports, never to a dealer's margin resale), so
/// every category resolves to the same rate class ([`MARGIN_RATE_CLASS`]); the
/// category is retained for eligibility, ALV-ilmoitus reporting and audit
/// provenance, not to select a different percentage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MarginCategory {
    /// Used goods generally (käytetyt tavarat).
    UsedGoods,
    /// Works of art (taide-esineet).
    Art,
    /// Antiques (antiikkiesineet, as defined by AVL 79 a § via the EU CN 9706
    /// heading — the statute governs, not a bare "over 100 years" rule of thumb).
    Antiques,
    /// Collectors' items (keräilyesineet).
    Collectibles,
    /// Second-hand vehicles (käytetyt ajoneuvot) — the common special case of used
    /// goods, kept distinct because it has its own documentation/eligibility rules.
    Vehicle,
}

impl MarginCategory {
    /// Parse the category segment that follows `margin-` in a VAT code. An unknown
    /// category is refused loudly (`VAT_CODE`) rather than silently accepted, so a
    /// typo in a margin code can never resolve to a wrong (or zero) filing figure.
    fn parse(segment: &str) -> Result<MarginCategory, DomainError> {
        Ok(match segment {
            "used-goods" => MarginCategory::UsedGoods,
            "art" => MarginCategory::Art,
            "antiques" => MarginCategory::Antiques,
            "collectibles" => MarginCategory::Collectibles,
            "vehicle" => MarginCategory::Vehicle,
            other => {
                return Err(DomainError::VatCode(format!(
                    "unknown margin-scheme category {other:?} — expected one of \
                     used-goods, art, antiques, collectibles, vehicle (AVL 79 a §)"
                )))
            }
        })
    }
}

/// The infix that introduces a margin-scheme category on a VAT code's rate-class
/// segment (`sales-margin-used-goods`). A reserved word, like [`DEDUCT_MARKER`]:
/// `validate_vat_rate_table` refuses a rate class containing `margin` so a class
/// name can never be mistaken for the scheme infix.
pub(super) const MARGIN_MARKER: &str = "margin-";

/// Every margin-scheme category is standard-rated on the margin (see
/// [`MarginCategory`]); the dated percentage (25,5 % now, 24 % before 2024-09-01)
/// still resolves from this rate class + the document date, so **no** rate is baked
/// into the scheme. This constant is the single place a future per-category margin
/// rate would branch.
pub(super) const MARGIN_RATE_CLASS: &str = "domestic-standard";

/// The suffix that introduces a partial-deductibility marker on a VAT code.
pub(super) const DEDUCT_MARKER: &str = "-deduct";

/// Split an optional `-deduct<NN>` marker off the tail of a code's rate-class
/// segment, returning the bare rate class (for the dated-table lookup) and the
/// [`Deductibility`]. `NN` must be a **canonical** `1..=99` — non-empty ASCII
/// digits, no leading zero, in range — so a code has exactly one spelling per
/// meaning (codes are hash-stable ledger data). A present-but-malformed marker
/// (`-deduct0`, `-deduct05`, `-deduct100`, `-deductfoo`, or a trailing `-deduct`
/// with an empty rate class) is refused loudly with `VAT_CODE` rather than silently
/// folded back into the rate class, where it would surface only as an opaque
/// "unknown rate class" further down.
pub(super) fn split_deductibility(rest: &str) -> Result<(&str, Deductibility), DomainError> {
    let Some((class, pct_str)) = rest.rsplit_once(DEDUCT_MARKER) else {
        return Ok((rest, Deductibility::Full));
    };
    let canonical = !pct_str.is_empty()
        && pct_str.bytes().all(|b| b.is_ascii_digit())
        && !pct_str.starts_with('0');
    let pct = pct_str
        .parse::<u32>()
        .ok()
        .filter(|p| canonical && (1..=99).contains(p))
        .ok_or_else(|| {
            DomainError::VatCode(format!(
                "partial-deductibility marker {DEDUCT_MARKER}{pct_str:?} is not a canonical \
                 1..=99 percent — AVL 117 § partial deductibility is written e.g. \
                 purchase-domestic-standard-deduct50; 100 % uses the plain code and a \
                 wholly non-deductible acquisition books the VAT into cost with no \
                 deductible line"
            ))
        })?;
    if class.is_empty() {
        return Err(DomainError::VatCode(
            "VAT code has a deductibility marker but an empty rate class before it".into(),
        ));
    }
    // `rsplit_once` takes the *rightmost* marker; a second `-deduct` left in the
    // class part means a duplicated/embedded marker (`...-deduct50-deduct25`). Rate
    // classes never contain `deduct` (enforced in `validate_vat_rate_table`), so this
    // is unambiguously malformed — refuse it rather than pass a bogus rate class to
    // the dated-table lookup, where it would surface only as an opaque miss.
    if class.contains(DEDUCT_MARKER) {
        return Err(DomainError::VatCode(format!(
            "VAT code has more than one {DEDUCT_MARKER}<NN> partial-deductibility \
             marker (remaining rate class {class:?} still contains one) — exactly one \
             is allowed"
        )));
    }
    Ok((class, Deductibility::Partial(pct)))
}

/// Per-voucher, per-direction accumulation for the F4 structural guard
/// ([`check_voucher_vat_structure`]). `net` is the signed sum of contributions,
/// `gross` the sum of their absolute values, both over the VAT **base** (the
/// posting amount). A direction with nonzero-magnitude contributions that cancel
/// (`gross > 0` yet `net == 0`) necessarily has opposite signs — the same-direction
/// reverse-charge signature.
#[derive(Default)]
pub(super) struct DirectionAccum {
    pub(super) net: Decimal,
    pub(super) gross: Decimal,
}

impl DirectionAccum {
    pub(super) fn add(&mut self, signed: Decimal) {
        self.net += signed;
        self.gross += signed.abs();
    }

    /// True iff nonzero-magnitude VAT-tagged bases exist in this direction
    /// (`gross > 0`) but their signed contributions cancel exactly to zero.
    pub(super) fn cancels_to_zero(&self) -> bool {
        self.gross > Decimal::ZERO && self.net.is_zero()
    }
}

pub(super) const VAT_RATES_JSON: &str = include_str!("../../data/vat-rates.json");

pub(super) fn vat_rate_table() -> &'static VatRateTable {
    static TABLE: OnceLock<VatRateTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        let table: VatRateTable =
            serde_json::from_str(VAT_RATES_JSON).expect("bundled vat-rates.json parses");
        // A regulatory calculation must not depend on JSON array order: reject a
        // table where two editions of the same rate class cover the same date, or
        // an edition with an inverted `[valid_from, valid_to]`. Bundled data, so a
        // violation is a build-time bug — a unit test exercises this init.
        validate_vat_rate_table(&table).expect("bundled vat-rates.json is a valid rate table");
        table
    })
}

/// Reject overlapping / inverted rate editions per class (see [`vat_rate_table`]).
pub(super) fn validate_vat_rate_table(table: &VatRateTable) -> Result<(), DomainError> {
    use std::collections::BTreeMap;
    let mut by_class: BTreeMap<&str, Vec<&VatRateEdition>> = BTreeMap::new();
    for e in &table.editions {
        // `deduct` is a reserved word in a VAT code: `parse_vat_code` splits an
        // optional trailing `-deduct<NN>` partial-deductibility marker (AVL 117 §)
        // off the rate class, so a rate class containing `deduct` would be silently
        // reinterpreted (part of the name taken for a marker). Reject such a class at
        // load time — bundled data, so a violation is a build-time bug a unit test
        // exercises. Keeps the reserved-suffix invariant enforced, not just asserted.
        // Reserve exactly the two markers the parser reacts to, no wider: the
        // partial-deductibility marker is the substring `-deduct` (`rsplit_once` finds
        // it anywhere in the class), and the margin scheme is the `margin-` *prefix* of
        // the rate-class segment (`strip_prefix`). Matching the precise markers rather
        // than a bare `contains("deduct")`/`contains("margin")` keeps an innocent name
        // like `marginal-rate` allowed, while still reserving anything that would
        // actually misparse (`x-deductible` contains `-deduct`, so it stays reserved).
        if e.rate_class.contains("-deduct") || e.rate_class.starts_with("margin-") {
            return Err(DomainError::VatCode(format!(
                "rate class {:?} contains a reserved marker (the -deduct<NN> partial-\
                 deductibility suffix or the leading margin-<category> scheme infix) — \
                 it would misparse",
                e.rate_class
            )));
        }
        if let Some(to) = e.valid_to {
            if to < e.valid_from {
                return Err(DomainError::VatCode(format!(
                    "rate class {:?} has an edition with valid_to {} before valid_from {}",
                    e.rate_class, to, e.valid_from
                )));
            }
        }
        by_class.entry(&e.rate_class).or_default().push(e);
    }
    for (class, mut editions) in by_class {
        editions.sort_by_key(|e| e.valid_from);
        for pair in editions.windows(2) {
            let (prev, next) = (pair[0], pair[1]);
            // `prev` sorts first, so an overlap exists iff `prev` is open-ended or
            // its close reaches `next`'s open.
            let overlaps = prev.valid_to.is_none_or(|to| to >= next.valid_from);
            if overlaps {
                return Err(DomainError::VatCode(format!(
                    "rate class {class:?} has overlapping editions from {} and {}",
                    prev.valid_from, next.valid_from
                )));
            }
            // Contiguity: once a class has more than one edition it must cover its
            // whole span without a hole, otherwise `resolve_vat_rate` fails at query
            // time (a `VAT_CODE` "no edition for this date") on any voucher that lands
            // in the gap — a runtime brick from bundled data. A gap is easy to
            // introduce in the multi-edition temporary-measure classes (a mistyped
            // day, a dropped edition), so reject it at load time. `prev.valid_to` is
            // `Some` here (an open-ended `prev` would have overlapped above), and the
            // gap-free adjacency is `next.valid_from == prev.valid_to + 1 day`. A
            // *leading* gap before a class's first edition is intentional (pre-2013
            // dates refuse), so this only guards *between* editions.
            let to = prev
                .valid_to
                .expect("open-ended prev already rejected as overlap");
            let contiguous = to
                .inner()
                .succ_opt()
                .is_some_and(|day_after| day_after == next.valid_from.inner());
            if !contiguous {
                return Err(DomainError::VatCode(format!(
                    "rate class {class:?} has a gap between editions: it ends {} but the \
                     next edition does not start the following day ({})",
                    to, next.valid_from
                )));
            }
        }
    }
    Ok(())
}

/// Round to the EUR cent, half **away from zero**, pinned to 2 dp so `amount_str()`
/// is canonical — the per-line VAT rounding policy (mirrors payroll must-spec #7).
pub(super) fn round_eur(mut d: Decimal) -> Decimal {
    d = d.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
    d.rescale(2);
    d
}

/// Split a semantic VAT code into its `(direction, rate_class, deductibility,
/// scheme)`. The direction is the first segment; the rate class is the remainder
/// (resolved against the dated table), less any trailing `-deduct<NN>` partial-
/// deductibility marker (AVL 117 §), which becomes the [`Deductibility`]. A rate-
/// class segment of the form `margin-<category>` selects the margin scheme (AVL
/// 79 a–k §): it fixes the rate class to [`MARGIN_RATE_CLASS`] and carries the
/// category as a [`VatScheme::Margin`]. An unrecognised direction, an empty rate
/// class, a malformed marker, a marker on a `sales-` code, an unknown margin
/// category, a `purchase-margin-*` code, or a margin code that also carries a
/// deductibility marker is rejected loudly — nothing is silently dropped from a
/// filing aggregate.
pub(super) fn parse_vat_code(
    code: &str,
) -> Result<(VatDirection, &str, Deductibility, VatScheme), DomainError> {
    let (direction, rest) = if let Some(rest) = code.strip_prefix("sales-") {
        (VatDirection::Sales, rest)
    } else if let Some(rest) = code.strip_prefix("purchase-") {
        (VatDirection::Purchase, rest)
    } else {
        return Err(DomainError::VatCode(format!(
            "VAT code {code:?} has no sales-/purchase- direction prefix — \
             cannot classify it for a filing aggregate"
        )));
    };
    let (rate_class, deductibility) = split_deductibility(rest)?;
    // The second-hand margin scheme (AVL 79 a–k §) is a `margin-<category>` infix on
    // the rate-class segment. It fixes the rate class to the standard domestic rate
    // (MARGIN_RATE_CLASS) and carries the category, so the VAT can later be extracted
    // from *within* the margin rather than levied on top of it.
    if let Some(category_segment) = rate_class.strip_prefix(MARGIN_MARKER) {
        if direction != VatDirection::Sales {
            return Err(DomainError::VatCode(format!(
                "VAT code {code:?} puts the margin scheme on a {}- code; \
                 marginaaliverotus (AVL 79 a–k §) is a reseller's output VAT and \
                 applies to sales- codes only",
                direction.label()
            )));
        }
        if deductibility != Deductibility::Full {
            return Err(DomainError::VatCode(format!(
                "VAT code {code:?} combines the margin scheme with a partial-\
                 deductibility marker; margin output VAT is never fractional"
            )));
        }
        let category = MarginCategory::parse(category_segment)?;
        return Ok((
            VatDirection::Sales,
            MARGIN_RATE_CLASS,
            Deductibility::Full,
            VatScheme::Margin(category),
        ));
    }
    if rate_class.is_empty() {
        return Err(DomainError::VatCode(format!(
            "VAT code {code:?} has an empty rate class"
        )));
    }
    if deductibility != Deductibility::Full && direction != VatDirection::Purchase {
        return Err(DomainError::VatCode(format!(
            "VAT code {code:?} puts a partial-deductibility marker on a {}- code; \
             AVL 117 § partial deductibility applies to input (purchase-) VAT only — \
             output VAT on a sale is never fractional",
            direction.label()
        )));
    }
    Ok((direction, rate_class, deductibility, VatScheme::Normal))
}

/// Resolve the VAT percentage in force for `rate_class` on `document_date`. Fails
/// with `VAT_CODE` if no edition covers the date (rates are dated data, never
/// extrapolated) or the class is unknown.
pub(super) fn resolve_vat_rate(
    rate_class: &str,
    document_date: Date,
) -> Result<Decimal, DomainError> {
    let table = vat_rate_table();
    let edition = table
        .editions
        .iter()
        .find(|e| {
            e.rate_class == rate_class
                && document_date >= e.valid_from
                && e.valid_to.is_none_or(|to| document_date <= to)
        })
        .ok_or_else(|| {
            DomainError::VatCode(format!(
                "no VAT rate edition for class {rate_class:?} on {document_date} \
                 (rates are dated data; add the edition)"
            ))
        })?;
    edition.percent.parse::<Decimal>().map_err(|e| {
        DomainError::VatCode(format!(
            "VAT rate class {rate_class:?} has a malformed percent {:?}: {e}",
            edition.percent
        ))
    })
}

/// Return whether a semantic VAT code resolves to a non-zero tax rate on the
/// supplied source-document date.
///
/// This is the narrow public query import adapters use before accepting a legacy
/// `vat_code`. It deliberately shares the filing parser and dated rate table:
/// malformed directions, unknown classes, and uncovered dates return `VAT_CODE`
/// instead of being mistaken for zero tax. The result says only whether the rate
/// is non-zero; it does not calculate or alter a posting.
pub fn vat_code_is_tax_bearing(code: &VatCode, document_date: Date) -> Result<bool, DomainError> {
    let (_, rate_class, _, _) = parse_vat_code(code.as_str())?;
    Ok(!resolve_vat_rate(rate_class, document_date)?.is_zero())
}
