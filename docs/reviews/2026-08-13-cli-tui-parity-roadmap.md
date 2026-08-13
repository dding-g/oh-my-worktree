---
title: CLI–TUI Capability Parity Outcome Roadmap
date: 2026-08-13
planning_style: outcome-based
time_horizon: 2-3 flexible release horizons
depends_on:
  - docs/reviews/2026-08-13-cli-tui-parity-evaluation.md
  - docs/reviews/2026-08-10-product-opportunity-review.md
  - docs/solutions/performance/worktree-loading-performance-plan.md
---

# CLI–TUI Capability Parity Outcome Roadmap

## 1. 전략적 맥락

`owt`의 CLI는 automation-friendly interface이고 TUI는 같은 capability를 발견하고 실행하기 쉽게 만드는 interactive adapter다. 두 surface가 별도 기능 집합으로 진화하면 사용자는 이름이 같은 action에서 다른 결과를 얻고, maintainer는 safety fix를 두 번 구현하게 된다.

따라서 roadmap의 중심은 기능 개수가 아니라 다음 결과다.

> **CLI에서 가능한 모든 user-facing 업무를 TUI에서도 동일한 core behavior와 safety invariant로 완료할 수 있게 한다.**

Parity는 flag나 출력 형식을 픽셀 단위로 복제하는 뜻이 아니다. request semantics, validation, side effect, safety, result가 같고 presentation만 surface에 맞게 달라야 한다.

## 2. North-star와 guardrail

### North-star

- **Semantic capability parity: 100%**
- 측정식: TUI equivalent가 있는 GA CLI capability / 전체 GA CLI semantic capability

### Guardrail

- destructive false positive: 0
- shared use-case를 우회하는 CLI/TUI mutation path: 0
- 기존 CLI default output breaking change: 0
- repo 안에서 기본 TUI first-frame regression: 기준선 대비 10% 이내
- 신규 GA capability의 parity declaration 누락: 0

## 3. Parity 정책

1. CLI command parser와 TUI event handler는 orchestration을 소유하지 않는다.
2. 두 surface는 동일한 typed `Request -> Progress/Event -> Result` use-case를 호출한다.
3. validation과 destructive live check는 core에 한 번만 존재한다.
4. 신규 GA CLI capability는 같은 release에서 TUI entry point와 parity scenario를 제공한다.
5. 예외는 transport-only option 또는 internal debug capability뿐이며 registry에 이유와 owner를 기록한다.
6. TUI가 아직 없는 capability는 experimental로 표시하고 GA로 문서화하지 않는다.

## 4. Horizon 0 — Parity debt를 보이게 만들기

### Outcome statement

> Enable maintainers to see and block CLI–TUI capability drift so that every future release preserves one coherent product.

### Deliverable candidates

- `CapabilityId`, surface mapping, exemption reason을 가진 capability registry
- 현재 command → request → core → CLI renderer/TUI state mapping 문서
- CI parity contract test
- SSOT에 “CLI capability SSOT, TUI semantic parity” 정책 추가

### Success metrics

- 현재 user-facing semantic capability가 빠짐없이 Full/Partial/Missing으로 등록됨
- 신규 GA command가 TUI mapping 또는 승인된 exemption 없이 test를 통과하지 못함
- mutation path inventory 100% 완료

### Dependencies

- `ssot:update` 절차로 `docs/ssot/03-cli-tui-use-case-contract.md` 변경 합의
- `test-cd`, serialization처럼 명시적으로 제외할 capability 확정

## 5. Horizon 1 — 동일 이름, 동일 결과

### Outcome A: 안전한 cleanup parity

**Status: 첫 vertical slice 구현 완료 (2026-08-13 working tree).** `run_prune` shared core, TUI preview/confirm, CLI interactive dry-run, live recheck contract test를 포함한다.

> Enable reviewers to preview and execute the same completed-PR cleanup from CLI or TUI so that no worktree is removed under different rules depending on surface.

Deliverable candidates:

- `PruneRequest { repo, launch_path, dry_run }`
- typed candidate/exclusion/progress/result model
- CLI `worktree prune`과 TUI `CleanupPreview`가 같은 use-case 호출
- TUI preview에서 removable/excluded reason, live-check state, confirmation 표시

Success metrics:

- CLI/TUI 동일 fixture에서 candidate와 exclusion reason 100% 일치
- unknown/dirty/current/HEAD/bare/detached protection parity test 통과
- TUI `x`가 metadata prune만 수행하는 현재 gap 제거

### Outcome B: 검색 결과 parity

> Enable users to find a worktree by any field available in CLI search so that interaction surface를 바꿔도 찾을 수 있는 대상이 같아진다.

Deliverable candidates:

- shared `WorktreeQuery`와 `worktree_matches`
- TUI filter에 path/name/branch/status/PR 지원
- match-only view와 dim view UX 선택; active field/match highlight

Success metrics:

- shared search fixture에서 CLI/TUI match set 100% 동일
- PR/status/path query TUI scenario test 통과

### Outcome C: create option parity without default-flow regression

> Enable advanced users to choose base, target path, and one-run tmux behavior in TUI while keeping the common create flow fast.

**Current status (2026-08-13):** option parity is implemented in the working tree: `Tab` cycles base, `Ctrl+p` edits explicit path, and `Ctrl+t` cycles per-create tmux `default/on/off`. The remaining follow-up is shared orchestration extraction and a rendered preview, not a missing CLI option.

Deliverable candidates:

- shared `CreateWorktreeRequest`
- add modal의 progressive disclosure advanced section
- custom target path, base, tmux `default/on/off` 선택
- create preview: resolved path, base ref, copy/post-add/tmux plan

Success metrics:

