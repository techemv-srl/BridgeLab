# Security Policy

## Supported Versions

Only the latest released version of BridgeLab receives security fixes.
Older releases are not patched — please update to the current release
before reporting an issue you can no longer reproduce there.

| Version        | Supported          |
| -------------- | ------------------ |
| 1.3.x (latest) | :white_check_mark: |
| < 1.3          | :x:                |

## Reporting a Vulnerability

Please report vulnerabilities **privately** — do not open a public issue.

- Preferred: GitHub private vulnerability reporting — *Security* tab →
  *Report a vulnerability* on this repository.
- Alternatively: e-mail **info@techemv.it** with subject `[SECURITY] BridgeLab`.

Include what you can: affected version, platform, reproduction steps or a
proof of concept, and the impact you see. If you want credit in the release
notes, tell us the name/handle to use.

**What to expect:** we acknowledge reports within **2 business days**, share
an initial assessment within **7 days**, and keep you updated while we work
on a fix. Fixes ship in a patch release; timing depends on severity, with
critical issues taking priority over everything else. We ask that you give
us a reasonable window (up to 90 days) before any public disclosure —
we will credit you when the fix is published.

## Scope

In scope:

- The BridgeLab application (this repository) and its installers.
- The license activation service (`license-api.techemv.it`) — but please
  **no volume/DoS testing and no automated scanning** against it: it is a
  small production service. A single proof-of-concept request is enough.

Out of scope:

- Denial of service, rate-limit exhaustion, spam.
- Social engineering of TECHEMV staff or customers.
- Vulnerabilities in third-party dependencies with no BridgeLab-specific
  impact — please report those upstream (a heads-up to us is still welcome).
- Circumvention of the license checks by modifying or rebuilding the
  application: the core is open source and the licensing is by design a
  fair-play mechanism, not a DRM boundary. Flaws that expose **customer or
  license data**, on the other hand, are very much in scope.

## Handling of clinical data

BridgeLab processes HL7/FHIR content locally on the user's machine and never
uploads message content to TECHEMV. If you find any behavior that contradicts
this, treat it as a critical report.
