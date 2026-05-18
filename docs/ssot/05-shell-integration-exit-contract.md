---
title: owt Shell Integration / Exit 계약
description: Enter key cd handoff, `OWT_OUTPUT_FILE`, `/dev/tty`, `owt setup` 정책
ref:
  - src/main.rs
  - src/app.rs
  - docs/getting-started/shell-integration.md
  - docs/usage/navigation.md
---

# 1. 문서 목적

이 문서는 `owt`가 TUI 종료 후 사용자의 shell directory를 바꾸는 방식의 정본 계약이다.

```yaml
document_contract:
  source_of_truth_for:
    - enter_key_exit_action
    - OWT_OUTPUT_FILE_contract
    - tty_contract
    - setup_command_behavior
  not_source_of_truth_for:
    - shell-specific syntax styling
    - terminal emulator behavior
```

# 2. Enter Key Directory Change 계약

| 조건 | 동작 |
|---|---|
| shell integration 있음 | `Enter`로 선택 worktree path를 `OWT_OUTPUT_FILE`에 기록하고 TUI 종료 후 shell function이 `cd`한다. |
| shell integration 없음 | TUI는 path를 stdout에 출력하고 setup 안내를 stderr에 표시한다. |
| background operation 진행 중 | `Enter`는 directory change를 수행하지 않고 operation 진행 메시지를 표시한다. |
| 선택 대상이 bare repo | worktree 진입 대상으로 취급하지 않는다. |

# 3. `OWT_OUTPUT_FILE` Trust Boundary

```yaml
output_file_policy:
  env_var: OWT_OUTPUT_FILE
  created_by: shell_function
  consumed_by: owt_binary
  must_be_existing_regular_file: true
  symlink_allowed: false
  unix_group_world_access_allowed: false
  content: selected_worktree_absolute_path
```

# 4. TTY 정책

TUI는 shell integration과 함께 동작해야 하므로 stdin/stdout redirection에 의존하지 않고 `/dev/tty`를 사용한다.

```yaml
tty_policy:
  tui_io: /dev/tty
  reason: "shell function이 stdout을 path handoff에 사용할 수 있으므로 TUI drawing은 real terminal에 붙어야 한다."
```

# 5. `owt setup` 정책

| Shell | 대상 config | 동작 |
|---|---|---|
| zsh | `~/.zshrc` | function snippet 추가 안내/확인 |
| bash | `~/.bashrc` | function snippet 추가 안내/확인 |
| fish | `~/.config/fish/functions/owt.fish` 또는 안내 문서 | fish function 안내 |
| unknown | manual snippet | 자동 감지 실패 시 수동 안내 |

Symlink-managed config는 자동 수정하지 않고 수동 추가 안내를 우선한다.

# 6. Debug Command

`owt test-cd`는 TUI 없이 `OWT_OUTPUT_FILE` handoff를 확인하는 debug command다. 일반 사용 흐름의 필수 단계가 아니며, docs에서는 troubleshooting context로만 다룬다.

# 7. 검증 규칙

- shell integration 변경은 `src/main.rs`, `docs/getting-started/shell-integration.md`, 이 SSOT를 함께 갱신한다.
- output file security check를 완화하지 않는다.
- Enter key behavior를 변경하면 `docs/usage/navigation.md`, keybinding docs, TUI use case SSOT를 함께 확인한다.
