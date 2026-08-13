---
title: owt Product Opportunity Review
date: 2026-08-10
scope: feature discovery and prioritization
methods:
  - product trio ideation
  - impact-effort-risk-alignment prioritization
status: discovery backlog
---

# owt Product Opportunity Review

## 1. 분석 전제

### 제품 목표

여러 Git worktree를 쓰는 terminal developer가 작업 context를 더 빠르고 안전하게 만들고, 찾고, 전환하고, 정리하게 한다.

### 핵심 사용자

- 여러 branch를 동시에 유지하는 individual developer
- PR을 로컬에서 반복 검토하는 reviewer
- 진행 중인 작업을 건드리지 않고 긴급 수정하는 hotfix operator
- stable plain CLI를 이용하는 script와 AI agent

### 원하는 결과와 측정 후보

| Outcome | Metric candidate |
|---|---|
| 시작 지연 감소 | 30개 worktree 기준 TUI first-frame p50/p95, 실행되는 Git subprocess 수 |
| 새 context 준비 시간 감소 | branch/PR 선택부터 usable shell까지 걸린 시간과 성공률 |
| 안전한 정리 증가 | cleanup 후보 중 사용자가 승인한 비율, destructive false positive 0건 |
| 초기 설정 성공률 증가 | 첫 실행에서 shell integration/config validation까지 완료한 비율 |
| 자동화 신뢰성 증가 | structured-output parse failure와 breaking schema incident 0건 |

현재 repository에는 공개 GitHub issue backlog가 없고 사용자 인터뷰·usage telemetry도 없다. 아래 점수는 README, SSOT, 최근 review, 성능 계획, 현재 구현에 기반한 가설이며 customer evidence가 아니다.

## 2. Opportunity map

| Opportunity | 현재 friction | 관련 outcome |
|---|---|---|
| O1. 큰 workspace를 즉시 열고 싶다 | worktree마다 status/log/ahead-behind subprocess를 실행해 첫 화면을 막음 | 시작 지연 감소 |
| O2. PR 검토용 context를 한 명령으로 만들고 싶다 | PR 확인, fetch, branch/path 결정, 생성, 진입, 정리를 사용자가 조합함 | context 준비 시간 감소 |
| O3. 삭제 전에 왜 안전한지 이해하고 싶다 | prune log는 있지만 TUI/CLI cleanup 경험과 reason presentation이 분산됨 | 안전한 정리 증가 |
| O4. 설정 오류를 실행 전에 알고 싶다 | hand-written config와 platform capability 실패가 실제 action 시점에 드러남 | 초기 설정 성공률 증가 |
| O5. agent/script가 안정적으로 통합하고 싶다 | TSV는 단순하지만 escaping/schema negotiation과 typed diagnostics가 없음 | 자동화 신뢰성 증가 |

## 3. Product trio 아이디어

### Product Manager 관점

| ID | Idea | Value hypothesis |
|---|---|---|
| PM1 | **PR Review Workspace** | `owt review <PR-number-or-URL>`가 PR fetch, worktree 생성, shell/tmux handoff, 완료 후 cleanup까지 연결하면 reviewer의 반복 작업을 핵심 workflow로 소유할 수 있다. |
| PM2 | **Lifecycle Inbox** | merged/closed/stale/large worktree를 reason과 함께 정리 후보로 모으면 방치된 worktree와 disk 사용량을 줄일 수 있다. |
| PM3 | **Creation Profiles** | `review`, `feature`, `hotfix` profile별 base branch, copy files, post-add setup을 재사용하면 팀별 생성 시간을 줄일 수 있다. |
| PM4 | **Guided First Run** | repo mode, shell integration, editor/terminal, optional `gh`/tmux를 점검하는 onboarding이 설치 후 첫 성공률을 높일 수 있다. |
| PM5 | **Workspace Manifest** | 현재 worktree/branch context를 manifest로 export/restore하면 새 장비나 재클론에서 workspace 복구 비용을 줄일 수 있다. |

### Product Designer 관점

| ID | Idea | Experience hypothesis |
|---|---|---|
| D1 | **Instant First Frame** | path/branch만 먼저 그리고 status/commit/ahead/PR을 점진적으로 채우면 큰 workspace에서도 제품이 즉각 반응한다고 느낀다. |
| D2 | **Context Detail Pane** | 선택 worktree에 PR link, last commit, upstream, dirty summary, disk size를 한곳에 보여주면 action 전 판단이 빨라진다. |
| D3 | **Command Palette** | fuzzy action palette가 keybinding을 외우지 않은 사용자에게 기능 발견성과 keyboard-first 흐름을 동시에 제공한다. |
| D4 | **Explainable Cleanup Review** | 삭제 후보별 `why removable`, 제외 사유, 영향, progress/retry를 보여주면 cleanup 신뢰와 승인율이 높아진다. |
| D5 | **Creation Preview** | 생성 전 target path, base SHA, copied files, script/tmux action을 preview하면 잘못된 branch/path 설정을 조기에 발견한다. |

