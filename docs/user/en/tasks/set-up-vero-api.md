# Set up direct VAT filing

With direct filing the agent sends the VAT return to the Tax Administration
through the Vero API, the Tax Administration's interface for software, using
your company's own credentials. Setting it up has several steps and waiting
times, so start in good time. Until then you file the return
[in OmaVero](file-vat-omavero.md).

## Before you start

- aggountant has no shared software key. Every user registers their own
  software in the Tax Administration's API portal and gets their own key, and
  the company obtains its own certificate.
- You do the portal, agreement and certificate steps yourself, because they
  are legal acts of the company. The agent gathers the information, checks
  readiness, runs the checks with your permission and tells you whose turn it
  is to clear each blocker.
- This page describes the steps in as much detail as aggountant checks them.
  Their exact names and order are in the Tax Administration's instructions.

## What you need from the Tax Administration

1. **Register your software** in the Vero API portal and keep the software
   key it gives you.
2. **Make the company's API agreement**, that is, the Vero API agreement. A
   submitted application is not enough: the agreement must be in force
   before you confirm it to the agent.
3. **Apply for a production certificate** for your company from the Tax
   Administration's certificate service. You get the certificate, its private
   key and the certificate authority's (CA) certificate.
4. **Choose the VAT interface's acceptance tests in the portal.** The agent
   runs them in step 3 below.
5. **Get production permission.** Once the tests have been accepted, you
   activate production use in the portal.

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

   The agent writes the Business ID and the contact person's name and phone
   number to the home's `vero-filing.json` file. The mode starts as
   `disabled`. The credential file's path is stored in the machine's own
   configuration file and not in the home, so each machine can have its own
   credentials. Then the agent checks readiness and tells you which blockers
   remain and whose they are.

3. **Give permission for the acceptance tests.** Once you have chosen the
   tests in the portal, the agent runs the Tax Administration's three fixed
   test scenarios with your production certificate. It asks for permission
   because the run is the first connection to the Tax Administration's
   production service. The run files no return and changes no state. When it
   succeeds, the program stores the evidence in the settings and records the
   tests as passed.

4. **Confirm the portal steps to the agent** one at a time, once you have
   done the registration, the agreement and the production permission in the
   portal. The program cannot check them and takes your word for it, so
   confirm only what you have already done.

5. **Approve the switch to production.** The agent writes the settings again
   in the mode `production` and binds the production activation to the
   current certificate's fingerprint. If the readiness check does not pass,
   the program refuses and nothing changes.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant vero-fi status
aggountant vero-fi configure --dry-run < settings.json
aggountant vero-fi configure < settings.json
aggountant vero-fi verify-portal
```

`status` checks readiness locally without a network connection, so it can be
run at any time. The program runs the same check just before every
submission.

`configure` merges the given fields into the stored settings. A field the
input leaves out stays as it was, and `--dry-run` lists every field that
would change. The program refuses an input that would clear or change the
evidence `verify-portal` recorded. `--replace` writes the input as the whole
settings file, and is needed only to drop evidence on purpose or to overwrite
a broken settings file.

`verify-portal` runs the acceptance tests.

`aggountant vero-fi verify --target test` is optional. It is for
troubleshooting against the test environment with separate test credentials,
and production does not need it.
</details>

## Done when

`aggountant vero-fi status` reports that readiness is in order (`ready`) and
there are no blockers.

## When the certificate changes

The certificate is valid for two years, and the program warns 90, 30 and 7
days before it expires. A new certificate has a new fingerprint, so evidence
bound to the old certificate is no longer valid:

1. Put the new certificate and key in the credential file.
2. Ask the agent to run the acceptance tests again and switch to production
   with the new fingerprint. The program binds the test evidence and the
   production activation to the new certificate.

Your confirmations of the registration, the agreement and the production
permission stay valid, and you do not need to give them again.

When you need an identifier for the certificate, for example for the Tax
Administration's support, use the SHA-256 fingerprint that `vero-fi status`
shows. It is not secret.

## If something goes wrong

- If blockers remain, each has a code and an owner. They are listed in
  [Vero.fi readiness blockers](../reference/vero-fi-blockers.md).
- If the software key or the certificate's private key has leaked, contact
  the Tax Administration as you would when any other credential leaks, and
  get new ones.

## See also

- [Send the VAT return directly to the Tax Administration](file-vat-direct.md)
- [The company home: what is stored where](../concepts/company-home.md)
- [Report travel allowances to Tulorekisteri](report-travel-allowances.md)
