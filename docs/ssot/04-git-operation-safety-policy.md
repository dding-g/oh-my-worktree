---
title: owt Git Operation / Safety 정책
description: Git command 실행, worktree operation, background operation safety 계약
ref:
  - src/git.rs
  - src/app.rs
  - src/types.rs
  - tests/git_test.rs
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
| list | TUI load/refresh | `git worktree list --porcelain` | bare entry와 non-bare worktree를 구분한다 | bare entry는 status/ahead/behind 계산 대상이 아니다 |
| add | `a` modal confirm | `git worktree add` | branch/base 정책에 맞는 worktree 생성 | 생성 후 usable worktree인지 확인/repair한다 |
| delete | `d` confirm | `git worktree remove` + optional branch delete | 선택 worktree 제거 | dirty worktree는 기본적으로 삭제하지 않는다 |
| fetch | `f` | selected worktree/repo remote fetch | remote refs와 ahead/behind 갱신 | long operation은 background op로 처리한다 |
| pull | `p` | selected worktree `git pull` | remote 변경 merge | clean worktree expectation을 문서에 노출한다 |
| push | `P` | selected branch push | remote에 현재 branch push | 실패는 status bar/message로 표시한다 |
| merge upstream | `m` | upstream branch merge | upstream을 현재 branch에 merge | conflict는 status `!`로 드러난다 |
| merge branch | `M` | branch select modal 후 merge | 선택 branch merge | cancel 가능해야 한다 |

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

# 6. Test-backed Invariant

- Worktree edge case는 isolated temporary Git repository에서 테스트한다.
- URL에서 repo name을 파싱하는 동작은 npm/docs clone UX와 연결된다.
- nested/worktreeConfig 관련 repair 동작은 bare 오인식 회귀를 막는 중요 invariant다.
- Git command env sanitization은 hook/shell 환경 회귀를 막는 중요 invariant다.

# 7. 검증 규칙

- `src/git.rs` 또는 Git operation을 바꾸면 `cargo test`를 실행한다.
- worktree edge case는 `tests/git_test.rs` 또는 `src/git.rs` tests에 regression test를 추가한다.
- user-visible Git operation semantics가 바뀌면 `docs/usage/git-operations.md`, `docs/usage/worktrees.md`, 이 SSOT를 함께 갱신한다.
