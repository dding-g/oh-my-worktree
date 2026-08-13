---
title: Windows CRLF release-version verification
date: 2026-08-13
category: docs/solutions/release
module: release_automation
problem_type: cross_platform_ci
component: version_sync
severity: high
applies_when:
  - changing scripts/sync-release-version.js
  - debugging a release workflow that passes locally but fails on Windows
tags: [windows, crlf, release, version-sync]
---

# Windows CRLF release-version verification

## Context

The release workflow verifies version mirrors on every release target. Windows checkouts may present `Cargo.lock` with CRLF line endings even when the Git blob uses LF.

## Rule

Any `Cargo.lock` parser in `scripts/sync-release-version.js` must accept both `\n` and `\r\n`. Use `\r?\n` in structural regexes rather than assuming an LF-only checkout.

## Validation

- Run `node scripts/sync-release-version.js v<version> --check` locally.
- Run an in-memory CRLF variant of the `Cargo.lock` package-header pattern.
- Keep the Windows target in `.github/workflows/release.yml`; release verification must happen before compilation.

## Related

- `scripts/sync-release-version.js`
- `.github/workflows/release.yml`
- `docs/ssot/06-documentation-release-asset-policy.md`
