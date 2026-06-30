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
  - .agents/prompts/install-owt.md
  - .agents/skills/owt-install/SKILL.md
  - .agents/skills/owt-worktree/SKILL.md
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
    - agent_plain_cli_and_skill_contract
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
| `owt worktree list` | agent/script가 TUI 없이 worktree 목록 확인 | tab-separated `kind path branch status last_commit ahead behind pr` record를 출력한다 | Git repo가 아니면 오류; `--pr` 실패는 `-` 표시 |
| `owt worktree create <BRANCH>` | agent/script가 TUI 없이 worktree 생성 | regular repo는 configured root 아래, `.bare` layout은 sibling path에 worktree를 생성한다. `--tmux=on`이면 생성 후 worktree pane을 연다 | branch 중복 checkout, git add 실패 시 오류 |
| `owt worktree delete <TARGET>` | agent/script가 TUI 없이 worktree 삭제 | branch/name/path로 단일 worktree를 찾아 제거하고 `--branch`면 local branch도 삭제한다 | bare repo 삭제 거부; dirty worktree는 `--force` 없으면 오류 |
| `owt worktree prune` | agent/script가 stale metadata와 완료된 worktree 정리 | stale metadata를 정리하고, `owt worktree list`가 조회하는 모든 worktree 판단 결과를 tab-separated log로 출력하며, non-current clean worktree 중 branch가 `HEAD`에 merge된 대상만 제거한다. `--dry-run`은 metadata prune을 preview하고 제거 가능한 worktree를 prompt로 확인하되 삭제하지 않는다 | Git repo가 아니면 오류; dirty/unmerged/current/bare/detached worktree와 branch는 삭제하지 않음 |
| `owt pr status` | agent/script가 GitHub merge/PR 상태 확인 | `gh` 기반으로 `open`, `closed`, `merged`, `draft`, `-` 중 하나를 출력한다 | non-GitHub/auth/network/lookup 실패는 `-` |
| `owt commit tree` | agent/script가 commit graph 확인 | 현재 worktree의 recent commit graph를 출력한다 | bare repo path면 오류 |
| `owt search <QUERY>` | agent/script가 worktree 검색 | path/name/branch/status/PR status를 검색하고 list와 같은 record shape을 출력한다 | Git repo가 아니면 오류 |
| `owt --version` | 버전 확인 | package version 출력 | 없음 |
| `owt test-cd` | shell integration debug | `OWT_OUTPUT_FILE` handoff를 TUI 없이 확인 | 일반 사용자 workflow가 아닌 debug command |

Plain CLI command group은 GitHub CLI의 noun-first pattern을 따른다. Top-level command group은 `worktree`, `pr`, `commit`, `search`처럼 단수 명사여야 하며, `owt <group> --help`와 action-level help를 제공해야 한다.

Agent-facing install prompt와 skills는 `.agents/`에 둔다. 이 asset들은 agent에게 TUI를 drive하지 말고 `owt worktree ...` plain CLI를 사용하도록 안내해야 하며, worktree mutation에서 raw `git worktree` fallback을 기본값으로 두면 안 된다.

# 4. TUI State 계약

| State | 진입 | 주요 key | 종료/전이 |
|---|---|---|---|
| `List` | TUI 기본 상태 | navigation, add/delete/git/open/config/help/search, PR metadata 표시. `tmux_worktree_mode`가 켜져 있고 matching pane title이 있으면 `Enter`는 해당 pane을 focus한다 | modal state 또는 quit |
| `AddModal` | `a` | branch type, branch name, `Tab`, `Enter`, `Esc` | `ExitAction::CreateWorktree` queue 후 quit 또는 cancel |
| `ConfirmDelete` | `d` | `y`/`Enter`, `n`/`Esc`, `b` | delete/cancel |
| `ConfigModal` | `c` | `j`/`k`, `Enter`, `s`, `Esc`/`q` | edit/save/close |
| `HelpModal` | `?` | scroll, close | return to list |
| `MergeBranchSelect` | `M` | `j`/`k`, `Enter`, `Esc` | merge/cancel |

`List`는 worktree row 또는 list metadata에 PR column을 둘 수 있다. 이 column은 GitHub remote에서 확인한 PR 상태만 표시하며, 허용 값은 `open`, `closed`, `merged`, `draft`뿐이다. PR이 없거나, remote가 GitHub가 아니거나, auth/network/lookup 실패가 있거나, provider가 지원되지 않거나, 알 수 없는 값 또는 그 밖의 값이면 `-`를 표시한다. PR 조회는 보조 metadata이며 worktree 목록 표시를 실패시키거나 block하면 안 된다.

# 5. Keybinding 계약

