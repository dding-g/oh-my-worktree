---
title: CLI–TUI Capability Parity Master Execution Plan
date: 2026-08-13
status: approved-planning-baseline
planning_style: PRD plus outcome roadmap
scope: all GA user-facing CLI semantic capabilities and their TUI equivalents
non_negotiable: TUI is an interactive wrapper over CLI capabilities, not an independent product surface
related:
  - docs/reviews/2026-08-13-cli-tui-parity-evaluation.md
  - docs/reviews/2026-08-13-cli-tui-parity-roadmap.md
  - docs/reviews/2026-08-10-product-opportunity-review.md
  - docs/solutions/performance/worktree-loading-performance-plan.md
  - docs/ssot/03-cli-tui-use-case-contract.md
  - docs/ssot/04-git-operation-safety-policy.md
---

# CLI–TUI Capability Parity Master Execution Plan

## 1. Summary

`owt` has one worktree-lifecycle product with two interfaces: a scriptable CLI and a keyboard-first TUI. The CLI owns semantic capability; the TUI must let a user complete every GA CLI job with the same validation, safety rules, side effects, and result.

This plan replaces duplicated CLI/TUI orchestration incrementally. It does not make the TUI spawn the `owt` binary. Both interfaces call a shared typed use-case layer, while CLI output and TUI interaction remain appropriate to each surface.

## 2. Contacts and operating assumptions

| Role | Owner | Responsibility |
|---|---|---|
| Product/technical decision | Maintainer | Accepts capability scope, exceptions, and release gates |
| Delivery | Maintainer or contributor | Owns one vertical slice end to end |
| Review | Maintainer + reviewer | Verifies shared core, safety, and surface parity |
| QA | Maintainer | Runs fixture, TUI scenario, cross-platform, and release checks |

No team capacity, velocity, user telemetry, or external issue backlog is available. This plan uses ordered horizons instead of dates or point commitments. A horizon finishes only when its measurable exit criteria pass.

## 3. Background and problem statement

CLI and TUI currently orchestrate related Git behavior in separate locations. This already created practical parity debt:

- CLI prune performed completed-PR cleanup while TUI `x` only pruned stale metadata.
- CLI search covers path/name/branch/status/PR; TUI filtering covers name/branch only.
- CLI create supports base, explicit destination, and per-run tmux choice; TUI supports only base selection.
- CLI PR, commit tree, help, version, clone, init, and setup have incomplete or absent TUI equivalents.

The cleanup discrepancy is addressed in the current working tree: CLI and TUI use `worktree_prune::run_prune`, and TUI `x` previews eligible/excluded candidates before explicit confirmation. This remains uncommitted at plan creation.

## 4. Objective and measurable outcomes

### Product objective

Enable a terminal developer to use CLI automation or TUI interaction interchangeably without changing the worktree-lifecycle result or weakening safety.

### North-star

`semantic parity coverage = Full GA CLI capabilities / all GA CLI semantic capabilities = 100%`

### Key results and guardrails

| Result | Target | Evidence |
|---|---:|---|
| Capability registry coverage | 100% of GA CLI capabilities classified | registry test and review |
| New capability parity | 100% have TUI mapping or approved exemption | CI contract test |
| Shared mutation boundary | 0 surface mutation paths bypass shared use-case | code inventory + scenario tests |
| Destructive false positive | 0 | delete/prune live-status tests |
| CLI compatibility | 0 default TSV/help regressions | CLI tests |
| TUI launch performance | no more than 10% regression from baseline | benchmark |
| Windows support | strict cross-target compile plus release-runner smoke | CI evidence |

## 5. Scope and exceptions

### In scope

- Every GA user-facing CLI semantic capability.
- Equivalent TUI entry point, input flow, progress, result, and error state.
- Shared request/result types and one safety/mutation boundary.
- CLI/TUI scenario parity tests, SSOT, README, keybinding/help, and release gates.
- TUI global entry for CLI functions that can run outside a repository.

### Explicit exemptions

