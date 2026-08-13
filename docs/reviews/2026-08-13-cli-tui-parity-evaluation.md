---
title: CLI–TUI Capability Parity Product Decision
date: 2026-08-13
scope: product architecture, feature parity, roadmap
decision_owner_constraint: TUI is a convenient wrapper over CLI capabilities
status: additional-validation-with-architecture-decision
---

# 제품 의사결정 평가: CLI–TUI Capability Parity

## 1. 판정

- 결정 상태: **추가 검증**
- 한 문장 결론: 성장 우선순위는 사용자 행동 근거가 부족해 추가 검증이 필요하지만, **CLI capability를 정본으로 두고 TUI가 모든 user-facing capability를 같은 use-case layer에서 제공한다**는 제품 구조는 즉시 채택해야 한다.
- 결론을 좌우한 근거: 현재 CLI와 TUI가 동일한 Git helper를 일부 공유하지만 orchestration은 `src/main.rs`와 `src/app.rs`에 갈라져 있으며, 실제로 TUI prune이 CLI prune의 completed-PR cleanup과 dry-run을 제공하지 않는 기능 차이가 발생했다.
- 결론을 좌우한 미확인 또는 반대 증거: 사용자 인터뷰, usage telemetry, command별 사용 빈도, startup latency baseline이 없다. 따라서 parity 이후 어떤 신규 기능이 adoption을 가장 크게 높일지는 확인되지 않았다.

제품 책임자가 제시한 parity 원칙은 시장 가설이 아니라 non-negotiable product constraint로 취급한다. 전체 제품 투자 판정이 `추가 검증`이어도 관찰된 parity debt는 바로 수정한다.

### 구현 상태 (2026-08-13 working tree)

첫 vertical slice인 completed-PR cleanup parity를 구현했다. `src/worktree_prune.rs::run_prune`이 CLI와 TUI의 shared core이며, TUI `x`는 `CleanupPreview`에서 eligible/excluded reason을 표시한 뒤 명시적 확인을 받아 실행한다. 실행은 preview cache를 신뢰하지 않고 같은 core에서 candidate와 live status를 다시 판정한다.

## 2. 평가 대상과 고정 기준

- 결정할 것: CLI/TUI 관계를 독립 구현, TUI의 CLI subprocess 호출, shared typed use-case + surface adapters 중 무엇으로 구성할지 결정한다.
- 후보 단위: **Developer productivity tools → multi-worktree terminal developer × 모든 worktree lifecycle 업무를 안전하게 실행 × individual developer 또는 engineering team lead**
- 비교 후보:
  1. A — 현재처럼 CLI/TUI orchestration을 별도로 유지
  2. B — TUI가 `owt ...` subprocess를 호출하는 literal wrapper
  3. C — CLI와 TUI가 동일한 typed use-case layer를 호출하는 adapter 구조
- 비교 전에 고정한 기준: semantic capability parity, destructive safety의 단일 경계, 사용자 피드백 품질, 성능, testability, 신규 기능 추가 비용, migration risk
- 시간 범위: parity debt는 다음 minor release 전, 전체 구조와 global capability parity는 2~3개 flexible release horizon
- 범위 포함: 모든 user-facing command와 option이 표현하는 semantic capability
- 범위 제외: TSV/JSON/color 같은 transport-only presentation, shell completion, 내부 debug command `test-cd`

### 확인된 사실

- CLI user-facing command는 `clone`, `init`, `setup`, `worktree list/create/delete/prune`, `pr status`, `commit tree`, `search`, help/version이다.
- TUI는 list/create/delete, PR column, commit detail, filter를 제공하며 fetch/pull/push/merge/editor/terminal 같은 TUI-only action도 제공한다.
- CLI prune은 stale metadata 정리와 completed-PR worktree cleanup을 함께 실행하고 `--dry-run`을 제공한다.
- TUI `x`는 `git::prune_worktrees`만 호출해 stale metadata만 정리한다.
- CLI search는 path/name/branch/status/PR을 검색하지만 TUI filter는 name/branch만 비교한다.
- 현재 working tree에서 TUI add modal은 base, explicit worktree path, per-run tmux override를 모두 지원한다. 공용 create orchestration 추출과 preview는 후속 architecture hardening이다.

