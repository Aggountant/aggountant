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
| `WageReportsToIR.xsd` | `c913b56c9ef8b6fa5254823731fb069f1f976d58346980911d0f928b81c4f83c` |
| `WageReportsToIRTypes.xsd` | `cec426861a27123a7d6f704ad772188c0083eba160a81847a0cb525691005a79` |
| `InvalidationsToIR.xsd` | `d2346fc908cc273b028c2b196f7490a273c7927d5cd702688d461374772ef86d` |
| `InvalidationsToIRTypes.xsd` | `26e2e305518f5ff8e99fc22d056294ea006c19a4f57a938c6f39edad575bac27` |
| `IRCommonTypes.xsd` | `c90cb61893ac01692c04f366827704d5552bc70a7c60a83fbae2c768a73236b4` |
| `xmldsig-core-schema.xsd` | `2c0b2cb702fe4332f4dae217e9d3e407bd2a5bc3472e3fc776d915d299288f5a` |

The wage and cancellation schemas were copied from the same ZIP on 2026-11-21.

`crates/broker/tests/payer_summary_schema.rs` validates the generated employer's
separate report, the travel report with its replacement, and the cancellation
record against these files. Any standard validator can also be used,
for example `xmllint --noout --schema PayerSummaryReportsToIR.xsd record.xml`.
