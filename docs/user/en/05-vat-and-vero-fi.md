# 5. VAT and Vero.fi

In Finland VAT is called ALV (arvonlisävero), and the return is filed with the
Finnish Tax Administration (Verohallinto), whose online services live at
Vero.fi. The ALV return is made from a closed period's figures. You can file
the figures derived from the books in two ways:

- **directly with the Tax Administration** through its Vero API interface, once
  your company has set it up (below)
- **by hand** in OmaVero, the Tax Administration's online service, or through
  Ilmoitin.fi, its file-filing service. The return is then recorded in
  aggountant as filed.

Either way, the tax is paid separately in OmaVero or in your bank. aggountant
pays nothing.

## The figures to report

Once the period is closed (chapter 4), the agent previews the return:

```sh
aggountant file-vat preview --period 2026-03
```

The preview only reads the books. It starts no run, reads no credentials and
opens no network connection. The result contains the return's numbered fields
in euros, the VAT on sales and on purchases, the net amount and the amount
payable. For example, a month with a single €124.00 purchase receipt gives
€25.20 in field 307 (deductible tax for the period) and −€25.20 in field 308
(tax payable, or refundable when negative). Nothing is payable.

For a period that has not been closed, the preview refuses
(`PERIOD_NOT_SEALED`). Settle any questions about the material before the
close. After the close and the return, a correction means a replacement return,
which aggountant does not make.

The preview's instruction text currently always tells you to enter the figures
by hand in OmaVero, even when direct filing is set up. That text is out of date.
Whether you can use the direct route depends only on whether Vero.fi readiness
is in order.

## Vero.fi onboarding

Direct filing is your company's own connection to the Tax Administration.
**Every user registers their own software and gets their own software key**
from the Tax Administration's API portal, and the company obtains its own
certificate. aggountant has no shared key you could borrow. A return is always
sent with your company's own credentials.

You do the portal, agreement and certificate steps yourself, because they are
legal acts of the company. The agent does not act in the portals for you. It
collects the details, checks readiness, runs the checks with your permission
and tells you whose turn it is to clear each blocker. The exact names and order
of the steps are in the Tax Administration's instructions. Here they are
described at the level of detail aggountant checks them.

### What you need from the Tax Administration

1. **Software registration and software key.** Register your software in the
   Vero API portal and keep the software key it gives you.
2. **API agreement.** The company's Vero API agreement or application.
3. **Production certificate.** Apply for a certificate for your company from
   the Tax Administration's certificate service. You receive the certificate,
   its private key and the certificate authority (CA) certificate. The
   certificate is valid for two years.
4. **ALV acceptance tests.** In the portal, choose the acceptance tests for the
   VAT interface. aggountant runs them for you (below).
5. **Production permission.** Once the tests are accepted, production use is
   activated in the portal.

### Credentials on your machine

Save the credentials yourself as a JSON file, for example
`~/.config/aggountant/credentials/vero-production.json`. The file contains:

- `target`: `"production"`
- `business_id`: your company's Business ID (Y-tunnus)
- `software_key`: your software key
- `client_certificate_pem`, `client_private_key_pem` and `ca_certificate_pem`:
  the certificate, its private key and the CA certificate in PEM format.

The file must be outside the company home and must never be added to git. Set
its permissions so that only you can read it. Do not paste credentials into the
conversation with the agent. When you need to identify the certificate, for
example to the Tax Administration's support, use the SHA-256 fingerprint shown
by `vero-fi status`. It is not secret.

The software key is yours. If it leaks, contact the Tax Administration as you
would for any other leaked credential.

### Checking readiness

```sh
aggountant vero-fi status
```

The check is local and opens no network connection, so you can run it at any
time. The same check runs automatically just before every submission. The
result shows whether readiness is in order (`ready`), together with blockers,
warnings and the certificate's validity. Every blocker has an owner: `human`
means information or a portal step of yours, `agent` means a setting the agent
can fix.

| Blocker | What it means |
|---|---|
| `VERO_BUSINESS_ID_MISSING`, `VERO_CONTACT_NAME_MISSING`, `VERO_CONTACT_PHONE_MISSING` | the Business ID or the contact person's details are missing from the settings |
| `VERO_SOFTWARE_REGISTRATION_UNATTESTED` | you have not confirmed the software registration |
| `VERO_API_AGREEMENT_UNATTESTED` | you have not confirmed the API agreement |
| `VERO_PRODUCTION_VERIFICATION_UNATTESTED` | the acceptance tests have not been recorded as passed |
| `VERO_PRODUCTION_PERMISSION_UNATTESTED` | you have not confirmed the production permission |
| `VERO_PORTAL_VERIFICATION_MISSING` | there is no evidence of an acceptance-test run |
| `VERO_CREDENTIAL_PATH_MISSING`, `VERO_CREDENTIAL_UNAVAILABLE`, `VERO_CREDENTIAL_INVALID` | the credential file is not configured, cannot be read, or is invalid |
| `VERO_CREDENTIAL_IN_ACCOUNTING_HOME` | the credential file is inside the company home |
| `VERO_CREDENTIAL_BUSINESS_ID_MISMATCH` | the Business ID in the credentials does not match the settings |
| `VERO_CREDENTIAL_TARGET_MISMATCH` | the credentials are for the test environment, not production |
| `VERO_DELEGATED_USE_UNSUPPORTED` | the credentials carry an authorisation token. Filing on behalf of another company is not supported. |
| `VERO_CERTIFICATE_EXPIRED`, `VERO_CERTIFICATE_NOT_YET_VALID` | the certificate is not valid |
| `VERO_ACTIVATION_CERTIFICATE_MISMATCH`, `VERO_PORTAL_VERIFICATION_CERTIFICATE_MISMATCH` | the production activation or the test evidence was made with a different certificate than the current one |
| `VERO_DISABLED` | direct filing is not switched on |

