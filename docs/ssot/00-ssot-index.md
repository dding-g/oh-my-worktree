---
title: owt SSOT 문서 지도
description: owt의 제품/동작/정책 계약을 문서별로 분리한 SSOT 인덱스
ref:
  - AGENTS.md
  - docs/solutions/README.md
  - docs/ssot/01-repository-worktree-policy.md
  - docs/ssot/02-configuration-trust-boundary-policy.md
  - docs/ssot/03-cli-tui-use-case-contract.md
  - docs/ssot/04-git-operation-safety-policy.md
  - docs/ssot/05-shell-integration-exit-contract.md
  - docs/ssot/06-documentation-release-asset-policy.md
---

# 1. 문서 목적

이 문서는 `owt`의 SSOT 문서 집합을 정의한다. SSOT는 사용자에게 노출되는 제품 동작, 정책, 신뢰 경계, 문서 동기화 계약의 정본이다.

```yaml
ssot_contract:
  language: Korean
  owns:
    - product_behavior_contract
    - user_case_contract
    - policy_boundary_contract
    - documentation_sync_contract
  does_not_own:
    - implementation_tutorial
    - session_transcript
    - release_changelog
    - low_level_refactor_notes
  companion_layers:
    agents_operational_guide: AGENTS.md
    reusable_context: docs/solutions/
    user_facing_docs: docs/
```

# 2. SSOT 문서 카탈로그

| 문서 | 정본 범위 | 대표 mirror 문서 |
|---|---|---|
| `00-ssot-index.md` | SSOT 문서 지도, ownership, update rule | `AGENTS.md`, `docs/solutions/README.md` |
| `01-repository-worktree-policy.md` | regular repository, bare repository, `.bare` layout, worktree 생성 위치 | `README.md`, `README.ko.md`, `docs/getting-started/quick-start.md`, `docs/usage/worktrees.md` |
| `02-configuration-trust-boundary-policy.md` | config precedence, safe override, post-add script trust boundary | `docs/reference/configuration.md`, `src/config.rs` |
| `03-cli-tui-use-case-contract.md` | CLI command와 TUI user case/state/keybinding 계약 | `README.md`, `docs/reference/keybindings.md`, `docs/usage/*.md` |
| `04-git-operation-safety-policy.md` | Git operation, clean-state guard, env sanitization, background op policy | `docs/usage/git-operations.md`, `src/git.rs`, `src/app.rs` |
| `05-shell-integration-exit-contract.md` | `OWT_OUTPUT_FILE`, `/dev/tty`, `owt setup`, `Enter` cd handoff | `docs/getting-started/shell-integration.md`, `src/main.rs` |
| `06-documentation-release-asset-policy.md` | README/docs/npm/docs asset/release doc synchronization | `docs/index.md`, `docs/index.html`, `npm/README.md`, `assets/` |

# 3. Update 규칙

```yaml
update_rules:
  behavior_change:
    must_update:
      - relevant_ssot
      - user_facing_docs_if_visible
      - regression_tests_if_code_behavior_changed
  docs_positioning_change:
    must_update:
      - README.md
      - README.ko.md
      - npm/README.md_if_package_summary_changes
      - docs/index.md_if_homepage_summary_changes
      - relevant_ssot
  durable_learning:
    route_to:
      - docs/solutions/
      - AGENTS.md_if_operational_rule
  verification:
    docs_only:
      - git diff --check
      - inspect_target_diff
    rust_behavior:
      - cargo test
```

# 4. SSOT와 Compound 문서의 경계

| 구분 | SSOT | `docs/solutions/` |
|---|---|---|
| 목적 | 현재 제품/정책 계약 | 재사용 가능한 맥락과 학습 |
| 문체 | 규칙, 표, YAML, 고정 계약 | 설명, 사례, 왜 중요한지 |
| 변경 기준 | 제품 동작이나 정책이 바뀔 때 | durable learning이 생길 때 |
| 예시 | `worktree_root` 적용 범위 | Git env sanitization을 왜 지켜야 하는지 |

# 5. Agent 적용 원칙

- 구현 전에는 `AGENTS.md`를 먼저 읽고, 관련 SSOT를 확인한다.
- SSOT가 user-facing docs와 충돌하면 SSOT를 기준으로 판단하되, 구현과 테스트가 더 최신이면 SSOT를 갱신한다.
- SSOT에는 session log를 넣지 않는다. 시행착오와 배경 설명은 `docs/solutions/`에 둔다.