| Capability | Why exempt | Enforcement |
|---|---|---|
| TSV/JSON/color serialization | transport renderer concern, not an interactive job | registry `transport_only` |
| shell completion | shell-owned integration | registry `shell_integration` |
| `test-cd` | internal debug command | registry `internal_debug` |

No other GA CLI command is exempt without owner, rationale, and review condition.

## 6. Capability baseline and target map

| ID | CLI semantic capability | Current TUI state | Target TUI equivalent | Horizon |
|---|---|---|---|---|
| C01 | Open/list repository worktrees | Full | list view and refresh | H0 |
| C02 | Clone `.bare` workspace | Full in working tree | global home → clone flow | H5 |
| C03 | Show init conversion guide | Full in working tree | global home → guide entry | H5 |
| C04 | Install shell setup | Full in working tree | global home → setup execute/result | H5 |
| C05 | Create with base/path/tmux | Full in working tree | progressive add flow; explicit path and per-create tmux override | H3 |
| C06 | Delete with force/branch | Full | batch-capable confirm flow | H0 |
| C07 | Prune completed-PR worktrees | Full in working tree | cleanup preview + confirm | H1 |
| C08 | Check PR status by selected/arbitrary/all branch | Full in working tree | PR detail/query modal | H4 |
| C09 | Browse commit tree with limit | Full in working tree | full-screen tree with adjustable limit | H4 |
| C10 | Search path/name/branch/status/PR | Full in working tree | unified search mode | H2 |
| C11 | Browse CLI help | Full in working tree | global CLI help browser | H5 |
| C12 | Show version/about | Full in working tree | Global Home About modal | H5 |

Target state after H5: C01–C12 are Full, and C05/C07 are committed rather than only present locally.

## 7. Architecture decision

```text
CLI parser ─┐
            ├─> typed use-case ─> Git/config/tmux/shell adapters
TUI event ──┘          │
                       ├─> CLI output renderer
                       └─> TUI state + view model
```

1. `src/main.rs` parses CLI arguments and renders text; it does not own business orchestration.
2. `src/app.rs` maps keystrokes/modal input to requests and renders progress; it does not replicate validation or mutation policy.
3. `src/git.rs` remains the low-level Git adapter.
4. A use-case owns request validation, target resolution, live safety check, mutation ordering, progress/result events, and typed errors.
5. TUI does not invoke `owt` as a subprocess.
6. A preview never authorizes a mutation; execute always re-evaluates live Git state.
7. Extract only what a vertical slice needs; do not block delivery on a repository-wide refactor.

## 8. Delivery horizons

### H0 — Governance and baseline

**Outcome:** Maintainers can detect capability drift before it ships.

| Work item | Acceptance criteria | Dependency |
|---|---|---|
| Capability registry | C01–C12, mapping, state, exemption, owner recorded in checked data | none |
| Parity test harness | GA capability with no mapping/scenario fails CI | registry |
| Startup baseline | first-frame time and Git command count for 1/10/30/100 worktrees | fixture generator |
| Contract update | SSOT says CLI semantic SSOT and GA TUI parity are required | accepted `ssot:update` |
| Cleanup commit | current shared prune core, preview, docs, and tests committed separately | current worktree |

**Exit gate:** registry complete; existing Full entries have scenario tests; cleanup slice committed and green.

### H1 — Cleanup parity

**Status:** implementation is complete in the working tree; integration/commit is pending.

**Outcome:** CLI and TUI cleanup have one candidate model and one destructive safety boundary.

| Required evidence | Acceptance criterion |
|---|---|
| Shared core | CLI and TUI call `run_prune`; no surface candidate logic |
| TUI preview | removable and excluded reasons visible; `Enter/y` confirm, `Esc/n` cancel |
| Execute recheck | live status and candidate rules run again during execute |
| CLI behavior | interactive `--dry-run` candidate selection remains available |

**Exit gate:** committed code; native/Windows strict Clippy; manual fixture shows no deletion after cancel and no deletion of unknown, dirty, current, HEAD, bare, or detached worktrees.

### H2 — Search parity

**Outcome:** A user finds the same worktrees through CLI and TUI for the same query.

