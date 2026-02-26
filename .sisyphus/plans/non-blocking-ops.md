# Non-Blocking Git Operations for TUI

## TL;DR

> **Quick Summary**: 모든 git 작업(delete, fetch, pull, push, add, merge)이 TUI를 블로킹하는 문제를 해결. 각 작업을 백그라운드 스레드로 이동하고, 작업 중에도 UI 조작이 가능하도록 이벤트 루프를 리팩터링한다.
> 
> **Deliverables**:
> - 6개 git 작업 모두 백그라운드 스레드에서 실행
> - 작업 중 리스트 탐색, 다른 작업 트리거, 모달 열기 등 전체 키보드 입력 가능
> - 워크트리별 스피너 표시 (선택된 항목이 아닌 실제 작업 대상에 표시)
> - 낙관적 인메모리 업데이트로 동기 refresh 제거
> 
> **Estimated Effort**: Medium
> **Parallel Execution**: YES - 4 waves
> **Critical Path**: Task 1 → Task 2 → Task 3 → Task 4

---

## Context

### Original Request
워크트리 삭제 시 TUI가 멈추고, 모든 워크트리가 삭제 완료될 때까지 다른 작업을 할 수 없음.
"유저 입장에서 최대한 빠르고 막힘없는 UX를 제공하는게 핵심" — 모든 git 작업에 적용.

### Interview Summary
**Key Discussions**:
- 삭제만이 아니라 fetch/pull/push/add/merge 전부 non-blocking 필요 확인
- 현재 delete만 부분적으로 async (스레드는 있지만 UI가 블로킹)
- fetch/pull/push/add/merge는 완전 동기 (메인 스레드에서 git 명령 실행)

**Research Findings**:
- 이벤트 루프 (app.rs:130-185): `is_*` 플래그가 true이면 `do_*()` 실행 후 `continue` → `handle_events()` 자체를 건너뜀
- `handle_events` (app.rs:315-318): `AppState::Deleting` 등의 상태에서 모든 키 입력 무시
- `refresh_worktrees()` (app.rs:805-819): 모든 작업 완료 후 동기적으로 3N개 git 명령 실행 (N=워크트리 수) — 숨겨진 병목
- `git::*` 함수들은 모두 `Command::new("git")` 사용 — 스레드 안전, `Send + 'static` 가능
- 기존 패턴 (delete/post_add_script): `std::thread::spawn` + `mpsc::channel` + `try_recv()` 폴링

### Metis Review
**Identified Gaps** (addressed):
- `refresh_worktrees()`가 실제 최대 병목 → 낙관적 인메모리 업데이트로 해결
- `selected_worktree()` 드리프트 — 디스패치 시점에 데이터 캡처 필수
- `do_add_worktree()`의 다단계 파이프라인 (add → copy → script → refresh) → 스레드 내 순차 실행
- 이벤트 드레이닝 패턴 (기존 `while event::poll` 루프) → non-blocking이므로 제거
- `try_recv() → Disconnected` 시 에러 표시 누락 → 에러 메시지 추가
- Git lock 충돌 위험 → 워크트리별 + bare repo별 충돌 방지

---

## Work Objectives

### Core Objective
모든 git 작업을 백그라운드 스레드로 이동하여, 작업 실행 중에도 TUI가 완전히 반응하도록 한다.

### Concrete Deliverables
- `src/types.rs`: `BackgroundOp`, `OpResult` 타입 추가. 블로킹 `AppState` variant 6개 제거.
- `src/app.rs`: 이벤트 루프 리팩터링, 6개 `do_*` 메서드 비동기화, 통합 폴링 시스템
- `src/ui/main_view.rs`: 워크트리별 작업 상태 표시, non-blocking 선택 커서
- `src/ui/confirm_modal.rs`: 활성 작업 중인 워크트리 삭제 방지

### Definition of Done
- [ ] `cargo build` 에러 없음
- [ ] `cargo test` 모두 통과
- [ ] 6개 작업 모두 실행 중 j/k 탐색, q 종료, ? 도움말 가능
- [ ] `grep -c "Ignore input during operations" src/app.rs` → 0

### Must Have
- 6개 작업(delete, fetch, pull, push, add, merge) 전부 non-blocking
- 작업 중 키보드 탐색 가능 (j/k, gg, G, /, q, ?, a, d 등)
- 워크트리별 스피너 표시 (해당 행의 마지막 커밋 열에 표시)
- 작업 결과 메시지 (성공/실패) 기존과 동일하게 표시
- verbose 모드 (`cmd_detail`) 기존과 동일하게 동작
- 동일 워크트리에 대한 충돌 작업 방지 (예: pull 중 같은 워크트리 delete 불가)

