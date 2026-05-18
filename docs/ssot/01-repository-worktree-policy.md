---
title: owt Repository Worktree Policy
description: regular repository와 bare `.bare` layout에서 `owt`가 보장하는 worktree 동작 계약
ref:
  - docs/ssot/00-ssot-index.md
  - README.md
  - README.ko.md
  - npm/README.md
  - docs/index.md
  - docs/getting-started/quick-start.md
  - docs/usage/worktrees.md
  - docs/concepts/bare-repository.md
  - docs/concepts/worktree-pattern.md
  - docs/reference/configuration.md
---

# 1. 문서 목적

이 문서는 `owt`가 Git repository layout별로 worktree를 탐지하고 생성하는 정책의 정본이다.

```yaml
document_contract:
  language: Korean
  primary_reader:
    - contributor
    - documentation_agent
    - implementation_agent
  source_of_truth_for:
    - repository_layout_terms
    - worktree_creation_location_policy
    - docs_positioning_policy
  not_source_of_truth_for:
    - Rust module internals
    - release process
    - generated site styling
  related_ssot:
    config_trust: docs/ssot/02-configuration-trust-boundary-policy.md
    cli_tui_use_cases: docs/ssot/03-cli-tui-use-case-contract.md
    git_operations: docs/ssot/04-git-operation-safety-policy.md
    shell_integration: docs/ssot/05-shell-integration-exit-contract.md
    docs_release_assets: docs/ssot/06-documentation-release-asset-policy.md
```

# 2. 범위

## 2.1 포함 범위

| 항목 | 정책 |
|---|---|
| 제품 포지셔닝 | `owt`는 regular repository와 bare repository layout 모두에서 Git worktree를 관리하는 TUI다. |
| regular repository | 기존 non-bare Git repository에서 `owt`를 바로 실행할 수 있다. |
| `.bare` layout | `owt clone`이 만드는 project-local sibling worktree layout이다. |
| worktree 생성 위치 | layout별 기본 생성 위치와 `worktree_root` 설정의 적용 범위를 정의한다. |
| 문서 동기화 | README, Korean README, usage docs, agent guidance가 같은 정책을 말해야 한다. |

## 2.2 비포함 범위

| 항목 | 제외 사유 |
|---|---|
| Git command implementation detail | `src/git.rs`와 테스트가 정본이다. |
| TUI rendering detail | `src/ui/`와 screenshot이 정본이다. |
| shell integration 설치 절차 | `owt setup` 문서와 구현이 정본이다. |
| release checklist | `AGENTS.md`와 release tooling이 정본이다. |

# 3. 용어와 Layout 정본

```yaml
repository_layout_terms:
  - term: regular_repository
    user_facing_name: "regular Git repository"
    definition: "작업 파일과 `.git/` directory를 함께 가진 일반 non-bare Git repository"
    can_run_owt_directly: true
    conversion_required: false
  - term: bare_repository
    user_facing_name: "bare repository"
    definition: "working directory 없이 Git data만 가진 repository"
    can_run_owt_directly: true
    conversion_required: false
  - term: dot_bare_layout
    user_facing_name: "`.bare` layout"
    definition: "project directory 아래 `.bare/` repository와 sibling worktree들을 두는 layout"
    created_by: "owt clone"
    recommended_for: "project-local sibling worktree organization"
```

# 4. Worktree 생성 위치 정책

| 현재 layout | 새 worktree 기본 위치 | 설정 override | 사용자 메시지 원칙 |
|---|---|---|---|
| regular repository | `~/.owt/worktree/<repo-name>/` | `worktree_root` | 변환 없이 바로 사용할 수 있다고 설명한다. |
| `.bare` layout | 기존 worktree들과 같은 parent directory | 해당 없음 | project-local sibling layout이라고 설명한다. |
| traditional bare repository | repository 인접 worktree 위치 | 구현 동작 기준 | `.bare` 외 bare layout도 지원한다고만 설명한다. |