| Category | Key | 동작 |
|---|---|---|
| navigation | `j`/`↓`, `k`/`↑` | selection 이동 |
| navigation | `gg`/`Home`, `G`/`End` | top/bottom 이동 |
| navigation | `Ctrl+d`, `Ctrl+u` | half-page 이동 |
| navigation | `g` | launch한 current worktree로 이동 |
| search | `/`, text, `Backspace`, `Esc`, `Enter` | filter 시작/수정/취소/선택 진입 |
| selection | `Space` | batch action 대상 worktree 선택/해제 |
| worktree | `Enter` | 선택 worktree로 cd handoff |
| worktree | `a`, `d` | add/delete modal. 체크된 worktree가 있으면 delete는 체크된 대상 전체에 적용 |
| git | `f`, `p`, `P`, `m`, `M` | fetch/pull/push/merge upstream/merge branch. 체크된 worktree가 있으면 pull은 체크된 대상 전체에 적용 |
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
    success: "TUI exits first; then new worktree creation, file copy, post-add script, tmux pane behavior, and shell handoff run according to config"
  - id: UC_SWITCH_WORKTREE
    actor: regular_repo_user
    trigger: "selects worktree and presses `Enter`"
    success: "matching tmux pane is focused when configured and present; otherwise TUI exits and shell integration changes directory when installed"
  - id: UC_DELETE_WORKTREE
    actor: reviewer
    trigger: "selects one or more worktrees with `Space`, presses `d`, confirms"
    success: "target worktree(s) are removed; optional branch delete follows confirmation state"
  - id: UC_HOTFIX
    actor: hotfix_operator
    trigger: "creates hotfix worktree while feature work remains untouched"
    success: "hotfix branch can be pushed/merged independently"
  - id: UC_PR_REVIEW
    actor: reviewer
    trigger: "fetches remote and creates review worktree"
    success: "review worktree can be tested and deleted; GitHub PR state, when available, is visible as open/closed/merged/draft metadata"
  - id: UC_AGENT_PLAIN_CLI
    actor: agent_or_contributor
    trigger: "runs noun-first plain CLI commands instead of opening the TUI"
    success: "install prompt and skills direct agents to use parseable owt stdout for worktree create/delete/list, PR status, commit tree, and search"
```

사용자가 수락한 구현 범위는 GitHub-only PR 상태 표시까지다. `UC_PR_REVIEW`는 GitHub PR 상태를 빠르게 확인하는 보조 경험을 포함하지만, non-GitHub provider 지원이나 repository layout 변경을 포함하지 않는다.

# 7. 검증 규칙

- keybinding이 바뀌면 `docs/reference/keybindings.md`, README keybinding table, help modal, 이 SSOT를 함께 갱신한다.
- TUI state가 추가되면 `src/types.rs::AppState`, rendering, input handler, docs를 함께 확인한다.
- user-facing flow가 바뀌면 `docs/usage/`와 이 SSOT를 함께 갱신한다.
- CLI parsing은 default TUI path, `--path`/`-p`, positional path, `clone`, `init`, `setup`, `test-cd`, help/version command를 test로 고정한다.
- Plain CLI parsing은 `worktree`, `pr`, `commit`, `search` group과 group/action `--help`를 test로 고정한다.
- Plain CLI stdout은 TUI escape, decorative table, color 없이 tab-separated record로 유지한다. `worktree prune` 제거 출력은 `pruned<TAB>worktree<TAB>branch<TAB>path` shape을 유지하고, 판단 로그는 `pruned<TAB>log<TAB>action<TAB>branch<TAB>path<TAB>reason` shape으로 출력한다.
- Agent install prompt와 skills를 변경하면 `.agents/`, README, 이 SSOT의 plain CLI 계약을 함께 확인한다.
- `tmux_worktree_mode`는 worktree 생성 후 pane open과 `Enter` pane focus를 app-level/fake tmux test로 고정한다.
- `owt clone <URL> [PATH]`는 `.bare` repository와 default branch 첫 worktree를 만드는 integration test로 고정한다.
- AddModal `Enter`는 background add operation을 시작하지 않고 `ExitAction::CreateWorktree`를 queue한 뒤 TUI를 종료하는 app-level test로 고정한다.
- `Enter` key는 정상 선택, filter 선택, background operation 중 block, bare repository 선택 거부를 app-level test로 고정한다.
- Dirty worktree delete guard는 force가 없을 때 delete operation을 시작하지 않는 test로 고정한다.
- Worktree status symbol/label과 ahead/behind display는 `types` unit test로 고정한다.
- PR column/list metadata는 GitHub remote에서 `open`, `closed`, `merged`, `draft`만 표시하고, PR 없음, non-GitHub remote, auth/network/lookup 실패, unsupported provider, unknown/other 값은 `-`로 표시하는 검증으로 고정한다.
- PR 조회 실패는 core worktree listing을 실패시키거나 block하지 않는 검증으로 고정한다.
