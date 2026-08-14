---
title: Fail-Closed Worktree Mutation Boundaries
date: 2026-08-10
category: docs/solutions/security
module: worktree_safety
problem_type: security_and_data_safety
component: git_and_configured_file_copy
severity: high
applies_when:
  - changing worktree deletion or prune behavior
  - changing copy_files path handling
  - adding post-create filesystem actions
  - changing worktree status collection
tags: [worktree, deletion, copy-files, path-traversal, fail-closed]
---

# Fail-Closed Worktree Mutation Boundaries

## Context

Worktree creation and removal cross two important trust boundaries: configured paths can write into a new worktree, and cached UI metadata can become stale before deletion. Both boundaries must fail closed. A status lookup failure is not evidence that a worktree is clean, and a configured path is not safe merely because it was joined to a trusted directory.

The policy contracts live in `docs/ssot/02-configuration-trust-boundary-policy.md` and `docs/ssot/04-git-operation-safety-policy.md`.

## Guidance

### Configured file copies

`src/copy_files.rs::copy_configured_files` owns the production copy implementation used by `src/main.rs`.

- Accept only non-empty relative paths made of normal components.
- Reject absolute paths, `..`, root/prefix components, and destination symlinks.
- Canonicalize the source and verify that it remains under the canonical source root.
- Walk destination parent components individually; create missing directories, but reject existing symlinks and non-directory components.
- Keep tests against the production helper. Do not introduce a test-only copy implementation that can drift from runtime behavior.

These checks prevent path traversal, source-symlink escape, and destination-symlink overwrite.

### Worktree status and deletion

`src/git.rs::parse_worktree_list` may collect status for display, but display metadata is not deletion authorization.

- A status command failure becomes `WorktreeStatus::Unknown`, never `Clean`.
- `src/git.rs::remove_worktree` must query the live status immediately before a non-force removal.
- Only `WorktreeStatus::Clean` may pass the non-force guard; dirty, conflicted, mixed, and unknown states must remain untouched.
- `src/worktree_prune.rs` may identify candidates from PR metadata, but the removal boundary must still perform the live clean check.
- Fetch failures during worktree creation must stop the operation instead of silently continuing from a stale base branch.

## Verification

Preserve regression coverage for:

- parent-directory and absolute-path rejection;
- source and destination symlink escape prevention;
- nested relative copy success;
- status lookup failure producing `unknown`;
- dirty worktree removal refusal;
- prune retaining an unknown-status worktree;
- worktree creation stopping when base-branch fetch fails.

Run `cargo test` and strict Clippy after changing these boundaries.

## When to Apply

Use this document when editing `copy_files`, add-worktree post-processing, worktree status derivation, delete confirmation, prune candidate selection, or Git fetch behavior.

## Related

- `docs/ssot/02-configuration-trust-boundary-policy.md`
- `docs/ssot/04-git-operation-safety-policy.md`
- `docs/solutions/best-practices/ai-agent-project-map.md`
- `src/copy_files.rs`
- `src/git.rs`
- `src/worktree_prune.rs`