- CLI/TUI request contract fixture 100% 동일
- 기본 create keystroke 수 증가 1회 이하
- collision/fetch/copy/script result behavior parity test 통과

## 6. Horizon 2 — 정보 탐색 parity와 shared core 완성

### Outcome statement

> Enable users to inspect PR and commit context completely inside TUI so that every read-only CLI investigation has an interactive equivalent.

Deliverable candidates:

- full-screen `CommitTree` state with paging and adjustable limit
- `PrStatus` detail modal: selected/all/arbitrary branch query
- shared read use-cases for list, PR status, commit graph, search
- CLI renderer와 TUI view model을 core result에서 분리

Success metrics:

- `pr status --branch/--all` semantic scenarios가 TUI에서 모두 완료됨
- commit graph limit/paging으로 CLI `-n` 결과 범위를 탐색 가능
- read capability parity 100%

### Performance lane: Instant First Frame

이 horizon과 병행하되 parity core와 결합한다.

- cheap list result로 첫 frame 렌더
- visible-first bounded metadata loading
- result를 index가 아닌 path로 적용
- unknown status는 clean처럼 보이지 않음

Success metrics:

- 30-worktree warm repo first-frame p95 목표 200ms 이하; baseline 측정 후 확정
- startup critical path의 Git subprocess 1개
- delete/prune은 cache를 사용하지 않고 live status 재조회

## 7. Horizon 3 — Global CLI capability parity

### Outcome statement

> Enable users to start, configure, and understand `owt` through TUI even before a repository workspace is available.

Deliverable candidates:

- repo 안에서는 즉시 list로 진입하고, repo 밖에서는 lightweight global home 표시
- clone workspace flow
- init conversion guide modal
- shell setup preview/execute/result flow
- About/version와 command help browser
- global command palette 또는 action launcher

Success metrics:

- clone/init/setup/help/version user-facing capability parity 100%
- 기존 repo launch first-frame guardrail 유지
- shell config write는 기존 trust/symlink policy를 공유

## 8. 이후 신규 기능의 dual-surface 규칙

이전 PM review의 개선 후보는 다음처럼 재정렬한다.

| 기존 후보 | 평가 후 결정 | CLI/TUI 적용 원칙 |
|---|---|---|
| Instant First Frame | 유지, Horizon 2 performance lane | core read model을 공유하고 TUI만 progressive loading 사용 |
| Typed Config + Doctor | 다음 discovery 후보 | `owt doctor`와 TUI Diagnostics를 같은 release에 제공 |
| Versioned JSON Output | 유지하되 transport exemption | JSON은 CLI renderer 기능이며 TUI는 동일 typed result를 시각화 |
| Explainable Cleanup Review | Horizon 1로 승격 | CLI dry-run result와 TUI preview가 같은 candidate/reason model 사용 |
| PR Review Workspace | parity foundation 이후 | `owt review <PR>`와 TUI Review flow를 동시에 설계·출시 |

## 9. 제안 코드 구조

최종 이름은 implementation discovery에서 조정하되 ownership은 다음처럼 분리한다.

```text
src/
├── usecases/
│   ├── worktree_list.rs
│   ├── worktree_create.rs
│   ├── worktree_delete.rs
│   ├── worktree_prune.rs
│   ├── pr_status.rs
│   ├── commit_tree.rs
│   └── search.rs
├── cli/
│   ├── parse.rs
│   └── render.rs
├── app.rs               # TUI state/event adapter
├── ui/                  # TUI view rendering
├── git.rs               # low-level Git adapter
└── types.rs             # shared domain/request/result types
```

핵심 dependency direction:

```text
CLI parser ─┐
            ├─> typed use-case ─> Git/config/tmux adapters
TUI event ──┘          │
                       ├─> CLI renderer
                       └─> TUI state/view model
```

TUI가 CLI binary를 subprocess로 재호출하지 않는다. “wrapper”는 product semantics의 wrapper이지 process wrapper가 아니다.

## 10. Test strategy와 release gate

### Contract tests

- request validation
- candidate selection and target resolution
- live safety guard
- typed result/error/progress

### Adapter parity tests

- 같은 fixture에서 CLI request와 TUI request가 동일한 use-case request를 생성
- 같은 core result가 CLI record와 TUI state에 손실 없이 표현
- option별 semantic scenario matrix

### Manual TUI scenarios

1. completed PR cleanup dry-run → excluded reason 확인 → confirm → progress/result
2. status/PR/path 검색
3. custom path + base + tmux override create
4. arbitrary branch PR status
5. commit graph paging/limit
6. repo 밖 global home에서 clone/setup, repo 안에서는 즉시 list

### Definition of Done

- code, tests, relevant Korean SSOT, README/README.ko, keybinding/help가 같은 change에 포함됨
- parity registry가 Full로 갱신됨
- `cargo test`, strict Clippy, fmt, Windows target check 통과
- destructive action은 live check evidence가 있음
- 성능-sensitive change는 before/after benchmark가 있음

## 11. 권장 실행 순서

1. capability registry와 parity contract를 먼저 고정한다.
2. **prune vertical slice** 하나로 shared use-case 구조를 검증한다.
3. 구조가 검증되면 search와 create를 같은 pattern으로 이동한다.
4. PR/commit read flows를 modal/full-screen state로 완성한다.
5. 마지막으로 global home을 추가해 clone/init/setup/help/version parity를 닫는다.
6. 이후 모든 신규 기능은 CLI와 TUI를 한 backlog item/acceptance gate로 취급한다.

이 순서는 architecture refactor를 먼저 크게 끝내지 않는다. 사용자에게 보이는 parity gap 하나씩 해결하면서 shared core를 추출하는 vertical-slice 방식이다.