The program warns you 90, 30 and 7 days before the certificate expires.

### Onboarding steps

1. **Basic settings.** The agent writes the home's `vero-filing.json` with
   `aggountant vero-fi configure`. The mode starts as `disabled`, and the file
   holds the Business ID, the contact person's name and phone number, and the
   path to the credential file. The path is stored in the machine's own
   configuration file, not in the home, so each machine can have its own
   credentials. The `--dry-run` option shows the result without writing
   anything.
2. **Acceptance tests.** Once you have chosen the tests in the portal and the
   credentials are in place, the agent runs them:

   ```sh
   aggountant vero-fi verify-portal
   ```

   The command runs the Tax Administration's three fixed test scenarios with
   your production certificate. It files nothing and changes no state, but it
   is the first connection to the Tax Administration's production
   environment. That is why the agent asks your permission for it. A
   successful run stores the evidence in the settings and records the tests as
   passed.
3. **Attestations.** Once you have completed the registration, the agreement
   and the production permission in the portal, you confirm them to the agent
   one at a time. The program cannot check them and relies on your word, so
   confirm only what has been done, not what you are expecting.
4. **Production.** The agent rewrites the settings in `production` mode and
   binds the production activation to the current certificate's fingerprint.
   The program accepts production mode only if the readiness check passes.
   Otherwise it refuses (`VERO_NOT_READY`) and nothing changes.

`configure` replaces the whole settings file. The agent has to carry the
earlier evidence over, or the test evidence is lost and production is blocked.
The `vero-fi` skill guides the agent through this.

`aggountant vero-fi verify --target test` is an optional troubleshooting check
against the test environment, with separate test credentials. It is not needed
for production.

### Renewing the certificate

A new certificate has a new fingerprint, so evidence bound to the old
certificate is no longer valid. When you change certificates, run
`verify-portal` again and switch to production with the new fingerprint.
Confirm to the agent the attestations that concern the new certificate.

## Filing the ALV return directly

1. **The preview** (above) gives the figures.
2. **The filing run is started** with the preview's figures:

   ```sh
   aggountant file-vat start < return.json
   ```

   The input contains the period, the VAT on sales, the VAT on purchases and
   the Tax Administration's bank account number (`payee_iban`), shown in
   OmaVero. The program recomputes the figures from the books and refuses if
   they differ (`VAT_FIGURES_MISMATCH`). The run checks readiness, fetches the
   period's status from the Tax Administration and waits for your approval
   (`ApproveFiling`). The period must not yet have been filed with the Tax
   Administration.
3. **You approve.** The agent shows the company, the period, the due date, the
   numbered fields, the net amount and the amount payable. The approval is tied
   to exactly this content to be sent:

   ```sh
   aggountant file-vat approve-filing --run-id fv-… --answer approve
   ```

   Rejecting (`--answer reject`) ends the run. A new round starts from the
   preview.
4. **Submission.** When the Tax Administration receives the return, the program
   stores its acknowledgement (an identifier and a timestamp) and marks the
   period as filed.

You can see the run's steps at any time with
`aggountant run show --run-id fv-…`.

**Payment.** If tax is payable, the run then opens a payment approval
(`ApprovePayment`). aggountant has no payment connection, so this step cannot
complete, and the run keeps showing a waiting state. The return has still been
filed. Pay the tax yourself with the OmaVero reference. A return that results
in a refund ends without a payment step.

### The return does not show in OmaVero

The acknowledgement means the Tax Administration received the return, not that
it has already been processed. In practice OmaVero has shown a return sent
through the interface only after about half a day. According to the Tax
Administration's support, a delay of about 12 hours is normal. This is a
practical observation, not a promised service level.

**Do not send the return again because it does not show yet.** A second
submission would be a replacement return at the Tax Administration, and
aggountant does not even allow a second submission for the same period
(`PERIOD_TRANSITION_CONFLICT`). Wait until the next working day. If the return
still does not show, or the due date is close, call the Tax Administration's
VAT service. For the call, the agent puts together the period, the
acknowledgement identifier and timestamp, the Business ID and the fields that
were sent.

### Uncertain outcome

If the connection drops, or the Tax Administration's service answers with an
error in the middle of a submission, the program does not know whether the
return arrived. The run then stays in a state that needs attention, and the
return is not sent again. The agent asks the Tax Administration for the status:

```sh
aggountant file-vat reconcile --run-id fv-…
```

If the Tax Administration has exactly the same return, the run ends without a
new submission. If the return does not show, or it differs, the run keeps
waiting, because the Tax Administration may not yet show the latest data. Wait
and ask again, or check OmaVero. A return is never resent blindly.
`file-vat retry` applies only to the payment step.

## Filing by hand in OmaVero

If direct filing is not set up, or you want to do it yourself, enter the
preview's numbered fields in OmaVero (or prepare an Ilmoitin.fi file yourself)
and pay the tax. Then archive the OmaVero confirmation and record the return as
filed:

```sh
aggountant artifact attach --file omavero-confirmation-2026-03.pdf
aggountant file-vat complete-manual --period 2026-03 --channel omavero \
  --evidence-ref <confirmation identifier>
```

The channel is `omavero` or `ilmoitin`. The command sends and pays nothing. It
marks the period as filed, which is a permanent state. If an uncertain
interface submission is left over for the same period, the command also needs
evidence that the Tax Administration's status has been checked
(`--remote-status-evidence-ref`). Otherwise a return filed by hand could
silently be a duplicate.

## Monthly and quarterly periods

The ALV return is filed for the same period the books are closed for: a month
(`2026-03`) or a quarter (`2026-Q1`). Direct filing has been tested with
monthly periods.
