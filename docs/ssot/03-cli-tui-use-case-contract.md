---
title: owt CLI / TUI Use Case 계약
description: CLI command, TUI state, keybinding, user case의 정본 계약
ref:
  - src/main.rs
  - src/app.rs
  - src/types.rs
  - src/ui/help_modal.rs
  - src/ui/main_view.rs
  - docs/reference/keybindings.md
  - docs/usage/navigation.md
  - docs/usage/worktrees.md
  - docs/usage/workflows.md
---

# 1. 문서 목적

이 문서는 `owt`의 사용자 actor, CLI command, TUI state/use case, keybinding 계약을 정의한다.

```yaml
document_contract:
  source_of_truth_for:
    - cli_command_meanings
    - tui_state_inventory
    - user_case_inventory
    - keybinding_contract
  not_source_of_truth_for:
    - ratatui_layout_coordinates
    - color_theme_values
```

# 2. Actor 정의

| Actor | 설명 | 주요 목표 |
|---|---|---|
| `regular_repo_user` | 기존 non-bare Git repository에서 `owt`를 실행하는 사용자 | 변환 없이 worktree 생성/전환 |
| `dot_bare_user` | `owt clone` 또는 수동 변환으로 `.bare` layout을 쓰는 사용자 | project-local sibling worktree 관리 |
| `reviewer` | PR/branch를 로컬에서 확인하는 사용자 | 임시 worktree 생성 후 삭제 |
| `hotfix_operator` | 진행 중인 작업을 유지한 채 긴급 수정하는 사용자 | hotfix worktree 생성, push, merge |
| `agent_or_contributor` | repo를 수정하는 사람/AI agent | SSOT와 docs를 기준으로 동작 보존 |

# 3. CLI Command 계약

| Command | User case | 정본 동작 | 실패/제약 |
|---|---|---|---|
| `owt [PATH]` | TUI 실행 | 현재 path 또는 지정 path에서 repo layout 탐지 후 TUI 실행 | Git repo가 아니면 오류 |
| `owt clone <URL> [PATH]` | 새 project-local `.bare` layout 시작 | bare clone을 만들고 default branch의 첫 worktree를 생성 | clone/add 실패 시 오류 |
| `owt init` | 기존 repo를 `.bare`로 바꾸고 싶은 사용자에게 guide 제공 | 변환 명령을 출력한다; 자동 변환하지 않는다 | Git repo가 아니면 오류 |
| `owt setup` | shell integration 설치 | shell별 function snippet을 안내/추가한다 | symlink-managed shell config는 수동 안내 |
| `owt --version` | 버전 확인 | package version 출력 | 없음 |
| `owt test-cd` | shell integration debug | `OWT_OUTPUT_FILE` handoff를 TUI 없이 확인 | 일반 사용자 workflow가 아닌 debug command |

# 4. TUI State 계약

| State | 진입 | 주요 key | 종료/전이 |
|---|---|---|---|
| `List` | TUI 기본 상태 | navigation, add/delete/git/open/config/help/search | modal state 또는 quit |
| `AddModal` | `a` | branch type, branch name, `Tab`, `Enter`, `Esc` | create worktree 또는 cancel |
| `ConfirmDelete` | `d` | `y`/`Enter`, `n`/`Esc`, `b` | delete/cancel |
| `ConfigModal` | `c` | `j`/`k`, `Enter`, `s`, `Esc`/`q` | edit/save/close |
| `HelpModal` | `?` | scroll, close | return to list |
| `MergeBranchSelect` | `M` | `j`/`k`, `Enter`, `Esc` | merge/cancel |

# 5. Keybinding 계약

| Category | Key | 동작 |
|---|---|---|
| navigation | `j`/`↓`, `k`/`↑` | selection 이동 |
| navigation | `gg`/`Home`, `G`/`End` | top/bottom 이동 |
| navigation | `Ctrl+d`, `Ctrl+u` | half-page 이동 |
| navigation | `g` | launch한 current worktree로 이동 |
| search | `/`, text, `Backspace`, `Esc`, `Enter` | filter 시작/수정/취소/선택 진입 |
| worktree | `Enter` | 선택 worktree로 cd handoff |
| worktree | `a`, `d` | add/delete modal |
| git | `f`, `p`, `P`, `m`, `M` | fetch/pull/push/merge upstream/merge branch |
| external | `o`, `t`, `y` | editor/terminal 열기, path copy |
| config/help | `c`, `?` | config modal/help modal |
| lifecycle | `q`, `Ctrl+c` | quit |

# 6. User Case Inventory

```yaml
user_cases:
  - id: UC_REGULAR_START
    actor: regular_repo_user
    trigger: "runs `owt` inside an existing regular Git repository"
    success: "TUI lists current/native worktrees and can create new worktrees under configured root"
  - id: UC_DOT_BARE_CLONE
    actor: dot_bare_user
    trigger: "runs `owt clone <URL>`"
    success: "project/.bare and first worktree are created"
  - id: UC_ADD_WORKTREE
    actor: regular_repo_user
    trigger: "presses `a`, enters branch, presses `Enter`"
    success: "new worktree is created, files/scripts run according to config, list refreshes"
  - id: UC_SWITCH_WORKTREE
    actor: regular_repo_user
    trigger: "selects worktree and presses `Enter`"
    success: "TUI exits and shell integration changes directory when installed"
  - id: UC_DELETE_WORKTREE
    actor: reviewer
    trigger: "selects worktree, presses `d`, confirms"
    success: "worktree is removed; optional branch delete follows confirmation state"
  - id: UC_HOTFIX
    actor: hotfix_operator
    trigger: "creates hotfix worktree while feature work remains untouched"
    success: "hotfix branch can be pushed/merged independently"
  - id: UC_PR_REVIEW
    actor: reviewer
    trigger: "fetches remote and creates review worktree"
    success: "review worktree can be tested and deleted"
```

# 7. 검증 규칙

- keybinding이 바뀌면 `docs/reference/keybindings.md`, README keybinding table, help modal, 이 SSOT를 함께 갱신한다.
- TUI state가 추가되면 `src/types.rs::AppState`, rendering, input handler, docs를 함께 확인한다.
- user-facing flow가 바뀌면 `docs/usage/`와 이 SSOT를 함께 갱신한다.
