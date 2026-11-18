# Incomes Register schemas (vendored, unmodified)

These XSD files are byte-for-byte copies from the Incomes Register's
"Technical interface, ZIP for application developers 2025"
(`tekninen-rajapinta-zip-sovelluskehittajalle-2025.zip`, published 8 January 2025,
SHA-256 `ca9b8c322136f3a2087918c6e51fb5821ee47b046829821f5e9b2dd9d22ceaf8`),
downloaded on 2026-11-16 from
<https://www.vero.fi/globalassets/tulorekisteri/dokumentaatio-2025/tekninen-rajapinta-zip-sovelluskehittajalle-2025.zip>.
The ZIP is linked from <https://www.vero.fi/en/incomes-register/software-developers/documentation/>.

The 2025 package is the one in force for reporting periods in 2026; the 2027
package (published 29 May 2026) changes `PayerSummaryReportsToIRTypes.xsd` for
reporting periods from January 2027 (optional `EmployeeGroups`, `Transactions`
becomes optional). Review that change before the first 2027 report.

| File | SHA-256 |
|---|---|
| `PayerSummaryReportsToIR.xsd` | `d802b7fc4aadc4fa5dbc1d5fc03f8ce9dd14004c3b126c1be1734ef3233c5852` |
| `PayerSummaryReportsToIRTypes.xsd` | `48d2be073d87a57365549819cf45015051e45fa008a5fe97323ca02341decfa0` |
| `IRCommonTypes.xsd` | `c90cb61893ac01692c04f366827704d5552bc70a7c60a83fbae2c768a73236b4` |
| `xmldsig-core-schema.xsd` | `2c0b2cb702fe4332f4dae217e9d3e407bd2a5bc3472e3fc776d915d299288f5a` |

`crates/broker/tests/payer_summary_schema.rs` validates the generated employer's
separate report against these files. Any standard validator can also be used,
for example `xmllint --noout --schema PayerSummaryReportsToIR.xsd record.xml`.
