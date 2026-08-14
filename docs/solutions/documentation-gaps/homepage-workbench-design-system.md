---
title: Homepage Workbench Design System
date: 2026-08-13
category: docs/solutions/documentation-gaps
module: documentation_homepage
problem_type: design_maintenance
component: docs/index.html
severity: low
applies_when:
  - changing docs/index.html homepage presentation
  - updating homepage styles, screenshots, or responsive layout
  - changing homepage language or theme controls
tags: [homepage, design-system, i18n, responsive, github-pages]
---

# Homepage Workbench Design System

## Context

`docs/index.html` is a static GitHub Pages homepage for a CLI/TUI product. Its
primary proof is the real TUI screenshot, not fabricated terminal or browser
chrome. The page keeps its existing English/Korean and dark/light controls.

## Implementation Pattern

- Put homepage design tokens in `docs/tokens.css`; use semantic `--color-*`,
  `--font-*`, spacing, motion, and radius tokens instead of new inline values.
- Put redesign overrides in `docs/homepage.css`, loaded after the legacy inline
  stylesheet. This keeps the original static page and behavior intact while
  allowing an incremental visual redesign.
- Keep the marketing page in a Workbench shape: an asymmetric hero pairs the
  product explanation with the real TUI capture; practical workflows and
  commands are the supporting proof.
- The navigation uses CLI vocabulary and the footer remains a compact technical
  colophon. Do not regress to a generic multi-column SaaS footer.
- Preserve `data-i18n` keys and the existing text-node translation mechanism.
  New user-visible homepage copy needs an English/Korean equivalent; commands,
  paths, and options remain literal.

## Verification

For homepage-only changes, run:

```text
git diff --check
```

Then render the static page and check 320, 375, 414, 768, and 1280 px widths
for horizontal overflow and wrapped actionable text. Verify that language and
theme toggles still update their accessible labels and persisted state.

## Related

- `docs/index.html`
- `docs/tokens.css`
- `docs/homepage.css`
- `docs/ssot/06-documentation-release-asset-policy.md`
