# Result: close-year-plain-help

`close-year --help` and its subcommands no longer use design labels (RF6/S6,
D7, `SealYear`, reconciliation). The group help says what the year-end close
does; `start` says it puts nothing in the books; `seal-correction` explains
what a correction is (a balanced entry for the closing year the program does
not calculate: a reclassification, an error found in the closed months, a
depreciation calculated elsewhere) and that it is permanent once sealed;
`resume` says it finishes what the user approved, including the year's seal;
`answer` says the two middle gates seal nothing. The `start` input help
describes corrections the same way, and close-year error messages say
"correction" instead of "D7 correction". A CLI test asserts no help page
carries the labels.

Landed with `close-year-next-command`; review and assessment are in that
issue's directory.