| Story | Acceptance criteria |
|---|---|
| Shared `WorktreeQuery` | path, display name, branch, status label, PR label use one tested matcher |
| TUI filter | `/` supports the same field set; match presentation is documented |
| PR data | unavailable PR stays `-` and never blocks search |
| Parity fixtures | CLI/TUI match sets are identical for path/status/PR/name/branch |

**Risk:** unloaded metadata must remain visibly distinct from `clean` and must preserve fail-closed rules.

### H3 — Create parity

**Outcome:** TUI users can make every CLI create decision without slowing the common path.

| Story | Acceptance criteria |
|---|---|
| Shared create request | CLI/TUI produce the same branch, base, target path, tmux intent, source/copy/script plan |
| Progressive add UI | common create remains branch + Enter; advanced section exposes base, path, tmux default/on/off |
| Create preview | resolved target, base ref, copy files, post-add, tmux actions are visible before mutation |
| Shared validation | collision, fetch, copy, script, and handoff behavior match CLI |

**Guardrail:** default create adds at most one keystroke versus current behavior.

### H4 — Read-context parity

**Outcome:** PR and commit investigation no longer requires leaving TUI for CLI.

| Story | Acceptance criteria |
|---|---|
| PR status modal | selected worktree, all worktrees, and arbitrary branch query; GitHub fallback stays `-` |
| Commit tree screen | graph, paging, and selectable limit represent CLI `commit tree -n` |
| Shared read models | CLI renderer and TUI view derive from same PR/commit result |
| Scenarios | every CLI PR/commit test has TUI semantic equivalent |

### H4P — Performance lane

**Outcome:** TUI opens quickly with many worktrees while mutations stay live-checked.

| Story | Acceptance criteria |
|---|---|
| Fast loader | first frame uses cheap worktree-list data only |
| Bounded enrichment | visible-first status/commit/ahead/PR refresh uses bounded concurrency and path application |
| Benchmark gate | 30-worktree warm p95 target is set from baseline and met |
| Safety proof | cache/unloaded metadata never authorizes delete/prune |

### H5 — Global capability parity

**Outcome:** Users can discover, initialize, configure, and understand `owt` through TUI before a repository is selected.

| Story | Acceptance criteria |
|---|---|
| Global home | repository launch bypasses home to preserve direct list entry |
| Clone | URL/path flow shares CLI clone use-case and result/error behavior |
| Init guide | same conversion guide content and copyable command as CLI |
| Setup | same shell detection and secure write/symlink policy; preview before write |
| Help/about | command help browser and exact version information |

## 9. Sequencing and Definition of Ready

1. Commit cleanup parity.
2. Build registry and parity harness.
3. Deliver H2 search parity.
4. Deliver H3 create parity.
5. Deliver H4 PR/commit parity and H4P performance lane.
6. Deliver H5 global capability parity.
7. Only then expand with Doctor, JSON renderer, and PR Review Workspace.

A story is ready only with a capability ID, user job, CLI request/result contract, TUI entry/state, safety invariants, fixtures, documentation impact, and declared performance impact.

## 10. Test and release plan

| Layer | What it proves |
|---|---|
| Unit | matchers, request validation, candidate/reason decisions |
| Git integration | isolated repositories, dirty/unknown/submodule paths |
| CLI contract | parsing, stdout shape, help, errors, side effects |
| TUI app state | key → request, progress, cancel/confirm, result |
| Adapter parity | same fixture/request gives same core semantic outcome |
| Cross-platform | native and `x86_64-pc-windows-msvc` strict Clippy |
| Manual smoke | terminal restoration, global home, shell setup, cleanup/create |

Required behavior-change gates:

```text
cargo fmt -- --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
rustup run stable cargo clippy --target x86_64-pc-windows-msvc --all-targets --all-features -- -D warnings
git diff --check
```

Run version-sync and npm-package checks when user-facing release assets change.

A release is blocked if a GA CLI capability is Missing/Partial without approved exemption, or if a TUI scenario duplicates destructive validation outside the shared core.

## 11. Documentation plan

