---
title: Compound Engineering Workflow for owt
date: 2026-05-18
category: docs/solutions/developer-experience
module: compound_engineering
problem_type: developer_experience
component: development_workflow
severity: medium
applies_when:
  - creating durable project knowledge for future agents
  - finishing architecture, behavior, docs, or release-workflow changes
  - updating AGENTS.md or SSOT guidance
tags: [compound-engineering, ai-agents, documentation, workflow]
---

# Compound Engineering Workflow for owt

## Context

The repository now has several durable knowledge layers:

- `AGENTS.md` for operational guidance.
- `docs/ssot/00-ssot-index.md` for the policy-contract map.
- `docs/ssot/01-repository-worktree-policy.md` for the repository-layout policy contract.
- `docs/solutions/` for reusable lessons and project maps.

Compound engineering means keeping those layers current so future AI agents do not rediscover the same context from scratch.

## Guidance

Use this workflow after any change that teaches something durable about the project.

1. Decide whether the learning belongs in `AGENTS.md`, `docs/ssot/`, `docs/solutions/`, or user-facing docs.
2. If the learning is an operational rule for agents, update `AGENTS.md`.
3. If the learning is a product or behavior contract, update the relevant SSOT first.
4. If the learning is a reusable implementation/doc/process pattern, add or refresh a `docs/solutions/` compound doc.
5. Cross-link related docs so future agents know the reading order.
6. Validate with `git diff --check`; run `cargo test` when Rust behavior changed.

## Knowledge Placement Rules

| Learning type | Destination | Example |
|---|---|---|
| Agent rule | `AGENTS.md` | “For git/worktree behavior changes, add or update regression tests first.” |
| Product contract | `docs/ssot/` | regular repo vs `.bare` layout policy |
| Reusable project context | `docs/solutions/best-practices/` | architecture map for future AI agents |
| Docs drift issue | `docs/solutions/documentation-gaps/` | duplicated README/docs layout positioning |
| Workflow improvement | `docs/solutions/developer-experience/` | compound engineering process |
| User-facing instructions | README or `docs/` site | quick start, keybindings, config |

## Why This Matters

The commit history shows recurring categories of hard-won knowledge:

- Shell integration required `/dev/tty`, secure output files, and a wrapper function.
- Git commands needed sanitized `GIT_*` environment variables.
- Worktree behavior changed from bare-first to regular-repo-first-class.
- Documentation became part of the product contract through the Korean SSOT.
- UI-visible data changes often span both Git formatting and Ratatui rendering.

Without compound docs, future agents will likely repeat old mistakes: overfitting to `.bare`, weakening the post-add trust boundary, or touching UI/docs without matching tests and policy updates.

## When to Apply

- After fixing a non-obvious bug.
- After changing repository layout, config trust, shell integration, Git command behavior, keybindings, docs, or release assets.
- Before large AI-agent handoffs or onboarding sessions.

## Examples

When changing `worktree_root`, update this set together:

```text
src/git.rs or src/config.rs
tests/git_test.rs or src/git.rs tests
README.md
README.ko.md
docs/reference/configuration.md
docs/usage/worktrees.md
docs/ssot/01-repository-worktree-policy.md
docs/solutions/documentation-gaps/repository-layout-documentation-contract.md if the policy changes
```

When changing UI-visible recent commit formatting, expect both data and rendering work:

```text
src/git.rs          # git log format and tests
src/ui/main_view.rs # rendering split/styling
README/docs         # only if user-facing behavior or screenshot docs change
```

## Related

- `AGENTS.md`
- `docs/solutions/best-practices/ai-agent-project-map.md`
- `docs/solutions/documentation-gaps/repository-layout-documentation-contract.md`
- `docs/ssot/01-repository-worktree-policy.md`