### Must NOT Have (Guardrails)
- `git.rs` 모듈 변경 금지 — 모든 `git::*` 함수는 현재 상태 그대로 사용
- `app.rs`를 여러 모듈로 분리하지 않음 — 이 PR의 범위가 아님
- 프로그레스 바, 작업 큐 UI, 작업 취소 UI 등 새로운 UI 요소 추가 금지
- `event::poll` 타임아웃 (100ms) 변경 금지
- `prune_worktrees()`, `list_local_branches()` 비동기화 — 범위 밖
- 동시 다중 작업 지원 — v1은 한 번에 하나의 작업만 (구조는 미래 확장 가능하게)
- AI slop: 과도한 주석, 불필요한 추상화, 제네릭 이름 사용 금지

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: YES (cargo test)
- **Automated tests**: Tests-after (기존 테스트 통과 확인 + 새 테스트 불필요)
- **Framework**: cargo test

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **TUI/CLI**: Use interactive_bash (tmux) — Run owt, send keystrokes, validate output
- **Build**: Use Bash (cargo) — Build, test, clippy

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Foundation — types infrastructure):
└── Task 1: Background operation types [quick]

Wave 2 (Core — main refactoring):
└── Task 2: App refactoring - event loop + all operations (depends: 1) [deep]

Wave 3 (UI — rendering updates):
└── Task 3: UI rendering for background operations (depends: 2) [unspecified-high]

Wave 4 (Verification):
└── Task 4: Build + test + manual QA (depends: 3) [quick]

Wave FINAL (After ALL tasks — independent review):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real manual QA (unspecified-high)
└── Task F4: Scope fidelity check (deep)

