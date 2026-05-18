---
title: Repository Layout Documentation Contract
date: 2026-05-18
category: docs/solutions/documentation-gaps
module: documentation_policy
problem_type: documentation_gap
component: documentation
severity: medium
applies_when:
  - changing regular repository or .bare behavior
  - updating README, docs site, npm README, AGENTS, or CLAUDE guidance
  - reviewing docs for stale bare-first positioning
tags: [ssot, documentation, worktree-layout, regular-repo, bare-layout]
---

# Repository Layout Documentation Contract

## Context

`owt` used to be described as a bare-repository-oriented tool. Since v0.10.0, regular non-bare repositories are first-class. Documentation must now preserve a two-layout product contract instead of drifting back to bare-first language.

The canonical policy lives in `docs/ssot/01-repository-worktree-policy.md`.
Use `docs/ssot/00-ssot-index.md` to find adjacent SSOT contracts for CLI/TUI use cases, config trust, Git operation safety, shell integration, and docs/release assets.

## Guidance

Every user-facing or agent-facing doc must preserve these claims:

| Claim | Required wording intent |
|---|---|
| Product shape | `owt` manages Git worktrees from regular repositories and bare repository layouts. |
| Regular repos | Users can run `owt` directly in an existing regular non-bare Git repository. No conversion is required. |
| Regular repo path | New regular-repo worktrees default to `~/.owt/worktree/<repo-name>/` unless `worktree_root` is configured. |
| `.bare` layout | `.bare` remains supported and is created by `owt clone`; it is a recommended project-local sibling layout, not the only supported shape. |
| `owt init` | Conversion guide for users who prefer `.bare`, not a prerequisite. |
| Config trust | Project config cannot enable automatic post-add script execution; only global config can opt into that trust boundary. |

## Docs Taxonomy

| Doc | Role |
|---|---|
| `README.md` | Primary English product contract. |
| `README.ko.md` | Korean mirror of the product contract. |
| `npm/README.md` | Short package-facing product contract; easy to drift because it duplicates summary content. |
| `docs/index.md` | GitHub Pages homepage and quick-start summary. |
| `docs/getting-started/quick-start.md` | Onboarding order: existing regular repo, new `.bare` project, optional conversion. |
| `docs/usage/worktrees.md` | Add/switch/delete worktree behavior and layout-specific creation locations. |
| `docs/concepts/bare-repository.md` | Explains bare repos as useful for project-local sibling workflows. |
| `docs/concepts/worktree-pattern.md` | Explains `.bare` as supported/recommended, not required. |
| `docs/reference/configuration.md` | Defines `worktree_root` and post-add script trust boundary. |
| `AGENTS.md` | Primary agent operations guide. |
| `CLAUDE.md` | Compatibility guide; currently a drift risk because `AGENTS.md` is primary. |

## Why This Matters

Agents often update one doc after a behavior change and leave duplicates stale. For this repo, stale docs are risky because the product positioning is part of the behavior contract: users must know they can start from an existing regular repository.

## When to Apply

- Any change to repository detection, worktree creation paths, `worktree_root`, `owt clone`, or `owt init`.
- Any README, docs site, npm README, AGENTS, or CLAUDE wording change involving Git repository layouts.
- Any release note or docs homepage refresh.

## Examples

Use this safe framing:

```md
Run `owt` directly inside an existing regular Git repository, or use `owt clone` to create the project-local `.bare` sibling layout.
```

Avoid this stale framing:

```md
Convert your repository to bare before using `owt`.
```

## Related

- `docs/ssot/01-repository-worktree-policy.md`
- `README.md`
- `README.ko.md`
- `docs/usage/worktrees.md`
- `docs/reference/configuration.md`
