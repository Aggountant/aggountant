#!/usr/bin/env bash
# Runs the practice month of docs/user/fi/start/practice-month.md against an
# aggountant binary in a throwaway home, as the agent would, and checks the
# figures the tutorial quotes.
#
# Usage: run-practice-month.sh [path-to-aggountant]   (default: aggountant on PATH)
# The throwaway home is created under $TMPDIR and removed on exit.
set -euo pipefail

bin="${1:-aggountant}"
here="$(cd "$(dirname "$0")" && pwd)"
work="$(mktemp -d "${TMPDIR:-/tmp}/aggountant-practice.XXXXXX")"
[[ -n "${KEEP_PRACTICE_HOME:-}" ]] || trap 'rm -rf "$work"' EXIT
home="$work/harjoitus"
ag() { "$bin" --home "$home" "$@"; }
agj() { "$bin" --home "$home" --json "$@"; }
fail() { echo "practice-month: $*" >&2; exit 1; }
expect() { [[ "$2" == "$3" ]] || fail "$1: expected '$3', got '$2'"; }

statement="$here/tiliote-2026-03.xml"
receipt="$here/kuitti-toimistotarvike-2026-03.txt"

# 1. Practice home.
ag init

# 2. Bank statement: two booked rows, the pending row is skipped.
rows="$(agj import camt053 parse --file "$statement" | jq '.transactions | length')"
expect "parsed rows" "$rows" 2
assignments='{
  "actor": "agent",
  "assignments": [
    {"bank_ref": "ESIM-2026-0314-001", "bank_account": "1910", "document_date": "2026-03-11",
     "contra_postings": [{"account": "7690", "amount": {"amount": "124.00", "currency": "EUR"}}],
     "vat_treatment": {"type": "domestic-input", "rate_class": "domestic-standard"}},
    {"bank_ref": "ESIM-2026-0320-002", "bank_account": "1910",
     "contra_postings": [{"account": "2000", "amount": {"amount": "2500.00", "currency": "EUR"}}]}
  ]
}'
ag import camt053 seal --file "$statement" <<<"$assignments"
ag import camt053 seal --file "$statement" <<<"$assignments" >/dev/null
count="$(agj voucher verify | jq '.voucher_count')"
expect "vouchers after re-import" "$count" 2

# 3. Receipt paid with the owner's own card.
ref="$(agj artifact attach --file "$receipt" | jq -er '.artifact_ref')"
receipt_json="$(jq -n --arg ref "$ref" '{
  natural_key: "receipt:toimistotarvike-2026-03-22",
  actor: "agent",
  source: {amount: {amount: "31.00", currency: "EUR"}, document_date: "2026-03-22",
           description: "Toimistotarvike Oy, tulostinpaperi ja kirjekuoret", attachment_ref: $ref},
  proposal: {posting_date: "2026-03-22", net_account: "7700", contra_account: "2740",
             vat: {type: "domestic-input", rate_class: "domestic-standard"}}
}')"
hash="$(agj capture-receipt record --dry-run <<<"$receipt_json" | jq -er '.preparation_hash')"
ag capture-receipt record --approved-preparation-hash "$hash" <<<"$receipt_json"
replayed="$(agj capture-receipt record --approved-preparation-hash "$hash" <<<"$receipt_json" | jq '.replayed')"
expect "receipt replay" "$replayed" true
ag voucher verify

# 4. Monthly report.
ag report statements --period 2026-03 --company "Esimerkki Oy" --format html \
  --output "$work/esimerkki-2026-03.html"
ag report statements --period 2026-03 --company "Esimerkki Oy" --format native-json \
  --output "$work/esimerkki-2026-03.json" >/dev/null
report="$work/esimerkki-2026-03.json"
expect "month result" "$(jq -r '.sections.month_income.result' "$report")" "-123.50"
expect "assets total" "$(jq -r '.sections.balance.assets_total' "$report")" "2407.50"
expect "equity and liabilities total" "$(jq -r '.sections.balance.equity_liabilities_total' "$report")" "2407.50"
expect "bank account" "$(jq -r '[.. | objects | select(.account == "1910") | .amount][0]' "$report")" "2376.00"

# 5. Close the month.
ag obligation list
run_id="$(agj run start close-period --period 2026-03 | jq -er '.run_id')"
echo "run $run_id started"
ag period show --period 2026-03
ag run answer --run-id "$run_id" --gate ApproveClose --answer approve
ag period show --period 2026-03
late="$(jq '.natural_key = "receipt:toimistotarvike-2026-03-22-b"' <<<"$receipt_json")"
late_hash="$(agj capture-receipt record --dry-run <<<"$late" | jq -er '.preparation_hash')"
if out="$(ag capture-receipt record --approved-preparation-hash "$late_hash" <<<"$late" 2>&1)"; then
  fail "sealing into a sealed month succeeded"
fi
echo "$out"

# 6. VAT figures.
vat="$(agj file-vat preview --period 2026-03)"
expect "field 307" "$(jq -r '.filing_fields["307"].amount' <<<"$vat")" "31.50"
expect "field 308" "$(jq -r '.filing_fields["308"].amount' <<<"$vat")" "-31.50"
expect "payment" "$(jq -r '.payment_amount' <<<"$vat")" "0.00"
ag file-vat preview --period 2026-03
echo "practice-month: ok"