Critical Path: Task 1 → Task 2 → Task 3 → Task 4 → F1-F4
```

### Dependency Matrix

| Task | Depends On | Blocks | Wave |
|------|-----------|--------|------|
| 1 | — | 2 | 1 |
| 2 | 1 | 3 | 2 |
| 3 | 2 | 4 | 3 |
| 4 | 3 | F1-F4 | 4 |
| F1-F4 | 4 | — | FINAL |

### Agent Dispatch Summary

- **Wave 1**: 1 task — T1 → `quick`
- **Wave 2**: 1 task — T2 → `deep`
- **Wave 3**: 1 task — T3 → `unspecified-high`
- **Wave 4**: 1 task — T4 → `quick`
- **FINAL**: 4 tasks — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

> Implementation + Test = ONE Task. Never separate.
> EVERY task MUST have: Recommended Agent Profile + Parallelization info + QA Scenarios.

- [ ] 1. Define Background Operation Types

  **What to do**:
  - `src/types.rs`에 새로운 타입 추가:
    - `OpKind` enum: `Fetch`, `Pull`, `Push`, `Add`, `Delete`, `Merge` — 작업 종류 식별
    - `OpResult` struct: `kind: OpKind`, `success: bool`, `message: String`, `cmd_detail: String`, `worktree_path: PathBuf`, `display_name: String` — 백그라운드 스레드에서 메인 스레드로 결과 전달
    - `ActiveOp` struct: `kind: OpKind`, `worktree_path: PathBuf`, `display_name: String` — UI에서 활성 작업 표시용 (receiver는 App struct에서 관리)
  - `AppState` enum에서 블로킹 variant 6개 제거: `Fetching`, `Adding`, `Deleting`, `Pulling`, `Pushing`, `Merging`
  - 기존 `ScriptResult` (app.rs:17-20)과 `DeleteResult` (app.rs:22-27)는 `OpResult`로 통합되므로 제거 가능 (또는 Task 2에서 제거)
  - 기존 `ScriptStatus` enum은 유지 (post-add script 용도, 별개의 비동기 시스템)
  - 블로킹 variant 제거로 인한 모든 `match self.state` 컴파일 에러 수정 (해당 arm 제거)

  **Must NOT do**:
  - `git.rs` 변경 금지
  - `AppState::List`, `AddModal`, `ConfirmDelete`, `ConfigModal`, `HelpModal`, `MergeBranchSelect` 변경 금지
  - 새로운 AppState variant 추가 금지 (작업 상태는 App struct 필드로 추적)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: types.rs 타입 정의 변경 + match arm 정리. 로직 없이 구조 변경만.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO (Wave 1 유일 태스크)
  - **Parallel Group**: Wave 1
  - **Blocks**: Task 2
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `src/app.rs:17-20` (`ScriptResult`) — OpResult 설계 시 참고할 기존 결과 타입
  - `src/app.rs:22-27` (`DeleteResult`) — OpResult의 직접적 모델. `worktree_path`, `cmd_detail` 필드 패턴
  - `src/types.rs:81-102` (`AppState` enum) — 제거 대상 variant: line 91-96

  **API/Type References**:
  - `src/types.rs:1` — `use std::path::PathBuf;` 이미 import됨
  - `src/types.rs:159-164` (`ScriptStatus`) — 유지 대상. BackgroundOp과 별개

  **WHY Each Reference Matters**:
  - `DeleteResult`는 OpResult의 정확한 선례 — 필드 구조를 일반화하면 됨
  - `AppState` 블로킹 variant 제거가 non-blocking의 전제조건
  - `ScriptStatus`를 건드리지 않아야 post-add script 기능 유지

  **Acceptance Criteria**:
  - [ ] `cargo build` 에러 없음
  - [ ] `OpKind` enum이 6개 variant 보유
  - [ ] `OpResult` struct에 kind, success, message, cmd_detail, worktree_path, display_name 필드
  - [ ] `grep -c 'AppState::Fetching' src/types.rs` → 0
  - [ ] `grep -c 'AppState::Merging' src/types.rs` → 0

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: New types compile and old AppState variants removed
    Tool: Bash (cargo)
    Preconditions: types.rs changes applied
    Steps:
      1. Run: cargo build 2>&1
      2. Run: grep -c 'OpKind' src/types.rs
      3. Run: grep -c 'OpResult' src/types.rs
      4. Run: grep -c 'AppState::Fetching' src/types.rs
      5. Run: grep -c 'AppState::Merging' src/types.rs
    Expected Result: build succeeds, OpKind>=1, OpResult>=1, Fetching=0, Merging=0
    Failure Indicators: compilation error or non-zero grep for removed variants
    Evidence: .sisyphus/evidence/task-1-types-compile.txt

  Scenario: No match exhaustiveness errors after variant removal
    Tool: Bash (cargo)
    Preconditions: All match arms updated
    Steps:
      1. Run: cargo build 2>&1 | grep -i 'non-exhaustive'
    Expected Result: 0 matches
    Evidence: .sisyphus/evidence/task-1-match-arms.txt
  ```

  **Commit**: YES
  - Message: `refactor(types): add background operation types and remove blocking AppState variants`
  - Files: `src/types.rs`, `src/app.rs` (match arms), `src/ui/main_view.rs` (match arms)
  - Pre-commit: `cargo build`

---