```yaml
worktree_root_policy:
  config_key: worktree_root
  default_value: "~/.owt/worktree"
  applies_to:
    - regular_repository
  docs_must_say:
    - "regular repository에서 새 worktree는 기본적으로 `~/.owt/worktree/<repo-name>/` 아래에 생성된다."
    - "다른 위치를 원하면 `worktree_root`를 설정한다."
  docs_must_not_say:
    - "bare repository로 변환해야 owt를 사용할 수 있다."
    - "`.bare` layout만 owt의 정식 사용 방식이다."
```

# 5. Command 의미 정책

| Command | 정본 의미 | 금지되는 설명 |
|---|---|---|
| `owt` | 현재 repository 또는 지정 path에서 TUI를 실행한다. | bare repository 전용 실행 명령이라고 쓰지 않는다. |
| `owt clone <URL> [PATH]` | `.bare` layout으로 clone하고 첫 worktree를 생성한다. | 모든 사용자가 반드시 거쳐야 하는 시작 단계라고 쓰지 않는다. |
| `owt init` | 기존 repository를 `.bare` layout으로 바꾸는 가이드를 보여준다. | regular repository 사용 전 필수 변환이라고 쓰지 않는다. |
| `owt setup` | shell integration을 설치한다. | repository layout 정책과 섞어 설명하지 않는다. |

# 6. 문서 포지셔닝 규칙

## 6.1 README 규칙

```yaml
readme_positioning:
  first_sentence: "regular repository와 bare `.bare` layout 모두를 포함해야 한다."
  getting_started_order:
    - existing_regular_repository
    - new_project_with_dot_bare
    - optional_convert_to_dot_bare
  required_claims:
    - "regular repository는 변환 없이 바로 사용할 수 있다."
    - "`.bare` layout은 project-local sibling worktree를 선호할 때 선택한다."
    - "regular repository의 기본 worktree root는 `~/.owt/worktree/<repo-name>/`이다."
```

## 6.2 Korean README 규칙

- `README.ko.md`는 `README.md`의 product contract를 같은 순서로 반영한다.
- 본문은 한국어로 작성하되 `regular repository`, `bare repository`, `.bare`, `worktree_root`, command, path는 code 또는 English identifier를 유지한다.
- 새 정책이 생기면 English README와 Korean README를 함께 수정한다.

## 6.3 Contributor 문서 규칙

| 문서 | 반드시 유지할 내용 |
|---|---|
| `AGENTS.md` | regular repository와 bare layout 모두 first-class support임을 명시한다. |
| `docs/index.md` | quick start에서 regular repository 직접 실행을 먼저 보여주고 `.bare` clone을 선택지로 보여준다. |
| `docs/getting-started/quick-start.md` | existing regular repository, new `.bare` project, optional conversion 순서를 유지한다. |
| `docs/usage/worktrees.md` | add worktree flow와 layout별 생성 위치를 명시한다. |
| `docs/concepts/bare-repository.md` | bare repository를 project-local sibling workflow에 유용한 선택지로 설명한다. |
| `docs/concepts/worktree-pattern.md` | `.bare`는 supported/recommended layout이지 유일한 layout이 아님을 명시한다. |
| `docs/reference/configuration.md` | `worktree_root`가 regular repository worktree root를 바꾸는 설정임을 명시한다. |
| `npm/README.md` | package registry용 축약 문서에서도 regular repository와 `.bare` layout을 둘 다 지원한다고 설명한다. |

# 7. 변경 검증 규칙

```yaml
verification_policy:
  docs_only_change:
    required:
      - "git diff --check"
      - "target document diff inspection"
    optional:
      - "local docs build when dependencies are installed"
  code_or_behavior_change:
    required:
      - "cargo test"
      - "regression tests for worktree edge cases"
  forbidden:
    - "문서 변경을 이유로 Rust source 변경을 함께 섞는다."
    - "regular repository 지원을 `.bare`의 부가 기능처럼 축소해서 설명한다."
```
