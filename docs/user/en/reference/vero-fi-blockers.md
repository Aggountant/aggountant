# Vero.fi readiness blockers

This page lists the blockers and warnings that `aggountant vero-fi status` can
show. The check is local and makes no network connection. The same check runs
just before every direct VAT return.

A direct return is possible only when readiness is in order (`ready`) and
there are no blockers. Every blocker has an owner: `human` means your
information, confirmation or portal step, `agent` a setting the agent can fix.
The setup steps are on
[Set up direct VAT filing](../tasks/set-up-vero-api.md).

## Your blockers (`human`)

| Blocker | What it means |
|---|---|
| `VERO_DISABLED` | direct filing is not switched on (the settings' mode is not `production`) |
| `VERO_BUSINESS_ID_MISSING` | the Business ID is missing from the settings |
| `VERO_CONTACT_NAME_MISSING` | the contact person's name is missing from the settings |
| `VERO_CONTACT_PHONE_MISSING` | the contact person's phone number is missing from the settings |
| `VERO_SOFTWARE_REGISTRATION_UNATTESTED` | you have not confirmed the software registration |
| `VERO_API_AGREEMENT_UNATTESTED` | you have not confirmed the API agreement |
| `VERO_PRODUCTION_VERIFICATION_UNATTESTED` | the acceptance tests have not been recorded as passed |
| `VERO_PRODUCTION_PERMISSION_UNATTESTED` | you have not confirmed the production permission |
| `VERO_CREDENTIAL_BUSINESS_ID_MISMATCH` | the Business ID in the credential file does not match the settings |
| `VERO_DELEGATED_USE_UNSUPPORTED` | the credentials carry a delegation identifier; filing on behalf of another company is not supported |
| `VERO_CERTIFICATE_NOT_YET_VALID` | the certificate is not valid yet |
| `VERO_CERTIFICATE_EXPIRED` | the certificate has expired |
| `VERO_ACTIVATION_CERTIFICATE_MISMATCH` | production was activated with a different certificate than the current one |

## The agent's blockers (`agent`)

| Blocker | What it means |
|---|---|
| `VERO_PROFILE_INVALID` | the home's `vero-filing.json` is missing or invalid; shown when the settings have not been made yet |
| `VERO_PORTAL_VERIFICATION_MISSING` | there is no evidence of an acceptance test run |
| `VERO_PORTAL_VERIFICATION_CERTIFICATE_MISMATCH` | the acceptance tests were run with a different certificate than the current one |
| `VERO_CREDENTIAL_PATH_MISSING` | the credential file's path has not been set |
| `VERO_CREDENTIAL_IN_ACCOUNTING_HOME` | the credential file is inside the books home |
| `VERO_CREDENTIAL_UNAVAILABLE` | the credential file cannot be read |
| `VERO_CREDENTIAL_INVALID` | the credential file is invalid |
| `VERO_CREDENTIAL_TARGET_MISMATCH` | the credentials are for the test environment, not production |

## Warnings

A warning does not block filing.

| Warning | What it means |
|---|---|
| `VERO_CERTIFICATE_EXPIRES_WITHIN_90_DAYS` | the certificate expires within 90 days |
| `VERO_CERTIFICATE_EXPIRES_WITHIN_30_DAYS` | the certificate expires within 30 days |
| `VERO_CERTIFICATE_EXPIRES_WITHIN_7_DAYS` | the certificate expires within 7 days |

When the certificate changes, see
[Set up direct VAT filing](../tasks/set-up-vero-api.md#when-the-certificate-changes).

## See also

- [Set up direct VAT filing](../tasks/set-up-vero-api.md)
- [Error codes](error-codes.md)
