: "${AGGOUNTANT_BIN:=aggountant}"
ARTIFACT_REF=$(jq -er '.artifact_ref' "$D/artifact.json") || exit $?
"$AGGOUNTANT_BIN" --home "$HOME_DIR" --json voucher list \
  --attachment-ref "$ARTIFACT_REF" \
  > "$D/source-candidates.json" 2> "$D/source-candidates-error.json" || exit $?
jq -e '.schema_version == 1 and (.count == (.vouchers | length)) and
  (.truncated | type == "boolean")' "$D/source-candidates.json" >/dev/null || exit $?
if ! jq -e '.truncated == false' "$D/source-candidates.json" >/dev/null; then
  echo "source candidate list is truncated; stop and ask before capture" >&2
  exit 1
fi
jq -r '.vouchers[].number' "$D/source-candidates.json" \
  > "$D/source-candidate-numbers" || exit $?
while IFS= read -r VOUCHER_NUMBER; do
  "$AGGOUNTANT_BIN" --home "$HOME_DIR" --json voucher show "$VOUCHER_NUMBER" \
    > "$D/source-candidate-${VOUCHER_NUMBER}.json" \
    2> "$D/source-candidate-${VOUCHER_NUMBER}-error.json" || exit $?
done < "$D/source-candidate-numbers"
