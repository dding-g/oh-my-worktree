---
title: owt Configuration / Trust Boundary 정책
description: config precedence, safe override, post-add script 실행 신뢰 경계를 정의한다
ref:
  - src/config.rs
  - src/app.rs
  - docs/reference/configuration.md
  - docs/solutions/best-practices/ai-agent-project-map.md
---

# 1. 문서 목적

이 문서는 `owt` 설정 파일과 환경 변수, project-level config, post-add script 실행 정책의 정본이다.

```yaml
document_contract:
  source_of_truth_for:
    - config_file_locations
    - config_precedence
    - project_config_safe_override_boundary
    - post_add_script_trust_boundary
  not_source_of_truth_for:
    - TOML parser internals
    - UI layout for config modal
```

# 2. 설정 Source와 Precedence

| Source | Path / 변수 | 적용 범위 | 우선순위 |
|---|---|---|---|
| global config | `~/.config/owt/config.toml` | 사용자 전체 기본 설정 | 1 |
| project config | `<project-root>/.owt/config.toml` | 현재 project/worktree group | 2 |
| environment | `EDITOR`, `TERMINAL` | command 실행 환경 | config 값이 없을 때 fallback |
| built-in default | 코드 default | config/env가 없을 때 | 마지막 fallback |

```yaml
config_precedence:
  load_order:
    - global_config
    - project_config_safe_overrides
  env_fallbacks:
    editor: EDITOR
    terminal: TERMINAL
  default_values:
    editor: vim
    worktree_root: ~/.owt/worktree
```

# 3. Config Key 계약

| Key | Type | 의미 | Project config override | Trust level |
|---|---|---|---|---|
| `editor` | string | `o` key로 worktree를 열 editor | yes | safe |
| `terminal` | string | `t` key로 worktree를 열 terminal | yes | safe |
| `worktree_root` | string | regular repository에서 새 worktree를 만들 root | yes | safe |
| `copy_files` | array[string] | 새 worktree 생성 후 복사할 파일 목록 | yes | safe with filesystem effects |
| `post_add_script` | string | 새 worktree 생성 후 실행할 script path | yes | inert unless globally enabled |
| `run_post_add_script_in_tmux` | bool | post-add script 자동 실행 여부 | no for enabling from project config | trusted global only |

# 4. Trust Boundary

Project config는 repository가 소유할 수 있으므로 자동 script 실행 권한을 부여하면 안 된다.

```yaml
post_add_script_policy:
  script_path_key: post_add_script
  auto_run_key: run_post_add_script_in_tmux
  execution_mode: detached_tmux_only
  direct_shell_fallback: false
  global_config_can_enable_auto_run: true
  project_config_can_enable_auto_run: false
  project_config_can_define_script_path: true
  when_disabled: "script is not run"
```

# 5. Project Config 저장 정책

| 저장 대상 | 포함 가능 | 포함 금지 |
|---|---|---|
| global config | 모든 key | 없음 |
| project config | `editor`, `terminal`, `worktree_root`, `copy_files`, `post_add_script` | `run_post_add_script_in_tmux = true` |

# 6. 검증 규칙

- `src/config.rs`를 변경하면 config parsing, save, merge test를 갱신한다.
- post-add script 실행 경계를 바꾸면 `docs/reference/configuration.md`, `README.md`, `README.ko.md`, 이 SSOT를 함께 갱신한다.
- trust boundary 변경은 단순 UX 변경이 아니라 security-sensitive behavior로 취급한다.
