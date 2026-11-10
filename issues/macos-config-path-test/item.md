---
created: 2026-11-10
updated: 2026-11-10
type: bug
reporter: conductor
status: fixed
priority: normal
closed: 2026-11-10
---

# vero-fi CLI test assumes the Linux config path

## Description

Hosted macOS CI fails configure_splits_non_secret_profile_from_platform_credential_path: the CLI correctly writes ~/Library/Application Support/aggountant/config.json on macOS, but the test reads ~/.config/aggountant/config.json.

## Resolution

### 2026-11-10T13:28:12Z · @issuectl

Fixed in the preceding commit.