- [ ] 2. Refactor App to Non-Blocking Background Operations

  **What to do**:
  This is the core refactoring task. Transform the event loop and all 6 git operations from blocking to non-blocking.

  **Phase A: App Struct Refactoring**
  - Remove all `is_*` boolean flags: `is_fetching`, `is_adding`, `is_deleting`, `is_pulling`, `is_pushing`, `is_merging` (app.rs:39-45)
  - Remove `delete_receiver: Option<mpsc::Receiver<DeleteResult>>` (app.rs:60)
  - Add new field: `active_op: Option<(OpKind, mpsc::Receiver<OpResult>)>` — tracks the single active background operation
  - Add new field: `active_op_info: Option<ActiveOp>` — stores worktree_path + display_name for UI rendering
  - Keep `script_receiver` and `script_status` as-is (separate system for post-add scripts)
  - Update `App::new()` initializer to remove old fields and initialize new ones

  **Phase B: Event Loop Refactoring** (app.rs:130-185)
  - Remove ALL `if self.is_*` blocks with `continue` statements (lines 135-176)
  - Remove `poll_delete_status()` call (line 180) — replaced by unified polling
  - Add single `self.poll_background_op()` call before `handle_events()`
  - The loop becomes: `draw() → poll_script_status() → poll_background_op() → handle_events()`

  **Phase C: Unified Polling Method**
  - Create `fn poll_background_op(&mut self)` method replacing both `poll_delete_status()` and inline operation execution
  - Use `try_recv()` on `active_op` receiver (non-blocking check)
  - On `Ok(result)`: process result based on `result.kind`:
    - **Delete**: Remove worktree from in-memory list (existing pattern at line 229), do NOT call `refresh_worktrees()`
    - **Fetch**: Call `refresh_worktrees()` (status may have changed)
    - **Pull**: Call `refresh_worktrees()` (branch position changed)
    - **Push**: Call `refresh_worktrees()` (ahead/behind changed)
    - **Add**: Add new worktree to list by calling `refresh_worktrees()`, select newly added worktree, trigger `run_post_add_script()` if applicable, call `copy_configured_files()`
    - **Merge**: Call `refresh_worktrees()` (branch state changed)
  - On `Err(TryRecvError::Empty)`: tick spinner, do nothing
  - On `Err(TryRecvError::Disconnected)`: show error message "Operation failed unexpectedly", clear active_op
  - Show result message (success/error) in `self.message` with verbose `cmd_detail` support
  - Clear `active_op` and `active_op_info` after processing result

  **Phase D: Convert Each Operation to Background Thread**

  All 6 conversions follow the SAME pattern (model after `do_delete_worktree` at app.rs:1053-1103):
  1. Capture worktree data at DISPATCH time (path, display_name, branch, flags)
  2. Create `mpsc::channel()`
  3. Clone required data for `move` closure (`bare_repo_path`, `worktree_path`, etc.)
  4. `std::thread::spawn(move || { ... })` — run the `git::*` function inside
  5. Construct `OpResult` and send via `tx.send()`
  6. Store receiver in `self.active_op`
  7. Store info in `self.active_op_info`

  **D1. Convert `do_fetch`** (app.rs:1244-1277):
  - Capture: `wt.path.clone()`, `wt.display_name()`
  - Thread body: `git::fetch_worktree(&worktree_path)`
  - Build `cmd_detail`: `format!("git -C {} fetch origin", path.display())`

  **D2. Convert `do_pull`** (app.rs:1367-1386):
  - Capture: `wt.path.clone()`, `wt.display_name()`
  - Thread body: `git::pull_worktree(&worktree_path)`
  - Build `cmd_detail`: `format!("git -C {} pull", path.display())`

  **D3. Convert `do_push`** (app.rs:1402-1421):
  - Capture: `wt.path.clone()`, `wt.display_name()`
  - Thread body: `git::push_worktree(&worktree_path)`
  - Build `cmd_detail`: `format!("git -C {} push", path.display())`

  **D4. Convert `do_add_worktree`** (app.rs:887-947):
  - This is the MOST complex conversion due to multi-step pipeline
  - Capture: `branch.clone()`, `bare_repo_path.clone()`, `worktree_path.clone()`, `default_branch`, `config.copy_files.clone()`, source worktree path for copy
  - Thread body: `git::add_worktree()` then file copying (both in thread)
  - IMPORTANT: `run_post_add_script()` must be called in the COMPLETION HANDLER (poll_background_op), not in the thread, because it uses `self.script_status` and `self.script_receiver`
  - `copy_configured_files` logic should be moved INTO the thread (it's pure file I/O, no app state needed)
  - Build `cmd_detail` using `git::build_add_worktree_command_detail()`

  **D5. Simplify `do_delete_worktree`** (app.rs:1053-1103):
  - Already spawns a thread — just migrate to use `OpResult` instead of `DeleteResult`
  - Remove `delete_receiver` usage, use `active_op` instead

  **D6. Convert `do_merge`** (app.rs:1508-1534):
  - Capture: `wt.path.clone()`, `wt.display_name()`, `merge_source_branch.clone()`
  - Thread body: `git::merge_upstream()` or `git::merge_branch()` based on source
  - Build `cmd_detail` accordingly

  **Phase E: Update Trigger Methods**
  - Each trigger method (`fetch_all`, `pull_worktree`, `push_worktree`, `merge_upstream`, `add_worktree`, `delete_selected_worktree`) now:
    1. Checks if `active_op.is_some()` — if so, show "Another operation is in progress" error and return
    2. Validates preconditions (is_bare, clean status, etc.) as before
    3. Captures worktree data, creates channel, spawns thread
    4. Sets `active_op` and `active_op_info`
    5. Shows "Doing X..." info message
    6. Does NOT change `AppState` — stays in `List` (or returns to `List` from `ConfirmDelete`)
  - Remove the old two-step pattern (trigger sets flag, loop calls do_*)

  **Phase F: Event Handling Unblocking**
  - In `handle_events` (app.rs:285-329): remove the match arm that ignores input during operations (lines 315-318)
  - All states now receive normal key handling
  - Remove event draining loops after do_pull, do_push, do_merge, do_add (lines 143-146, 156-158, 164-166, 172-174) — no longer needed since operations don't block
  - Keep event draining after `open_editor()` and `open_post_add_script_editor()` — these still block (external process)
  - In `handle_list_input`: check `active_op_info` for conflict prevention. When user presses operation key, check if the target worktree already has an active operation:
    - `d` (delete): reject if selected worktree has ANY active operation
    - `f`/`p`/`P`/`m` (fetch/pull/push/merge): reject if selected worktree has ANY active operation
    - Navigation (j/k/g/G etc): always allowed
    - Modal opening (a/d/c/?): allowed (except delete on active-op worktree)

  **Must NOT do**:
  - Change `git.rs` in any way
  - Change `event::poll` timeout (100ms)
  - Support multiple concurrent operations — one at a time only for v1
  - Add new UI widgets or panels
  - Split `app.rs` into multiple modules
  - Modify `run_post_add_script` or `poll_script_status` methods (they stay as-is)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Complex refactoring of 1500-line file touching event loop, 6 operation methods, struct fields, and event handling. Requires careful reasoning about thread safety, completion handlers, and state transitions.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO (depends on Task 1)
  - **Parallel Group**: Wave 2 (solo)
  - **Blocks**: Task 3
  - **Blocked By**: Task 1

  **References**:

  **Pattern References** (CRITICAL — follow these exactly):
  - `src/app.rs:978-1030` (`run_post_add_script`) — THE template for spawn+channel+poll. Shows: mpsc::channel creation, data cloning for move closure, thread::spawn, tx.send, receiver storage
  - `src/app.rs:1053-1103` (`do_delete_worktree`) — Shows the thread spawn pattern WITH git operation inside. Note line 1100: `is_deleting = false` after spawn
  - `src/app.rs:216-254` (`poll_delete_status`) — THE template for try_recv polling. Shows: Ok/Empty/Disconnected handling, in-memory list update (line 229), state reset
  - `src/app.rs:130-185` (event loop) — The code to refactor. Shows all `is_*` checks with `continue`
  - `src/app.rs:285-329` (`handle_events`) — The blocking match arm at lines 315-318 to remove
  - `src/app.rs:805-819` (`refresh_worktrees`) — Sync refresh method called after operations

  **Operation Method References** (each method to convert):
  - `src/app.rs:1244-1277` (`do_fetch`) — fully sync, calls `git::fetch_worktree`
  - `src/app.rs:1367-1386` (`do_pull`) — fully sync, calls `git::pull_worktree`
  - `src/app.rs:1402-1421` (`do_push`) — fully sync, calls `git::push_worktree`
  - `src/app.rs:887-947` (`do_add_worktree`) — sync multi-step: add + copy + script + refresh
  - `src/app.rs:1508-1534` (`do_merge`) — sync, calls `git::merge_upstream` or `git::merge_branch`

  **Trigger Method References** (each trigger to update):
  - `src/app.rs:1230-1242` (`fetch_all`) — sets `is_fetching = true`
  - `src/app.rs:1349-1365` (`pull_worktree`) — sets `is_pulling = true`
  - `src/app.rs:1388-1400` (`push_worktree`) — sets `is_pushing = true`
  - `src/app.rs:1423-1440` (`merge_upstream`) — sets `is_merging = true`
  - `src/app.rs:875-885` (`add_worktree`) — sets `is_adding = true`
  - `src/app.rs:1032-1051` (`delete_selected_worktree`) — sets `is_deleting = true`

  **Event Draining References** (to remove):
  - `src/app.rs:143-146` (after do_add)
  - `src/app.rs:156-158` (after do_pull)
  - `src/app.rs:164-166` (after do_push)
  - `src/app.rs:172-174` (after do_merge)

  **Acceptance Criteria**:
  - [ ] `cargo build` 에러 없음
  - [ ] `cargo test` 모두 통과
  - [ ] `grep -c 'is_fetching\|is_adding\|is_deleting\|is_pulling\|is_pushing\|is_merging' src/app.rs` → 0
  - [ ] `grep -c 'Ignore input during operations' src/app.rs` → 0
  - [ ] `grep -c 'poll_background_op' src/app.rs` ≥ 2 (definition + call)
  - [ ] `grep -c 'active_op' src/app.rs` ≥ 5 (field + usage in polling + usage in triggers)
  - [ ] `grep -c 'thread::spawn' src/app.rs` ≥ 6 (one per operation + post_add_script)

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: All operations compile and basic structure correct
    Tool: Bash (cargo)
    Steps:
      1. Run: cargo build 2>&1
      2. Run: cargo test 2>&1
      3. Run: grep -c 'is_fetching' src/app.rs
      4. Run: grep -c 'active_op' src/app.rs
      5. Run: grep -c 'poll_background_op' src/app.rs
    Expected Result: build OK, tests pass, is_fetching=0, active_op>=5, poll_background_op>=2
    Evidence: .sisyphus/evidence/task-2-build-and-structure.txt

  Scenario: Event loop no longer blocks on operations
    Tool: Bash (grep)
    Steps:
      1. Run: grep -n 'Ignore input during operations' src/app.rs
      2. Run: grep -n 'continue' src/app.rs | grep -i 'fetch\|add\|delet\|pull\|push\|merg'
    Expected Result: 0 matches for both (no blocking patterns remain)
    Evidence: .sisyphus/evidence/task-2-no-blocking.txt

  Scenario: Each operation spawns a background thread
    Tool: Bash (grep)
    Steps:
      1. Run: grep -c 'thread::spawn' src/app.rs
      2. Run: grep -B5 'thread::spawn' src/app.rs (verify context shows each operation)
    Expected Result: >= 6 thread::spawn calls, each in a different operation context
    Evidence: .sisyphus/evidence/task-2-thread-spawns.txt
  ```

  **Commit**: YES
  - Message: `refactor(app): convert all git operations to non-blocking background threads`
  - Files: `src/app.rs`
  - Pre-commit: `cargo build && cargo test`

---

- [ ] 3. Update UI Rendering for Background Operations

  **What to do**:
  Update main_view.rs and confirm_modal.rs to correctly display background operation state.

  **3A. main_view.rs: Per-Worktree Operation Indicators**
  - Current code (lines 91-92): `let is_loading = app.is_adding || app.is_deleting || ...` — replace with `let is_loading = app.active_op_info.is_some()`
  - Current code (lines 117-131): cursor hidden during loading for ALL rows — change to show cursor normally. Only the row with an active operation gets special treatment.
  - Current code (lines 133-139): cursor color dimmed during loading — remove this. Keep normal cursor behavior.
  - Current code (lines 169-186): operation spinner shown on `is_selected` row — change to show on the worktree that MATCHES `active_op_info.worktree_path` regardless of selection:
    ```
    // Instead of: if app.is_fetching && is_selected
    // Use: if active_op matches this worktree's path AND kind is Fetch
    let is_op_target = app.active_op_info.as_ref()
        .map(|op| op.worktree_path == wt.path)
        .unwrap_or(false);
    let op_kind = app.active_op_info.as_ref().map(|op| &op.kind);
    ```
  - Show operation-specific spinner text based on `OpKind`:
    - `Fetch` → "⦇ Fetching..." (amber)
    - `Pull` → "⦇ Pulling..." (amber)
    - `Push` → "⦇ Pushing..." (amber)
    - `Add` → "⦇ Adding..." (amber)
    - `Delete` → "⦇ Deleting..." (red)
    - `Merge` → "⦇ Merging..." (amber)
  - Current code (line 239): `let selected = if is_loading { None } else { Some(app.selected_index) }` — change to ALWAYS show selection: `let selected = Some(app.selected_index)`
  - Spinner animation: use existing `SPINNER_FRAMES` and `app.spinner_tick` (already working)

  **3B. main_view.rs: Footer Status Update**
  - Current footer (lines 305-350): already shows `ScriptStatus::Running` and `app.message` — keep this logic
  - Add active operation indicator when no message is displayed:
    - If `active_op_info.is_some()` and no message: show "⦇ {op kind} {display_name}..." in amber
    - This gives feedback when user navigates away from the operation target row

  **3C. confirm_modal.rs: Conflict Prevention Display**
  - When user opens delete confirmation for a worktree that has an active operation:
    - The trigger method (Task 2, Phase E) already rejects this and shows an error
    - No changes needed in confirm_modal.rs itself — the modal simply won't open

  **Must NOT do**:
  - Add new UI widgets, panels, or layout changes
  - Change footer keybinding display
  - Change table column widths or structure
  - Change color theme values

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: UI rendering logic with conditional state display. Not visual design work (no CSS/layout), but requires understanding app state flow.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3 (solo)
  - **Blocks**: Task 4
  - **Blocked By**: Task 2

  **References**:

  **Pattern References**:
  - `src/ui/main_view.rs:14` (`SPINNER_FRAMES`) — existing spinner animation frames
  - `src/ui/main_view.rs:91-92` — current `is_loading` check to replace
  - `src/ui/main_view.rs:117-131` — cursor logic during loading to update
  - `src/ui/main_view.rs:169-186` — operation spinner rendering to refactor
  - `src/ui/main_view.rs:239` — selection disable during loading to remove
  - `src/ui/main_view.rs:305-350` — footer rendering with script status

  **API/Type References**:
  - `src/types.rs` — `OpKind` enum (from Task 1) — needed for kind-specific spinner text
  - `src/types.rs` — `ActiveOp` struct (from Task 1) — provides `worktree_path`, `display_name`, `kind`
  - `src/app.rs` — `active_op_info: Option<ActiveOp>` field (from Task 2) — read by UI

  **Acceptance Criteria**:
  - [ ] `cargo build` 에러 없음
  - [ ] `grep -c 'is_fetching\|is_adding\|is_deleting\|is_pulling\|is_pushing\|is_merging' src/ui/main_view.rs` → 0
  - [ ] `grep -c 'active_op_info' src/ui/main_view.rs` ≥ 2 (is_loading replacement + spinner rendering)
  - [ ] Selection cursor always visible: `grep 'if is_loading { None }' src/ui/main_view.rs` → 0

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: UI compiles and references new types correctly
    Tool: Bash (cargo)
    Steps:
      1. Run: cargo build 2>&1
      2. Run: grep -c 'is_fetching' src/ui/main_view.rs
      3. Run: grep -c 'active_op_info' src/ui/main_view.rs
      4. Run: grep 'if is_loading { None }' src/ui/main_view.rs
    Expected Result: build OK, is_fetching=0, active_op_info>=2, selection always Some
    Evidence: .sisyphus/evidence/task-3-ui-compile.txt

  Scenario: Spinner text matches OpKind variants
    Tool: Bash (grep)
    Steps:
      1. Run: grep -c 'Fetching' src/ui/main_view.rs
      2. Run: grep -c 'Pulling' src/ui/main_view.rs
      3. Run: grep -c 'Pushing' src/ui/main_view.rs
      4. Run: grep -c 'Deleting' src/ui/main_view.rs
      5. Run: grep -c 'Adding' src/ui/main_view.rs
      6. Run: grep -c 'Merging' src/ui/main_view.rs
    Expected Result: each >= 1 (spinner text for each OpKind)
    Evidence: .sisyphus/evidence/task-3-spinner-texts.txt
  ```

  **Commit**: YES
  - Message: `refactor(ui): update rendering for non-blocking operation indicators`
  - Files: `src/ui/main_view.rs`
  - Pre-commit: `cargo build`

---

- [ ] 4. Build, Test, and Manual QA Verification

  **What to do**:
  Comprehensive verification that all changes work correctly end-to-end.

  - Run full build and test suite
  - Run clippy for lint warnings
  - Execute ALL success criteria verification commands
  - Manual QA: launch owt in a real or test bare git repo via tmux, test each operation

  **Must NOT do**:
  - Make any code changes (this is verification only)
  - Skip any verification command

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Run commands and verify output. No code changes.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 4 (solo)
  - **Blocks**: Final Verification Wave
  - **Blocked By**: Task 3

  **References**:
  - Success Criteria section of this plan (verification commands)
  - `.sisyphus/evidence/` directory for evidence files

  **Acceptance Criteria**:
  - [ ] `cargo build` 에러 0개
  - [ ] `cargo test` 모두 통과
  - [ ] `cargo clippy` 새 경고 없음
  - [ ] 모든 Success Criteria verification commands 통과
  - [ ] TUI에서 작업 중 j/k 탐색 동작 확인

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Full build and test pass
    Tool: Bash
    Steps:
      1. Run: cargo build 2>&1
      2. Run: cargo test 2>&1
      3. Run: cargo clippy 2>&1
    Expected Result: 0 errors, all tests pass, no new clippy warnings
    Evidence: .sisyphus/evidence/task-4-build-test.txt

  Scenario: All blocking patterns removed (verification commands)
    Tool: Bash
    Steps:
      1. Run: grep -c 'AppState::Fetching' src/types.rs (expect: 0)
      2. Run: grep -c 'is_fetching' src/app.rs (expect: 0)
      3. Run: grep -c 'Ignore input during operations' src/app.rs (expect: 0)
      4. Run: grep -c 'active_op' src/app.rs (expect: >= 5)
      5. Run: grep -c 'poll_background_op' src/app.rs (expect: >= 2)
    Expected Result: all assertions pass
    Evidence: .sisyphus/evidence/task-4-pattern-verification.txt

  Scenario: TUI remains interactive during git operations
    Tool: interactive_bash (tmux)
    Preconditions: bare repo with >= 2 worktrees available
    Steps:
      1. Launch: owt in the bare repo directory
      2. Navigate to a worktree with j/k
      3. Press 'f' to trigger fetch
      4. IMMEDIATELY press j/k to navigate — verify cursor moves
      5. Wait for fetch to complete — verify success message appears
      6. Press 'q' to quit
    Expected Result: Navigation works during fetch, cursor responds to j/k, message appears after completion
    Failure Indicators: TUI freezes on 'f' press, j/k don't move cursor, no spinner visible
    Evidence: .sisyphus/evidence/task-4-interactive-qa.txt

  Scenario: Conflict prevention works
    Tool: interactive_bash (tmux)
    Preconditions: bare repo with >= 2 worktrees
    Steps:
      1. Launch owt
      2. Press 'f' to start fetch on selected worktree
      3. While fetch is running, press 'p' to try pull
      4. Check that an error message appears ("Another operation is in progress")
    Expected Result: Second operation rejected with informative message
    Evidence: .sisyphus/evidence/task-4-conflict-prevention.txt
  ```

  **Commit**: NO (verification only)

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

> 4 review agents run in PARALLEL. ALL must APPROVE. Rejection → fix → re-run.

- [ ] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (read file, run command). For each "Must NOT Have": search codebase for forbidden patterns — reject with file:line if found. Check evidence files exist in .sisyphus/evidence/. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [ ] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo build 2>&1`, `cargo test 2>&1`, `cargo clippy 2>&1`. Review all changed files for: `as any` equivalent (unwrap chains), empty catches, dead code, unused imports. Check AI slop: excessive comments, over-abstraction, generic names.
  Output: `Build [PASS/FAIL] | Tests [N pass/N fail] | Clippy [N warnings] | Files [N clean/N issues] | VERDICT`

- [ ] F3. **Real Manual QA** — `unspecified-high`
  Start from clean state. Test every operation (delete, fetch, pull, push, add, merge) via tmux:
  1. Launch owt in a test bare repo
  2. Trigger each operation and verify j/k navigation works during execution
  3. Verify spinner displays on correct worktree row
  4. Verify result message appears after completion
  5. Test conflict prevention (try delete during pull on same worktree)
  Save evidence to `.sisyphus/evidence/final-qa/`.
  Output: `Operations [N/N pass] | Non-blocking [N/N] | Conflicts [N tested] | VERDICT`

- [ ] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff (git log/diff). Verify 1:1 — everything in spec was built, nothing beyond spec was built. Check "Must NOT do" compliance: git.rs unchanged, no new UI elements, no module splitting. Flag unaccounted changes.
  Output: `Tasks [N/N compliant] | Must NOT [N/N clean] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

- **1**: `refactor(types): add background operation types and remove blocking AppState variants` — src/types.rs
- **2**: `refactor(app): convert all git operations to non-blocking background threads` — src/app.rs
- **3**: `refactor(ui): update rendering for background operation indicators` — src/ui/main_view.rs, src/ui/confirm_modal.rs
- **4**: Final verification — no commit

---

## Success Criteria

### Verification Commands
```bash
cargo build 2>&1     # Expected: no errors
cargo test 2>&1      # Expected: all tests pass
cargo clippy 2>&1    # Expected: no new warnings

# Verify no blocking states remain
grep -c "AppState::Fetching\|AppState::Adding\|AppState::Pulling\|AppState::Pushing\|AppState::Merging" src/types.rs
# Expected: 0

# Verify no is_* operation flags remain
grep -c "is_fetching\|is_adding\|is_deleting\|is_pulling\|is_pushing\|is_merging" src/app.rs
# Expected: 0

# Verify input is never unconditionally ignored
grep -c "Ignore input during operations" src/app.rs
# Expected: 0
```

### Final Checklist
- [ ] All "Must Have" present
- [ ] All "Must NOT Have" absent
- [ ] All tests pass
- [ ] 6 operations non-blocking verified via tmux QA