### 추론

- 현재 차이는 문서 누락이 아니라 orchestration이 두 surface에 중복된 구조에서 생긴 parity regression이다.
- TUI가 CLI subprocess를 호출하면 parity는 쉬워 보이지만 progress, cancellation, typed errors, terminal lifecycle, testability가 나빠진다.
- shared use-case layer가 CLI 정본 원칙과 TUI UX를 동시에 만족시키는 가장 작은 지속 가능한 구조다.

### 미확인 가정

- global command인 clone/init/setup도 TUI에서 접근할 필요가 있다.
- TUI에서 모든 CLI option을 그대로 노출하기보다 동일한 사용자 결과를 더 적합한 interaction으로 제공하면 parity로 인정된다.
- parity 개선이 실제 사용 빈도나 retention을 높인다.

## 3. 모호성 감사

| 등급 | 모호한 항목 | 가능한 해석 | 결정 영향 | 필요한 답 또는 작업 가정 |
|---|---|---|---|---|
| Decision-sensitive | “모든 기능”의 범위 | workspace command만 / clone·setup 포함 모든 user-facing command | global home/command palette 범위가 달라짐 | 사용자의 문장을 엄격히 적용해 모든 user-facing command를 포함한다. |
| Decision-sensitive | option parity | flag 1:1 / semantic outcome parity | modal complexity와 UX가 달라짐 | semantic parity를 원칙으로 하고 transport-only flag는 제외한다. |
| Decision-sensitive | TUI가 CLI wrapping이라는 의미 | subprocess 호출 / shared application API의 UI adapter | architecture, progress, error handling이 달라짐 | shared typed use-case adapter를 선택한다. |
| Editorial | “CLI가 지원하는 기능” | 현재 GA command / 향후 experimental command 포함 | release gate 문구가 달라짐 | GA capability는 동시 parity 필수, experimental은 명시적 예외와 만료일을 요구한다. |

## 4. 세 가지 근거

| 판단 근거 | 상태 | 확인된 근거 | 미확인 | 반대 증거 | 최소 기준 판정 |
|---|---|---|---|---|---|
| Customer Evidence | 미확인 | README/SSOT에 regular user, reviewer, hotfix operator, agent actor와 반복 workflow가 명시됨 | 실제 사용자 행동, command frequency, parity 불만, 시간 절감 baseline | 공개 issue backlog가 비어 있음 | 미통과 |
| Market Evidence | 미확인 | Git worktree와 terminal workflow라는 접근 가능한 OSS category에 이미 배포됨 | active installs, retention, comparable tool adoption, buyer/budget | 유료 구매 시장과 예산 근거 없음 | 미통과 |
| Our Advantage | 부분 확인 | regular repo + `.bare`, TUI + agent CLI, PR-aware cleanup, shell/tmux handoff를 한 binary에서 제공 | 경쟁 제품 대비 측정된 속도·완결성 우위 | 기능 breadth 자체는 복제 가능 | 미통과 |

## 5. 후보 비교

| 후보 | Parity | Safety | UX/Performance | Testability | Migration | 결론 |
|---|---|---|---|---|---|---|
| A. 독립 orchestration 유지 | 낮음 | 경계 중복 | surface별 최적화 가능 | parity 회귀를 잡기 어려움 | 낮음 | 중단 |
| B. TUI → CLI subprocess | 높음 | CLI 경계 재사용 | progress/cancel/terminal 제어가 약함 | end-to-end 의존 증가 | 중간 | 보류 |
| C. shared typed use-case + adapters | 높음 | 단일 mutation boundary | TUI-native progress와 CLI output 분리 가능 | contract + adapter test 가능 | 중간~높음 | 선택 |

## 6. 현재 Capability Parity Audit