### Software Engineer 관점

| ID | Idea | Technical leverage hypothesis |
|---|---|---|
| E1 | **Versioned JSON Output** | `--format json --schema-version 1`을 list/search/prune에 추가하면 agent/script가 display-oriented TSV에 덜 의존한다. |
| E2 | **Typed Config + Doctor** | `serde`/`toml` schema와 `owt doctor`/`owt config check`가 unknown key, invalid path, trust boundary, optional dependency 문제를 action 전에 설명한다. |
| E3 | **Operation Journal** | create/delete/prune의 단계와 결과를 기록하면 partial failure를 진단하고 안전하게 retry하기 쉬워진다. |
| E4 | **Platform Capability Adapter** | tty, clipboard, terminal, editor, shell integration을 capability로 모델링하면 Windows/Linux/macOS 지원 수준과 failure message가 일관된다. |
| E5 | **Event Hook API** | typed lifecycle event가 사용자 자동화를 확장할 수 있지만 post-add script보다 더 넓은 trust surface를 만든다. |

## 4. 우선순위 방법

Customer reach 데이터가 없으므로 RICE 숫자를 만들지 않고 5점 척도를 사용했다.

- Impact: 위 outcome을 움직일 잠재력
- Strategic alignment: worktree context 전환, keyboard-first TUI, agent CLI와의 적합성
- Confidence: 현재 코드·문서에서 문제 근거가 확인되는 정도
- Effort: 1이 작고 5가 큼
- Risk: product/technical uncertainty, 1이 낮고 5가 큼
- Priority score: `0.35×Impact + 0.25×Alignment + 0.20×Confidence + 0.10×(6−Effort) + 0.10×(6−Risk)`

## 5. Top 5

| Rank | Candidate | Impact | Align | Confidence | Effort | Risk | Score | 선택 이유 |
|---:|---|---:|---:|---:|---:|---:|---:|---|
| 1 | **Instant First Frame** | 5 | 5 | 5 | 4 | 2 | **4.60** | 이미 O(N) subprocess 병목과 구현 계획이 확인되어 가장 증거가 강하다. 첫 체감 품질을 직접 개선하며 live mutation recheck와도 양립한다. |
| 2 | **Typed Config + Doctor** | 4 | 5 | 4 | 3 | 2 | **4.15** | install/config/platform failure를 사전에 설명하고 최근 강화한 trust boundary를 사용자 가치로 바꾼다. 독립적으로 작게 출시할 수 있다. |
| 3 | **Versioned JSON Output** | 4 | 5 | 4 | 3 | 2 | **4.15** | agent/script가 명시된 핵심 actor이므로 전략 적합성이 높다. 기존 TSV를 유지한 additive change로 위험을 제한할 수 있다. |
| 4 | **Explainable Cleanup Review** | 4 | 5 | 4 | 3 | 3 | **4.05** | 이미 prune safety와 reason log가 있어 foundation이 준비되어 있다. 안전성은 유지하면서 cleanup 사용 빈도와 신뢰를 높일 수 있다. |
| 5 | **PR Review Workspace** | 5 | 5 | 3 | 4 | 4 | **4.00** | `owt`의 reviewer use case를 end-to-end workflow로 만든다는 점에서 가장 차별화 가능성이 크다. PR identifier/ref 처리와 cleanup semantics는 먼저 검증해야 한다. |

### 1. Instant First Frame

한 문장: TUI는 `git worktree list --porcelain` 결과로 먼저 열고, visible-first bounded worker가 status/commit/ahead/PR을 path key로 갱신한다.

- 선택 근거: `docs/solutions/performance/worktree-loading-performance-plan.md`에 현재 command fan-out과 안전 경계가 구체적으로 기록되어 있다.
- 핵심 가정: 10개 이상 worktree를 가진 사용자가 startup latency를 반복적으로 경험한다.
- 가장 싼 검증: 1/10/30/100 worktree benchmark와 subprocess count를 먼저 측정한다. 첫 frame 목표는 local warm repo p95 200ms 이하로 임시 설정한다.
- guardrail: unloaded status는 `unknown`이며 delete/prune은 계속 live status를 재조회한다.

### 2. Typed Config + Doctor

한 문장: `owt doctor`가 repository mode, config parse/precedence, unsafe ignored option, copy path, Git/gh/tmux/editor/terminal, shell integration을 read-only로 진단한다.

- 선택 근거: 현재 config trust boundary와 platform capability가 여러 문서·코드 경로에 흩어져 있다.
- 핵심 가정: 많은 초기 실패가 Git 자체보다 config와 optional dependency에서 발생한다.
- 가장 싼 검증: parser 교체 전에 기존 parser를 사용하는 read-only `doctor --format text` prototype으로 dogfood한다.
- guardrail: doctor는 config를 자동 수정하거나 project config의 trusted execution을 enable하지 않는다.

