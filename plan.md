# OWT Improvement Plan

## 개요
oh-my-worktree (owt) TUI 도구의 UX 개선 및 기능 추가 계획

**현재 버전: v0.3.4**

## Checklist

### 1. Enter 키로 워크트리 이동 기능 수정 (Shell Integration)
- [x] **분석 완료**: Enter 키 기능은 이미 구현됨
- [x] **문제 확인**: Shell integration (`owt setup`)이 필요함
- [x] **개선 1**: TUI에서 shell integration 미설정 시 안내 메시지 표시
- [x] **개선 2**: Enter 키 동작 수정 - shell integration 없어도 작동
- [x] **버그 수정**: shell integration 체크가 Enter를 차단하던 문제 해결
- [x] **버그 수정**: 필터 모드에서 Enter 누르면 worktree에 진입하도록 수정
- [x] **버그 수정**: 초기 선택이 bare repo가 아닌 첫 번째 non-bare worktree로 설정
- [x] **UX 개선**: shell integration 미설정 시 시작 메시지 표시
- [x] **배포 수정**: npm 바이너리 업데이트 (실제로 사용되는 바이너리)
- [x] **코드 검증**: Enter 키 처리 로직 완료
- [x] **디버그**: `owt test-cd` 명령어 추가 (shell integration 테스트용)
- [x] **UX**: Enter 시 `→ /path` 피드백 메시지 표시

### 2. 검색/필터 기능 추가
- [x] `/` 키로 검색 모드 진입
- [x] 실시간 필터링으로 worktree 목록 필터
- [x] `Esc`로 검색 취소, `Enter`로 워크트리 진입
- [x] 필터링 시 매치되지 않는 항목 흐리게 표시

### 3. 현재 워크트리로 빠르게 이동
- [x] `g` 키로 현재 워크트리(●)로 포커스 이동
- [x] 현재 워크트리가 없으면 메시지 표시

### 4. 워크트리 정렬 옵션
- [x] `s` 키로 정렬 모드 순환 (이름순/최근 수정순/상태순)
- [x] 현재 정렬 상태를 footer에 표시

### 5. 키보드 네비게이션 개선
- [x] `G` / `End` - 목록 맨 아래로 이동
- [x] `gg` / `Home` - 목록 맨 위로 이동
- [x] `Ctrl+d` / `Ctrl+u` - 반 페이지 스크롤

### 6. 상태 표시 개선
- [x] Worktree 상태에 더 자세한 정보 (ahead/behind commits)
- [x] Git fetch 상태 아이콘 표시 (↑↓ 화살표)

### 7. 추가 UX 개선
- [x] Worktree 경로 복사 (`y` 키로 경로를 클립보드에 복사)

## Iteration 6 - 핵심 문제 발견 및 해결

### 문제 원인
**`command owt`가 잘못된 바이너리를 실행하고 있었음!**

```
/opt/homebrew/bin/owt (npm 버전, 1.1M) - 실제로 실행됨
/Users/ddingg/.cargo/bin/owt (cargo 버전, 1.3M) - 수정된 버전
```

shell function의 `command owt`가 PATH에서 먼저 나오는 `/opt/homebrew/bin/owt`를 실행

### 해결 방법
npm 바이너리를 cargo로 빌드한 최신 바이너리로 교체:
```bash
cp /Users/ddingg/.cargo/bin/owt /opt/homebrew/lib/node_modules/oh-my-worktree/bin/owt
```

## 테스트 방법

### 필수 사전 조건
1. **새 터미널 열기** (매우 중요!)
2. `owt --version` 실행하여 `v0.3.4` 확인

### 테스트 절차
```bash
# 1. 새 터미널 열기 (필수!)

# 2. 버전 확인
owt --version
# 출력: owt v0.3.4

# 3. 디버그 모드로 테스트
owt_debug

# 4. TUI에서:
#    - j/k로 worktree 선택 (bare repo 제외)
#    - Enter 키 누르기 (q가 아님!)
#
# 5. DEBUG 출력 확인:
#    DEBUG: tmpfile=/tmp/xxx
#    DEBUG: owt exit code=0
#    DEBUG: tmpfile content='/path/to/worktree'
#    DEBUG: changing directory to '/path/to/worktree'
#    DEBUG: now in /path/to/worktree

# 6. pwd로 확인
pwd
```

### 중요 사항
| 키 | 동작 |
|---|---|
| **Enter** | 선택한 worktree로 cd (디렉토리 변경) |
| **q** | 단순 종료 (디렉토리 변경 없음) |

- **반드시 새 터미널**에서 테스트 (shell function 로드 필요)
- **Enter 키**를 눌러야 cd가 됨 (q는 종료만)

## 기능 요약

### 키바인딩
| 키 | 동작 |
|---|---|
| Enter | 선택한 worktree로 이동 (cd) |
| j/k, ↑/↓ | 위/아래 이동 |
| g | 현재 worktree로 이동 |
| gg, Home | 맨 위로 |
| G, End | 맨 아래로 |
| Ctrl+d/u | 반 페이지 스크롤 |
| / | 검색 모드 |
| s | 정렬 순환 |
| y | 경로 복사 |
| a | worktree 추가 |
| d | worktree 삭제 |
| o | 에디터에서 열기 |
| t | 터미널에서 열기 |
| f | fetch |
| r | 새로고침 |
| c | 설정 |
| ? | 도움말 |
| q | 종료 |