판정 기준은 `Full`, `Partial`, `Missing`, `Exempt`다. `Partial`은 결과 또는 중요한 option이 빠진 상태다.

| CLI capability | CLI surface | TUI equivalent | 판정 | Gap |
|---|---|---|---|---|
| repository context/list | `owt [PATH]`, `worktree list [--pr]` | launch context, table, automatic PR refresh | Full | output shape는 surface-specific |
| clone `.bare` workspace | `owt clone` | Global Home 또는 repository `N` clone modal | Full in working tree | terminal 복원 뒤 shared CLI clone action |
| init conversion guide | `owt init` | Global Home entry 또는 repository `I` | Full in working tree | repository entry는 current project root로 CLI guide 실행 |
| shell setup | `owt setup` | Global Home 또는 repository `S` | Full in working tree | terminal 복원 뒤 CLI setup action |
| worktree create | `worktree create --base --worktree-path --tmux` | add modal + base/path/tmux override | Full in working tree | `Ctrl+p` explicit path, `Ctrl+t` default/on/off; shared orchestration extraction은 후속 |
| worktree delete | `delete --force --branch` | confirm modal의 force/branch toggle, batch 확장 | Full | TUI가 batch superset 제공 |
| completed-PR prune | `prune [--dry-run]` | `x` cleanup preview + confirm | **Full** | shared core와 live safety recheck를 사용; CLI dry-run의 terminal prompt는 TUI explicit confirm으로 표현 |
| PR status | `pr status --branch/--all` | `R` PR detail/query modal | Full in working tree | selected/all/arbitrary branch를 background query |
| commit graph | `commit tree -n` | `C` full-screen commit tree | Full in working tree | background query, `+`/`-` limit 조절 |
| search | `search [--pr]` across path/name/branch/status/PR | `/` unified filter | Full in working tree | shared matcher가 path/name/branch/status/PR label을 처리 |
| help | `--help`와 command/action help | contextual keybinding help + Global Home command browser | Full in working tree | command group과 `owt <command> --help` route 제공 |
| version | `--version` | Global Home/repository About modal | Full in working tree | package version 표시 |
| debug shell handoff | `test-cd` | 없음 | Exempt | internal debug command, product capability 아님 |
| serialization | TSV, 향후 JSON/color flags | visual widgets | Exempt | transport parity가 아니라 semantic parity 적용 |

현재 user-facing semantic capability 기준:

- Full: 12
- Partial: 0
- Missing: 0
- Parity coverage: `Full / (Full + Partial + Missing) = 100%`
- Weighted coverage(`Full=1`, `Partial=0.5`): `100%`

## 7. 주요 리스크

| 순위 | Fails if | 영향받는 결정·근거 | 현재 증거 | 영향 | 가능성 | 가장 싼 검증 | Kill / Change / Continue 기준 | 잔여 위험 |
|---:|---|---|---|---|---|---|---|---|
| 1 | shared layer extraction이 기존 destructive guard를 우회한다 | 제품 범위·Our Advantage | live status guard와 prune policy가 여러 module에 있음 | 데이터 손상 | unknown | prune/delete request-result contract test를 extraction 전에 작성 | guard가 adapter에 남으면 change; core에 한 번만 존재하면 continue | Git 외부 상태 race |
| 2 | TUI에 모든 option을 그대로 넣어 modal이 복잡해진다 | 제품 구성·Customer Evidence | create 옵션이 이미 분기됨 | 사용성 악화 | unknown | progressive disclosure add modal prototype | 기본 create가 2-step 이상 느려지면 change | advanced option 발견성 |
| 3 | global home이 기존 `owt`의 빠른 repo 진입을 늦춘다 | 제품 범위·Our Advantage | 현재 repo에서는 바로 list로 진입 | 핵심 UX 퇴행 | unknown | repo context에서는 home을 건너뛰는 spike | first frame budget 초과 시 global flow 분리 | cold Git latency |
| 4 | parity registry가 실제 behavior가 아닌 체크박스가 된다 | 제품 구성 | 현재 문서와 코드 drift 경험 | 회귀 지속 | 높음 | command registry에서 adapter mapping을 compile/test gate로 요구 | 신규 GA capability가 mapping 없이 merge되면 kill | semantic 품질은 scenario test 필요 |
| 5 | 시장 가치가 낮아 architecture 비용만 증가한다 | 메인 시장·Customer/Market Evidence | 사용자 telemetry 없음 | opportunity cost | unknown | parity debt vertical slice 후 dogfood task timing | task completion 개선이 없으면 신규 global scope 축소 | maintainer preference bias |

