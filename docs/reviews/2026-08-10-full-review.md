---
title: owt Full Review (2026-08-10)
date: 2026-08-10
scope: code, tests, distribution, documentation
release: 0.14.4
---

# owt Full Review

## 결론

**Recommendation: APPROVE FOR PR**

초기 review에서 발견한 P1 2건, P2 6건, P3 3건을 현재 working tree에서 모두 해소했다. README와 설치·keybinding mirror 문서도 실제 최신 release `v0.14.4` 기준으로 동기화했다. 이 개선은 아직 commit/push/release되지 않았으므로 공개 `v0.14.4` binary에는 포함되지 않는다.

| Priority | Count | 의미 |
|---|---:|---|
| P1 | 2 / 2 resolved | 보안/지원 플랫폼 결함 해소 |
| P2 | 6 / 6 resolved | correctness/safety 결함 해소 |
| P3 | 3 / 3 resolved | 품질 게이트와 유지보수 개선 완료 |

## P1

### P1-1. [Resolved] Project `copy_files`가 worktree 밖으로 탈출해 파일을 덮어쓸 수 있음

- Evidence: project config의 `copy_files`를 그대로 병합한다([`src/config.rs`](../../src/config.rs#L58)). 생성 시 각 값을 검증하지 않고 `source.join(file)`과 `destination.join(file)`에 사용한 뒤 `fs::copy`를 실행한다([`src/main.rs`](../../src/main.rs#L795)).
- Impact: repository가 제공한 `.owt/config.toml`에 `../` component가 있으면 destination이 새 worktree root 밖으로 벗어난다. 기존 파일도 overwrite할 수 있어, script auto-run을 막은 config trust boundary와 별개로 파일 손상이 가능하다.
- Fix: absolute path, `ParentDir`, `RootDir`, platform prefix를 거부한다. canonicalized source가 source root 아래인지 확인하고, destination parent를 안전하게 만든 뒤 worktree root 아래인지 다시 확인한다. symlink destination도 거부하고 regression test를 추가한다.
- Resolution: production 경로를 `src/copy_files.rs`로 통합하고 relative normal component, canonical source confinement, destination root/parent/final symlink 검사를 적용했다. parent/absolute/source symlink/destination symlink 회귀 테스트가 실제 production helper를 호출한다.

### P1-2. [Resolved] Windows artifact를 배포하지만 기본 TUI는 `/dev/tty` 때문에 실행되지 않음

- Evidence: `run_tui`가 platform 분기 없이 `/dev/tty`를 연다([`src/main.rs`](../../src/main.rs#L191)). 반면 npm package는 `win32`를 지원 OS로 선언하고([`npm/package.json`](../../npm/package.json#L36)), release workflow도 Windows binary를 게시한다([`.github/workflows/release.yml`](../../.github/workflows/release.yml#L30)).
- Impact: 설치와 `owt --version`은 성공할 수 있지만 핵심 제품인 기본 `owt` TUI는 Windows에서 시작하지 못한다.
- Fix: Unix `/dev/tty`와 Windows console handle을 platform adapter로 분리하고 Windows smoke test를 추가한다. 구현 전까지는 Windows asset/package 지원 표기를 제거하는 편이 안전하다.
- Resolution: Unix는 `/dev/tty`, non-Unix는 console stdout을 사용하는 writer adapter로 분리했다. `x86_64-pc-windows-msvc` target strict Clippy compile을 통과했다. 실제 Windows console runtime smoke는 release workflow 환경에서 추가 확인이 필요하다.

## P2

### P2-1. [Resolved] Worktree status 조회 실패를 `clean`으로 처리함

- Evidence: `get_status` 실패를 두 곳에서 `WorktreeStatus::Clean`으로 바꾼다([`src/git.rs`](../../src/git.rs#L125)). prune과 delete safety는 이 값을 신뢰한다.
- Impact: permission, filesystem, broken worktree, transient Git 오류가 안전한 상태로 오인된다. 특히 completed-PR prune의 destructive 판단이 fail-open이 된다.
- Fix: `Unknown/Error` 상태를 추가하거나 목록 enrichment 오류를 별도 보존한다. delete/prune 직전에 live status를 다시 확인하고, 확인 실패 시 항상 보존한다. submodule fallback의 `remove_dir_all` 전에도 동일한 검증이 필요하다.
- Resolution: `WorktreeStatus::Unknown`을 추가했고 delete boundary에서 live status를 재조회한다. status 실패, dirty/untracked, prune unknown 회귀 테스트로 fail-closed 동작을 고정했다.

### P2-2. [Resolved] Base branch fetch 실패를 무시해 stale commit에서 branch를 만들 수 있음

- Evidence: TUI post-create와 plain CLI create 모두 `fetch_remote_branch` 결과를 버린다([`src/main.rs`](../../src/main.rs#L304), [`src/main.rs`](../../src/main.rs#L396)). test-only add flow도 동일하다.
- Impact: network/auth/fetch 오류가 있어도 기존 `origin/<base>` ref가 있으면 새 branch가 오래된 commit에서 조용히 생성된다.
- Fix: fetch 실패를 기본적으로 create 실패로 전파하거나, 명시적인 `--offline`에서만 stale ref 사용을 허용한다. 최소한 stderr warning과 생성 결과 metadata에 base SHA를 노출한다.
- Resolution: TUI post-create와 plain CLI create 모두 fetch 오류를 전파하고 생성 전에 중단한다. stale remote를 가진 실패 origin 회귀 테스트를 추가했다.

### P2-3. [Resolved] `recent` sort가 시간을 문자열로 비교함

- Evidence: `last_commit_time`은 `%ar` 상대시간 문자열이고, sort는 문자열 역순 비교를 한다([`src/app.rs`](../../src/app.rs#L1114)).
- Impact: `2 weeks ago`, `9 hours ago`, `yesterday` 같은 값이 실제 시간순으로 정렬되지 않는다.
- Fix: Git에서 epoch timestamp를 함께 받아 numeric sort key로 저장하고, `%ar`는 display 전용으로 유지한다.
- Resolution: `%ar`와 `%ct`를 한 번에 읽고 `last_commit_timestamp` 숫자 key로 정렬한다.

### P2-4. [Resolved] 초기화 오류 시 terminal raw/alternate-screen 복구가 보장되지 않음

- Evidence: raw mode와 alternate screen 진입 뒤 `Terminal::new`와 `App::new`에 `?`를 사용하며, cleanup은 그 뒤에만 있다([`src/main.rs`](../../src/main.rs#L206)).
- Impact: 초기 worktree scan 같은 startup 오류가 발생하면 사용자의 terminal이 raw mode 또는 alternate screen에 남을 수 있다.
- Fix: RAII terminal guard를 사용하고 모든 return/panic 경로에서 복구한다. startup fault-injection test를 추가한다.
- Resolution: raw/alternate-screen lifecycle을 `TerminalModeGuard`로 감싸 초기화와 runtime error return에서 복구되게 했다.

### P2-5. [Resolved] 문서의 단일 `g` 동작이 실제로는 즉시 실행되지 않음

- Evidence: 첫 `g`는 `last_key`만 설정하고, 다음 non-`g` key가 들어와야 current worktree jump가 실행된다([`src/app.rs`](../../src/app.rs#L453)). 다음 key의 원래 action도 소비된다.
- Impact: help와 keybinding 문서의 “`g`: Jump to current worktree”가 실제 UX와 다르다.
- Fix: `gg`와 single `g`를 동시에 유지하려면 짧은 timeout/state machine을 사용하거나, current jump를 충돌 없는 별도 key로 옮긴다.
- Resolution: 300ms sequence timeout을 적용해 `gg`는 top, single `g`는 timeout 후 current worktree로 이동하며 다음 non-`g` action도 보존한다.

### P2-6. [Resolved] `copy_files` 테스트가 실제 runtime 구현이 아닌 test-only 복제본을 검증함

- Evidence: production TUI는 `ExitAction::CreateWorktree`를 queue하고 실제 복사는 `main.rs`에서 수행한다([`src/app.rs`](../../src/app.rs#L1194), [`src/main.rs`](../../src/main.rs#L304)). 반면 app tests는 `#[cfg(test)]`로 남은 별도 add/copy 구현을 호출한다([`src/app.rs`](../../src/app.rs#L1238), [`src/app.rs`](../../src/app.rs#L2113)).
- Impact: production copy path가 바뀌어도 test가 통과할 수 있으며, P1-1 path confinement 회귀를 현재 test suite가 막지 못한다.
- Fix: copy validation/execution을 하나의 production module로 추출하고 unit/integration test가 같은 함수를 호출하게 한다. test-only mirror 구현은 제거한다.
- Resolution: test-only add/copy 구현을 제거하고 TUI/CLI runtime과 tests가 `src/copy_files.rs`를 공유한다.

## P3

### P3-1. [Resolved] Strict clippy gate 실패

`cargo clippy --all-targets --all-features -- -D warnings`가 실패했다. RTK 요약과 raw diagnostic 집계 방식에 따라 개수 표시는 달랐지만, 주요 항목은 dead fields, `&PathBuf` API, duplicate condition, type complexity, nested `format!`, collapsible `if`다. CI에 fmt/test/clippy를 고정하고 warning budget을 0으로 줄이는 것이 좋다.

불필요 field/helper와 lint를 정리했고 strict Clippy를 통과한다. 새 CI workflow가 version sync, fmt, strict Clippy, test를 PR/main push에 고정한다.

### P3-2. [Resolved] Footer에 도달 불가능한 duplicate active-operation branch가 있음

[`src/ui/main_view.rs`](../../src/ui/main_view.rs#L468)에서 `active_op_info`를 이미 처리한 뒤 동일 조건을 다시 검사한다. 기능 영향은 작지만 clippy 실패와 UI 수정 혼동을 만든다.

중복 branch를 제거했다.

### P3-3. [Resolved] Release version이 user-facing 문서에 자동 전파되지 않음

`0.14.4` release 이후 README, docs homepage, install guide, agent install skill이 `0.13.0`에 남아 있었다. 이번 review에서 수정했지만, release workflow가 docs version을 검증하지 않으므로 재발 가능하다.

`scripts/sync-release-version.js`가 Cargo/package/lock/docs mirror를 동기화·검증한다. release-it bump와 PR/release workflow에서 자동 실행하며 tag version 불일치도 차단한다.

## 기능 개선 제안

1. **Fast first frame + progressive metadata (후속 범위)**: [`worktree-loading-performance-plan.md`](../solutions/performance/worktree-loading-performance-plan.md)의 phase 1을 구현해 첫 화면은 `git worktree list --porcelain`만 기다리고 status/ahead/commit을 bounded background worker로 채운다.
2. **Safe cleanup UX (부분 완료)**: destructive action 직전 live status recheck는 완료했다. TUI `x`와 plain CLI prune 의미 통합, `--non-interactive`, `--yes`, `--format tsv|json`은 후속 범위다.
3. **Typed config + diagnostics**: hand-written TOML parser를 `serde`/`toml` 기반 schema로 교체하고 `owt config check` 또는 `owt doctor`로 unknown key, invalid path, trust-boundary 설정을 설명한다.
4. **Cross-platform terminal adapter (부분 완료)**: TUI writer와 cleanup guard는 platform 분리했다. clipboard/editor/terminal/shell integration capability 통합은 후속 범위다.
5. **Release SSOT automation (완료)**: Cargo/package version을 기준으로 README/docs/agent skill의 pinned version을 검증하는 script와 CI check를 추가했다.

## README/문서 최신화

- `README.md`, `README.ko.md`, `npm/README.md`: verified current release `v0.14.4`, batch selection, refresh/prune, verbose mode, selected-worktree push, plain CLI output/command surface 동기화.
- `docs/reference/keybindings.md`: `Space`, `x`, `v`, batch pull/delete, force-delete toggle 반영.
- `docs/index.md`, `docs/index.html`, `docs/getting-started/installation.md`, `docs/_config.yml`, `.agents/skills/owt-install/SKILL.md`: release/install version `0.14.4`로 동기화.

## 실제 배포 검증

- GitHub Release `v0.14.4`는 2026-07-03 09:45:25 UTC에 non-draft/non-prerelease로 게시됐고 release workflow는 성공했다.
- macOS arm64/x64, Linux arm64/x64, Windows x64 asset 5개를 모두 실제 다운로드해 Mach-O/ELF/PE binary 형식을 확인했다. 현재 환경의 `owt-darwin-arm64`는 `owt v0.14.4`로 실행됐다.
- npm registry의 `latest`와 package version은 모두 `0.14.4`다. 임시 prefix에 `oh-my-worktree@0.14.4`를 설치했으며 postinstall이 GitHub asset을 내려받은 뒤 wrapper가 `owt v0.14.4`로 실행됐다.
- 공개 GitHub README, npm `0.14.4` package README, GitHub Pages는 아직 `v0.13.0` 설치 문구를 노출한다. 현재 local 문서 수정은 commit/push 전이므로 공개되지 않았다. GitHub README/Pages는 main 반영 후 갱신되지만, 이미 게시된 npm `0.14.4` tarball README는 덮어쓸 수 없으므로 다음 package version에서 반영해야 한다.
- 현재 이 Mac의 global `/opt/homebrew/bin/owt`는 `v0.14.3`이다. 배포 검증은 global install을 변경하지 않고 임시 directory에서 수행했다.

## 검증 결과

| Check | Result |
|---|---|
| Remote Git tag | `v0.14.4` 확인 |
| npm latest | `0.14.4` 확인 |
| GitHub release assets | 5개 다운로드/형식 확인, native arm64 실행 성공 |
| npm isolated install | `oh-my-worktree@0.14.4` 설치 및 wrapper 실행 성공 |
| public README/Pages | 아직 `v0.13.0` 표기; local fix 미배포 |
| `cargo test` | 102 passed |
| `cargo fmt -- --check` | passed |
| strict clippy | native + `x86_64-pc-windows-msvc` target passed |
| release version sync | `v0.14.4` tag/package/docs mirror check passed |
| npm package dry-run | `oh-my-worktree@0.14.4`, expected 4 files 확인 |
| dependency audit | `cargo-audit` 미설치로 미실행 |

## 완료된 수정 순서

1. `copy_files` path confinement와 regression tests.
2. Windows 지원 결정을 확정하고 TUI adapter 구현 또는 배포 표기 축소.
3. status fail-closed + destructive live recheck.
4. fetch failure propagation, numeric recent sort, terminal RAII, `g` key state machine, production-path copy tests.
5. clippy cleanup과 release-doc CI.