### 3. Versioned JSON Output

한 문장: `worktree list`, `search`, `pr status`, `prune --dry-run`에 additive `--format json`과 top-level `schema_version`을 제공한다.

- 선택 근거: agent/script actor와 plain CLI contract가 이미 제품 전략에 포함되어 있다.
- 핵심 가정: TSV escaping과 고정 column보다 typed fields가 integration 비용을 실질적으로 줄인다.
- 가장 싼 검증: `worktree list --format json` 한 command만 구현한 schema fixture를 `.agents/` workflow에서 사용해 본다.
- guardrail: 기존 default TSV와 field meaning은 변경하지 않는다.

### 4. Explainable Cleanup Review

한 문장: TUI와 CLI가 동일한 candidate/reason model을 사용해 removable/excluded 이유, disk impact, live-check 결과, progress와 retry를 보여준다.

- 선택 근거: safe prune 기반과 tab-separated reason log가 이미 있어 완전 신규 기능보다 leverage가 높다.
- 핵심 가정: 사용자는 자동 삭제보다 이해 가능한 추천과 명시적 승인에 더 높은 신뢰를 보인다.
- 가장 싼 검증: mutation 없이 `owt worktree prune --dry-run --format json`과 TUI read-only preview부터 제공한다.
- guardrail: unknown/dirty/current/HEAD/bare/detached는 계속 제외하며 live recheck를 우회하지 않는다.

### 5. PR Review Workspace

한 문장: `owt review 123` 또는 PR URL로 remote ref를 확인하고 격리된 review worktree를 만든 뒤 shell/tmux로 handoff한다.

- 선택 근거: reviewer가 SSOT actor이고 PR status와 completed-PR cleanup foundation이 이미 존재한다.
- 핵심 가정: 사용자는 PR review 때 수동 fetch/branch/worktree 조합을 자주 반복한다.
- 가장 싼 검증: GitHub-only `owt review <number> --dry-run`으로 resolved repo/ref/path/action plan을 출력해 dogfood한다.
- guardrail: fork PR ref, auth/network failure, existing branch/worktree collision을 명시적으로 처리하고 자동 cleanup은 기본 off로 둔다.

## 6. 후순위와 이유

| Candidate | 결정 | 이유 |
|---|---|---|
| Lifecycle Inbox | Top 5의 cleanup review에 먼저 흡수 | disk size/staleness 기준은 사용자 수요와 안전 threshold 검증이 필요함 |
| Creation Profiles / Preview | 후속 | 현재 config/copy/post-add/tmux로 일부 해결되며 profile taxonomy 근거가 부족함 |
| Guided First Run | Doctor에 먼저 흡수 | 별도 wizard보다 진단 command가 더 작고 CLI 정체성에 맞음 |
| Context Detail Pane | 후속 | 정보 가치는 있으나 좁은 terminal에서 layout cost가 크고 사용 빈도 근거가 없음 |
| Command Palette | 후속 | discoverability 문제의 실제 크기가 측정되지 않았고 help modal이 존재함 |
| Workspace Manifest | 보류 | cross-machine path/config portability와 remote branch state가 복잡함 |
| Operation Journal | enabling investment | multi-step retry 문제가 관측되면 PR review/create reliability 기반으로 재평가 |
| Platform Capability Adapter | enabling investment | Windows runtime smoke와 platform-specific failure data를 먼저 수집 |
| Event Hook API | 보류 | trust surface와 support burden이 크며 post-add script로 상당 부분 대체 가능 |

## 7. 권장 discovery 순서

1. **Benchmark week**: first-frame baseline과 command count를 수집하고 Instant First Frame의 성공 threshold를 확정한다.
2. **Two small prototypes**: `owt doctor` read-only output과 `worktree list --format json` schema fixture를 만든다.
3. **Workflow validation**: `owt review <PR> --dry-run`을 실제 reviewer workflow 5~10회에 사용해 missing cases를 기록한다.
4. **Cleanup concept test**: current prune reason을 기반으로 read-only cleanup preview를 만들고 후보 이해도/승인 의향을 확인한다.
5. 검증 결과로 top 5 순위를 다시 계산한다. 구현량이 아니라 outcome signal이 없는 candidate를 뒤로 보낸다.

## 8. 제품 결정

바로 delivery로 넘길 1순위는 **Instant First Frame**이다. 이미 문제와 구현 경로가 구체적이며, 안전성 개선과 충돌하지 않고 모든 TUI 사용자의 반복 체감에 영향을 준다.

동시에 discovery prototype으로는 **Typed Config + Doctor**와 **Versioned JSON Output**이 가장 작다. **PR Review Workspace**는 차별화 잠재력이 가장 크지만, fork PR과 cleanup policy를 검증한 뒤 delivery 범위를 확정한다.