| Change type | Required updates |
|---|---|
| CLI/TUI capability or keybinding | SSOT 03, README/README.ko, npm README when relevant, keybindings, TUI help |
| Git safety | SSOT 04, usage worktrees, regression tests, durable solution doc for new pattern |
| Global shell/setup | SSOT 05, shell docs, README, agent install assets |
| Release/version | version-sync inputs, docs homepage, npm README, CI |
| Durable architecture | `docs/solutions/` and project map |

Planning remains in `docs/reviews/`; accepted current behavior moves to SSOT.

## 12. Risks and decision rules

| Risk | Early signal | Mitigation | Decision rule |
|---|---|---|---|
| Shared core weakens safety | adapter does live check/mutation itself | contract tests before extraction | stop until core owns guard |
| Modal overload | basic create takes more steps | progressive disclosure and preview | change interaction, not parity goal |
| Global UI slows repo launch | direct list entry disappears | bypass home in repo | split global flow if needed |
| Registry is paperwork | mapping without scenario | CI requires scenario ID | do not mark Full without evidence |
| Performance regression | first-frame p95 worsens | benchmark per read-path change | defer enrichment |
| Unverified demand | no dogfood task-time improvement | collect workflow timing/feedback | prioritize discovery before large new features |

## 13. Completion audit

The initiative is complete only when:

- C01–C12 are Full or explicitly exempt.
- CLI and TUI share core behavior for create, delete, prune, PR status, commit tree, search, clone, init, and setup.
- Every mutation has one live safety boundary and failure-path coverage.
- Registry, SSOT, docs, TUI help, and tests agree.
- Native and Windows gates pass.
- TUI first-frame benchmark is within the guardrail.
- No parity code or planning artifact is unintentionally left uncommitted outside release scope.

## 14. Immediate next actions

1. Commit the completed cleanup, registry, search, and create parity work with planning documents.
2. Implement PR status and commit-tree read-context parity (H4).
3. Measure startup baseline before fast-loader work.
4. Implement global clone/init/setup/help/about parity (H5).

## SSOT 변경 작업 계약

### Goal

- CLI의 사용자 의미를 정본으로 두고, 모든 GA CLI capability에 동일한 TUI 완료 경로를 강제하는 계약을 `SSOT 03`에 반영한다.

### Scope In

- `docs/ssot/03-cli-tui-use-case-contract.md`의 CLI/TUI 동등성, 검색, 검증 규칙.
- `src/capability_registry.rs`의 C01–C12 추적과 CLI/TUI 공용 검색 matcher.

### Scope Out

- TSV/JSON/color 출력 직렬화, shell completion, `test-cd` internal debug command.
- Git mutation 안전 규칙, regular repository 및 `.bare` layout 정책, shell trust boundary의 의미 변경.

### Constraints

- CLI parser와 TUI event는 binary subprocess가 아닌 공용 typed use-case 또는 matcher를 사용한다.
- preview는 mutation 권한이 아니며, destructive execute는 live state를 재검증한다.
- SSOT 산문은 한국어로 유지하고, 현재 미완료 capability는 숨기지 않고 release blocker로 기록한다.

### Success Criteria

- C01–C12가 코드상 registry에 한 번씩만 기록되고, Full 항목은 traceable scenario를 가진다.
- `/` TUI filter와 `owt search`가 path/name/branch/status/PR label의 동일 matcher를 사용한다.
- SSOT, keybinding/help, README가 이 사용자 계약과 일치한다.

### Assumptions

- 사용자의 “CLI가 지원하는 기능은 모두 TUI에서 지원” 지시를 기존 GA capability의 완료 기준으로 해석한다.
- 예외는 이 계획의 명시적 transport-only/shell-integration/internal-debug 항목으로 제한한다.

### Open Questions

- 없음. capability별 화면 설계는 각 horizon의 구현 전에 현재 CLI request/result와 test evidence로 구체화한다.

### Execution Plan

1. registry와 공용 검색 matcher를 추가한다.
2. 검색 동등성 test와 사용자 문서를 갱신한다.
3. `SSOT 03`에 parity release gate를 반영하고 표준 품질 게이트로 검증한다.
