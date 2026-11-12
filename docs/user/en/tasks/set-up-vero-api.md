# Set up direct VAT filing

Direct filing is your company's own connection to the Tax Administration: the
agent sends the VAT return through the Vero API, the Tax Administration's
interface for software, using your company's own credentials. Setting it up
has several steps and waiting times, so start in good time. Until then you
file the return [in OmaVero](file-vat-omavero.md). The end of this page tells
you what to do when the certificate changes.

## Before you start

- **Every user registers their own software and gets their own software key**
  from the Tax Administration's API portal, and the company obtains its own
  certificate. aggountant has no shared key that you could borrow.
- You do the portal, agreement and certificate steps yourself, because they
  are legal acts of the company. The agent does not use the portals on your
  behalf. It gathers the information, checks readiness, runs the checks with
  your permission and tells you whose turn it is to clear each blocker.
- The exact names and order of the steps are in the Tax Administration's
  instructions. This page describes them in as much detail as aggountant
  checks them.

## What you need from the Tax Administration

1. **Software registration and software key.** Register your software in the
   Vero API portal and keep the software key it gives you.
2. **API agreement.** The company's Vero API agreement or application.
3. **Production certificate.** Apply for a certificate for your company from
   the Tax Administration's certificate service. You get the certificate, its
   private key and the certificate authority's (CA) certificate. The
   certificate is valid for two years.
4. **VAT acceptance tests.** Choose the VAT interface's acceptance tests in
   the portal. aggountant runs them for you (step 3 below).
5. **Production permission.** Once the tests have been accepted, production
   use is activated in the portal.

## How to do it

1. **Save the credentials in a file yourself.** Create a JSON file outside
   the books home, for example
   `~/.config/aggountant/credentials/vero-production.json`. It holds:
   - `target`: `"production"`
   - `business_id`: your company's Business ID
   - `software_key`: your software key
   - `client_certificate_pem`, `client_private_key_pem` and
     `ca_certificate_pem`: the certificate, its private key and the CA
     certificate in PEM format.

   Set the permissions so that only you can read the file, and do not add it
   to git. **Do not paste the credentials into the conversation with the
   agent**, because conversations are often saved in logs. Tell the agent only
   the file's path.

2. **Ask the agent to do the basic settings.**

   > Let's set up direct VAT filing. The credentials are in the file
   > ~/.config/aggountant/credentials/vero-production.json.

   The agent writes the home's `vero-filing.json` file: Business ID, contact
   person's name and phone number. The mode starts as `disabled`. The
   credential file's path is stored in the machine's own configuration file,
   not in the home, so each machine can have its own credentials. Then the
   agent checks readiness and tells you the remaining blockers and their
   owners.

3. **Give permission for the acceptance tests.** Once you have chosen the
   tests in the portal, the agent runs the Tax Administration's three fixed
   test scenarios with your production certificate. The run makes no return
   and changes no state, but it is the first connection to the Tax
   Administration's production service, which is why the agent asks for
   permission. A successful run stores the evidence in the settings and
   records the tests as passed.

4. **Confirm the portal steps.** Once you have done the registration, the
   agreement and the production permission in the portal, you confirm them to
   the agent one at a time. The program cannot check them and takes your word
   for it, so **confirm only what has been done, not what you expect**.

5. **Approve the switch to production.** The agent writes the settings again
   in the mode `production` and binds the production activation to the
   current certificate's fingerprint. The program accepts production mode only
   if the readiness check passes. Otherwise it refuses, and nothing changes.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant vero-fi status
aggountant vero-fi configure --dry-run < settings.json
aggountant vero-fi configure < settings.json
aggountant vero-fi verify-portal
```

`configure` replaces the whole settings file. The agent must carry the earlier
evidence along, or the test evidence is lost and production is blocked. The
`vero-fi` skill guides the agent in this.

`aggountant vero-fi verify --target test` is optional troubleshooting against
the test environment with separate test credentials. It is not needed for
production.
</details>

## Done when

`aggountant vero-fi status` reports that readiness is in order (`ready`) and
there are no blockers. The check is local and makes no network connection, so
it can be run at any time. The same check runs automatically just before
every submission.

## When the certificate changes

The certificate is valid for two years, and the program warns 90, 30 and 7
days before it expires. A new certificate has a new fingerprint, so evidence
bound to the old certificate is no longer valid:

1. Put the new certificate and key in the credential file.
2. Ask the agent to run the acceptance tests again and switch to production
   with the new fingerprint.
3. Confirm to the agent the portal steps that concern the new certificate.

When you need an identifier for the certificate, for example for the Tax
Administration's support, use the SHA-256 fingerprint that `vero-fi status`
shows. It is not secret.

## If something goes wrong

- **Blockers remain.** Every blocker has a code and an owner:
  [Vero.fi readiness blockers](../reference/vero-fi-blockers.md).
- **The software key or the certificate's private key has leaked.** Contact
  the Tax Administration as you would when any other credential leaks, and
  get new ones.

## See also

- [Send the VAT return directly to the Tax Administration](file-vat-direct.md)
- [The company home: what is stored where](../concepts/company-home.md)
- [Report travel allowances to Tulorekisteri](report-travel-allowances.md)
