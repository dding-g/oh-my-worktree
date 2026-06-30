---
title: owt Git Operation / Safety 정책
description: Git command 실행, worktree operation, background operation safety 계약
ref:
  - src/git.rs
  - src/app.rs
  - src/types.rs
  - tests/git_test.rs
  - .agents/skills/owt-worktree/SKILL.md
  - docs/usage/git-operations.md
  - docs/usage/worktrees.md
---

# 1. 문서 목적

이 문서는 `owt`가 Git CLI를 호출할 때 지켜야 하는 operation/safety 계약의 정본이다.

```yaml
document_contract:
  source_of_truth_for:
    - git_command_execution_boundary
    - worktree_operation_contract
    - background_operation_contract
    - clean_state_and_conflict_policy
  not_source_of_truth_for:
    - exact stderr wording for every git failure
    - ratatui spinner rendering
```

# 2. Git Command 실행 경계

모든 Git operation은 `std::process::Command`로 `git` CLI를 직접 호출한다. Hook이나 shell integration 환경에서 전달된 Git 환경 변수가 repo 탐지를 오염시키면 안 된다.

```yaml
git_command_policy:
  command: git
  always_remove_env:
    - GIT_DIR
    - GIT_WORK_TREE
    - GIT_INDEX_FILE
    - GIT_COMMON_DIR
  reason: "외부 hook/shell 환경이 `owt`의 repository detection과 operation target을 오염시키지 않게 한다."
```

# 3. Worktree Operation 계약

| Operation | Trigger | 구현 경계 | 성공 계약 | Safety rule |
|---|---|---|---|---|
| list | TUI load/refresh | `git worktree list --porcelain` + optional GitHub/gh-style PR lookup | bare entry와 non-bare worktree를 구분하고, GitHub PR 상태가 확인되면 list metadata로 표시한다 | bare entry는 status/ahead/behind 계산 대상이 아니며 PR lookup 실패는 list를 실패시키거나 block하지 않는다 |
| add | `a` modal confirm | `git worktree add` | branch/base 정책에 맞는 worktree 생성 | 생성 후 usable worktree인지 확인/repair한다 |
| delete | `d` confirm | `git worktree remove` + optional branch delete | 선택 worktree 제거. `Space`로 체크한 worktree가 있으면 체크된 대상 전체에 적용 | dirty worktree는 기본적으로 삭제하지 않는다 |
| prune | `owt worktree prune` | `git worktree prune -v` + `gh pr list` 단일 조회 기반 completed PR worktree scan; `--dry-run`은 `git worktree prune --dry-run -v`와 serial candidate review | stale metadata를 정리하고 완료된 worktree를 병렬 제거하며 모든 worktree 판단 로그를 출력한다. `--dry-run`은 삭제 없이 selected candidate를 기록한다 | non-current, clean, GitHub PR 상태가 `merged` 또는 `closed`인 worktree만 제거한다. `HEAD` branch worktree와 branch는 삭제하지 않는다 |
| fetch | `f` | selected worktree/repo remote fetch | remote refs와 ahead/behind 갱신 | long operation은 background op로 처리한다 |
| pull | `p` | selected worktree `git pull` | remote 변경 merge. `Space`로 체크한 worktree가 있으면 체크된 대상 전체에 적용 | clean worktree expectation을 문서에 노출한다 |
| push | `P` | selected branch push | remote에 현재 branch push | 실패는 status bar/message로 표시한다 |
| merge upstream | `m` | upstream branch merge | upstream을 현재 branch에 merge | conflict는 status `!`로 드러난다 |
| merge branch | `M` | branch select modal 후 merge | 선택 branch merge | cancel 가능해야 한다 |

Agent worktree mutation은 raw `git worktree add/remove/prune` 대신 `owt worktree create/delete/prune` plain CLI를 기본 경로로 사용한다. fallback은 `owt`가 실행 불가능하고 사용자가 명시적으로 승인한 경우로 제한한다.

# 4. Background Operation 정책

```yaml
background_operation_policy:
  op_kinds:
    - Fetch
    - Pull
    - Push
    - Add
    - Delete
    - Merge
  ui_contract:
    - active operation blocks conflicting input
    - spinner ticks while operation is running
    - result is surfaced as info/error AppMessage
    - list/details refresh after successful state-changing operation
```

# 5. Status 계약

| Status | Symbol | 의미 |
|---|---|---|
| clean | `✓` | 변경 없음 |
| staged | `+` | staged 변경 있음 |
| unstaged | `~` | unstaged 변경 있음 |
| conflict | `!` | merge conflict 있음 |
| mixed | `*` | staged + unstaged 변경 있음 |

```yaml
ahead_behind_display:
  ahead_only: "↑N"
  behind_only: "↓N"
  ahead_and_behind: "↑N↓M"
  no_difference: null
```

```yaml
pr_status_display:
  supported_provider: GitHub
  shown_values:
    - open
    - closed
    - merged
    - draft
  fallback: "-"
  fallback_when:
    - no_pr
    - non_github_remote
    - missing_auth
    - failed_auth
    - failed_network
    - failed_lookup
    - unsupported_provider
    - unknown_or_other_value
  blocking_policy: auxiliary_only
```

PR status는 worktree row의 보조 표시다. GitHub remote에서 확인된 `open`, `closed`, `merged`, `draft`만 사용자에게 노출한다. 그 밖의 모든 경우는 `-`로 표시하며, PR lookup은 core worktree listing, status, ahead/behind 계산보다 우선하지 않는다.

# 6. Test-backed Invariant

- Worktree edge case는 isolated temporary Git repository에서 테스트한다.
- URL에서 repo name을 파싱하는 동작은 npm/docs clone UX와 연결된다.
- nested/worktreeConfig 관련 repair 동작은 bare 오인식 회귀를 막는 중요 invariant다.
- Git command env sanitization은 hook/shell 환경 회귀를 막는 중요 invariant다.
- Git helper command는 shell integration stdout/stderr handoff를 오염시키면 안 된다. Remote URL 확인처럼 값을 조회하는 helper는 child stdout/stderr를 capture해야 한다.
- 새 worktree 생성 직후 list refresh와 selection reconcile은 즉시 `Enter` 했을 때 새 worktree path를 handoff해야 한다.
- Remote base branch fetch는 새 branch 생성 시 최신 `origin/<base>` commit을 기준으로 worktree를 만들 수 있어야 한다.
- CLI prune은 dirty, PR 미완료, current, HEAD-branch, bare, detached worktree를 삭제하지 않고 `--dry-run`이 삭제를 수행하지 않는 regression test로 고정한다.
- PR status lookup은 GitHub-only 보조 조회다. 실패, 누락, non-GitHub remote, unsupported provider는 모두 `-` 표시로 수렴해야 하며 list operation의 성공 여부를 바꾸면 안 된다.

# 7. 검증 규칙

- `src/git.rs` 또는 Git operation을 바꾸면 `cargo test`를 실행한다.
- worktree edge case는 `tests/git_test.rs` 또는 `src/git.rs` tests에 regression test를 추가한다.
- user-visible Git operation semantics가 바뀌면 `docs/usage/git-operations.md`, `docs/usage/worktrees.md`, 이 SSOT를 함께 갱신한다.
- PR status 표시 구현은 GitHub remote의 `open`, `closed`, `merged`, `draft` mapping과 모든 fallback `-` 경로를 검증해야 한다.
- PR lookup failure path는 worktree list refresh를 실패시키거나 지연시키지 않는다는 기대를 검증해야 한다.