## 8. 네 가지 결정

### 메인 시장

- 초기 진입: 여러 branch/worktree를 동시에 사용하는 terminal-first individual developer와 reviewer
- 장기 확장: engineering teams와 AI agents가 동일한 worktree lifecycle을 공유하는 workflow
- 결정 문장: multi-worktree terminal developer의 context 생성·탐색·전환·정리를 한 제품에서 제공하되, buyer와 budget evidence가 없으므로 상업 시장 판정은 보류한다.

### 제품 범위

| 직접 제공 | 연동·파트너 | 범위 밖 |
|---|---|---|
| 모든 CLI semantic capability의 TUI equivalent, shared validation/mutation, progress/error/result model | Git, GitHub CLI, tmux, editor, terminal, shell | raw Git hosting UI, IDE 자체 기능, transport serialization의 TUI 모사 |

### 제품 구성

- 구성: **하나의 제품, 하나의 application/use-case core, CLI와 TUI 두 adapter**
- 구매자·예산·사용 흐름 근거: 같은 사용자가 terminal에서 automation과 interactive workflow를 오가며 같은 repository state를 다룬다.
- 기술 기반과 판매 상품의 구분: CLI/TUI를 별도 상품이나 module로 나누지 않는다. output renderer와 interaction adapter만 분리한다.

### 경쟁·포지셔닝

- 전략: 특정 업무 중심 통합 제품
- 직접 경쟁: 미확인; 최신 외부 경쟁 조사는 이번 architecture 결정 범위에 포함하지 않음
- 부분 경쟁: raw `git worktree`, shell aliases/scripts, standalone worktree TUIs
- 인접·보완: GitHub CLI, tmux, IDE/editor
- 포지셔닝 문장: 여러 branch를 동시에 다루는 terminal developer가 context를 안전하게 만들고 전환하고 정리하도록 돕는 worktree workflow 제품. raw Git 조합과 달리 동일한 capability를 automation-friendly CLI와 discoverable TUI에서 일관되게 제공한다.

## 9. 최종 결정 기록

- 선택한 고객군과 핵심 업무: multi-worktree terminal developer/reviewer의 worktree lifecycle 관리
- 직접 지원 범위: 모든 user-facing CLI semantic capability의 TUI equivalent
- 제품·모듈·패키지 구성: shared typed use-case core + CLI adapter + TUI adapter
- 경쟁·포지셔닝 전략: 특정 업무 중심 통합 제품
- 핵심 근거: 관찰된 prune/search/create parity debt와 중복 orchestration
- 반대 증거: user demand와 시장 규모 증거가 없음
- 남은 위험: global TUI scope creep, modal complexity, extraction 중 safety regression

## 10. 다음 행동

- 결정을 바꿀 수 있는 최소 검증: CLI prune과 TUI cleanup이 같은 request/result use-case를 호출하는 한 개 vertical slice를 만들고 scenario parity/safety/usability를 검증한다.
- 책임자: maintainer
- 기한: 다음 minor release candidate 전
- 판정 기준:
  - Continue: CLI/TUI가 같은 core 결과를 사용하고 live safety test가 통과하며 TUI dry-run/confirm flow가 이해 가능함
  - Change: subprocess wrapper가 더 단순해도 progress/typed error 요구를 만족하거나 shared extraction 비용이 예상보다 큼
  - Kill: extraction이 mutation safety를 약화하거나 기본 TUI task 시간을 유의미하게 늘림
