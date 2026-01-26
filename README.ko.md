# owt (oh-my-worktree)

Git worktree를 쉽게 관리할 수 있는 TUI 도구입니다.

<img width="786" height="580" alt="Image" src="https://github.com/user-attachments/assets/929a7bf2-cd66-4a87-a73e-8b9567cb0a08" />

## Git Worktree란?

Git worktree를 사용하면 하나의 저장소에서 여러 브랜치를 동시에 체크아웃할 수 있습니다. stash나 branch 전환 없이 여러 작업을 병렬로 진행할 수 있습니다.

```
project.git/              # bare repository
├── main/                 # main 브랜치 worktree
├── feature-auth/         # feature 브랜치 worktree
└── hotfix-payment/       # hotfix 브랜치 worktree
```

**owt**는 이 워크플로우를 간단한 TUI로 관리할 수 있게 해줍니다.

## 설치

### npm (권장)

```bash
npm install -g oh-my-worktree
```

npx로 설치 없이 바로 실행:

```bash
npx oh-my-worktree
```

### Cargo

```bash
cargo install --git https://github.com/mattew8/oh-my-worktree
```

소스에서 빌드:

```bash
git clone https://github.com/mattew8/oh-my-worktree.git
cd oh-my-worktree
cargo build --release
# 바이너리: ./target/release/owt
```

## 시작하기

### 새 프로젝트

```bash
# bare repo로 클론 + 첫 번째 worktree 자동 생성
owt clone https://github.com/user/repo.git

# TUI 실행
cd repo.git
owt
```

### 기존 프로젝트 변환

```bash
owt init
```

기존 일반 저장소를 bare + worktree 구조로 변환하는 가이드를 보여줍니다.

수동 변환:

```bash
mv .git ../myproject.git
cd ../myproject.git
git config --bool core.bare true
git worktree add ../myproject/main main
owt
```

## 사용법

```bash
# bare repo 또는 worktree 내에서 실행
owt

# 경로 지정
owt /path/to/repo.git
```

### 키 바인딩

| 키 | 동작 |
|---|------|
| `j` / `↓` | 아래로 이동 |
| `k` / `↑` | 위로 이동 |
| `Enter` | 선택한 worktree로 이동 |
| `a` | 새 worktree 추가 |
| `d` | worktree 삭제 |
| `o` | 에디터에서 열기 |
| `t` | 터미널에서 열기 |
| `f` | 모든 remote fetch |
| `r` | 목록 새로고침 |
| `q` | 종료 |

### 상태 아이콘

| 아이콘 | 의미 |
|-------|------|
| `✓` | Clean |
| `+` | Staged 변경사항 |
| `~` | Unstaged 변경사항 |
| `!` | 충돌 |
| `*` | Staged + Unstaged |

## 설정

### 환경 변수

| 변수 | 설명 | 기본값 |
|-----|------|-------|
| `EDITOR` | worktree를 열 에디터 | `vim` |
| `TERMINAL` | 터미널 앱 (macOS) | `Terminal` |

```bash
export EDITOR=code
export TERMINAL=Ghostty
```

## 요구사항

- Git 2.5+ (worktree 지원)
- Bare repository

## 라이선스

MIT
