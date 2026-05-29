# Security policy

## Supported versions

my-fam-tree is pre-1.0. Security fixes are applied to `main` and the most recent release branch.

## Reporting a vulnerability

Please **do not open a public issue** for security vulnerabilities. Instead:

1. Open a private security advisory on the project repository, or
2. Email the maintainers at `security@my-fam-tree.eu` (substitute with the project's real contact once the hosted instance launches).

Include:

- A description of the issue and its impact.
- Steps to reproduce.
- Affected versions / commits.
- Any proof-of-concept code (please mark it clearly).

We aim to acknowledge reports within 72 hours and provide an initial assessment within 7 days.

## Scope

In scope:

- The Rust Stack (`crates/*`).
- The Vue frontend (`fe/`).
- Container images built from `.docker/`.
- The hosted instance operated by the project owner [my-fam-tree.eu](https://my-fam-tree.eu).

Out of scope (please don't report):

- Issues that require physical access to a user's device.
- Self-XSS issues that require the victim to paste attacker-supplied content.
- Vulnerabilities in third-party dependencies that we already track via Dependabot/Renovate.
- Denial-of-service through resource exhaustion against the hosted instance — please report capacity concerns separately.

## Disclosure

Once a fix is released we will:

- Credit the reporter (with consent).
- Publish a GitHub Security Advisory.
- Cut a patched release and update affected branches.
