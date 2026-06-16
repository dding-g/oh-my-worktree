# owt (oh-my-worktree)

[한국어](./README.ko.md) | [English](./README.md)

Git branch를 단순한 이름이 아니라 작업 공간으로 쓰는 개발자를 위한 빠른 terminal UI입니다.

<img width="786" height="580" alt="Image" src="./owt.png" />

## owt가 필요한 이유

현실의 개발은 한 브랜치에서만 끝나지 않습니다. PR을 리뷰하고, hotfix를 확인하고, 오래 열린 feature를 유지하고, release 전에 main을 다시 테스트해야 합니다. `git switch`만으로 이 흐름을 처리하면 uncommitted file, dependency, editor state, 머릿속 context가 계속 엉킵니다.

Git worktree는 이 문제의 정답에 가깝습니다. `owt`는 그 정답을 매일 쓸 수 있게 만듭니다.

`owt`를 열고 worktree를 고르고, 새 worktree를 만들고, 오래된 worktree를 지우고, fetch/pull/push/merge를 실행하세요. 기존 일반 repository에서도 바로 동작하고, 모든 worktree를 한 폴더에 모으고 싶다면 `.bare` layout도 지원합니다.

## 제공하는 것

- worktree 탐색과 관리를 위한 keyboard-first TUI
- agent와 script를 위한 tab-separated plain CLI surface
- 기존 regular repository 지원
- sibling worktree를 선호하는 사용자를 위한 선택적 `.bare` project layout
- local/remote branch 기반의 빠른 worktree 생성
- dirty state, ahead/behind, last commit, GitHub PR 상태 표시
- fetch, pull, push, upstream merge, branch merge, editor open, terminal open, path copy 내장
- `Enter`로 선택한 worktree에 shell을 이동시키는 shell integration

## 설치

현재 릴리즈(`v0.13.0`)를 Cargo로 설치:

```bash
cargo install --git https://github.com/dding-g/oh-my-worktree --tag v0.13.0 --force
```

Prebuilt 바이너리는 최신 [GitHub Release](https://github.com/dding-g/oh-my-worktree/releases/latest)에 첨부됩니다.

npm wrapper는 npm registry package가 현재 릴리즈로 올라와 있을 때 사용할 수 있습니다.

```bash
npm install -g oh-my-worktree
```

소스에서 빌드하려면:

```bash
git clone https://github.com/dding-g/oh-my-worktree.git
cd oh-my-worktree
cargo build --release
```

## 이미 있는 repository에서 시작

repository를 변환할 필요가 없습니다.

```bash
cd ~/src/my-app
owt
```

regular repository에서 새 worktree를 만들면 기본 위치는 다음과 같습니다.

```text
~/.owt/worktree/<repo-name>/
```

다른 위치를 원하면 `worktree_root`를 설정하세요.

## `.bare` workspace로 시작

모든 worktree를 project folder 안에 나란히 두고 싶다면 `owt clone`을 사용하세요.

```bash
owt clone https://github.com/user/repo.git
cd repo/main
owt
```

생성되는 구조는 다음과 같습니다.

```text
repo/
├── .bare/
├── main/
├── feature-login/
└── hotfix-api/
```

기존 repository를 이 layout으로 옮기고 싶다면 `owt init`이 수동 변환 guide를 출력합니다.

## 매일 쓰는 흐름

```bash
owt
```

TUI에서 다음 키를 사용합니다.

| Key | Action |
| --- | --- |
| `j` / `k` | selection 이동 |
| `Space` | batch action 대상 worktree 선택/해제 |
| `Enter` | 선택한 worktree로 이동 |
| `a` | worktree 추가 |
| `d` | 선택한 worktree 삭제. 체크된 worktree가 있으면 여러 개 삭제 |
| `f` | remote fetch |
| `p` / `P` | 선택한 worktree pull / 현재 worktree push |
| `m` / `M` | upstream merge / 선택 branch merge |
| `o` / `t` | editor / terminal에서 열기 |
| `y` | path 복사 |
| `/` | filter |
| `s` | sort mode 전환 |
| `c` | config 보기 |
| `?` | help |
| `q` | 종료 |

## 목록에서 보이는 정보

| Signal | Meaning |
| --- | --- |
| `✓ clean` | local 변경 없음 |
| `+ staged` | staged 변경 있음 |
| `~ unstaged` | unstaged 변경 있음 |
| `! conflict` | merge conflict |
| `* mixed` | staged와 unstaged 변경이 모두 있음 |
| `↑N` / `↓N` | upstream보다 ahead / behind |
| `PR` | GitHub PR 상태: `open`, `closed`, `merged`, `draft`, 또는 `-` |

`PR` column은 GitHub 전용 best-effort 정보입니다. PR 없음, non-GitHub remote, auth 누락, network 실패, 알 수 없는 상태는 모두 `-`로 표시되어 worktree 목록의 속도와 안정성을 해치지 않습니다.

## Agent용 plain CLI

안정적인 non-TUI 출력이 필요할 때는 단수 명사 command group을 사용합니다. 각 command는 `owt worktree --help`, `owt pr status --help`처럼 자체 help를 제공합니다.

Agent bootstrap asset은 `.agents/`에 버전 관리됩니다. `.agents/prompts/install-owt.md`를 agent에게 전달하고, `.agents/skills/owt-install/SKILL.md`와 `.agents/skills/owt-worktree/SKILL.md`를 설치하거나 따르게 해서 worktree handling이 `owt`를 통하도록 합니다.

```bash
owt worktree list
owt worktree create feature/login --base main
owt worktree delete feature/login --branch --force
owt worktree prune
owt pr status --branch feature/login
owt commit tree -n 12
owt search login
```

`worktree list`와 `search`는 tab-separated record를 출력합니다.

```text
kind<TAB>path<TAB>branch<TAB>status<TAB>last_commit<TAB>ahead<TAB>behind<TAB>pr
```

## Shell integration

shell helper를 설치합니다.

```bash
owt setup
```

shell을 다시 로드하세요. 그 다음부터 TUI에서 `Enter`를 누르면 `owt`가 종료되고 현재 shell이 선택한 worktree로 이동합니다. TUI에서 worktree를 만들 때는 먼저 TUI를 종료한 뒤 일반 terminal에서 worktree 생성, file copy, post-add script, tmux 단계를 실행하고 새 worktree path로 handoff합니다. Shell integration이 없어도 `owt`는 대상 path를 출력하므로 wrapper script나 수동 이동에 사용할 수 있습니다.

## 설정

설정 파일:

```text
~/.config/owt/config.toml
```

예시:

```toml
editor = "code"
terminal = "Ghostty"
worktree_root = "~/.owt/worktree"
copy_files = [".env", ".envrc"]
post_add_script = ".owt/post-add.sh"
tmux_worktree_mode = false
run_post_add_script_in_tmux = false
```

주요 옵션:

| Option | Purpose |
| --- | --- |
| `editor` | `o` 키에서 사용할 command |
| `terminal` | `t` 키에서 사용할 terminal app |
| `worktree_root` | regular repository에서 새 worktree를 만들 root |
| `copy_files` | 새 worktree로 복사할 파일. 파일만 복사하며 복사 문제는 생성 후 warning으로 표시됩니다. |
| `post_add_script` | post-add setup script path. 상대 path는 현재 effective project root 기준입니다. |
| `tmux_worktree_mode` | 새 worktree 생성 후 tmux pane을 열고, `Enter` 시 같은 이름의 pane이 있으면 focus합니다. |
| `run_post_add_script_in_tmux` | worktree 생성 후 post-add script를 detached tmux에서 실행. 이 값은 global config에서만 켤 수 있습니다. |

`.owt/config.toml`의 project config는 `post_add_script` 같은 safe value를 override할 수 있지만 자동 post-add 실행은 켤 수 없습니다. Regular linked worktree는 자기 자신의 project config만 읽고, 부모 directory의 `.owt/config.toml`을 상속하지 않습니다.

## Commands

| Command | Purpose |
| --- | --- |
| `owt [PATH]` | repository 또는 worktree에서 TUI 열기 |
| `owt clone <URL> [PATH]` | `.bare` layout으로 clone하고 첫 worktree 생성 |
| `owt init` | `.bare` layout 수동 변환 guide 출력 |
| `owt setup` | shell integration 설치 |
| `owt worktree list` | worktree를 tab-separated record로 출력 |
| `owt worktree create <BRANCH>` | TUI 없이 worktree 생성. 해당 실행에서 tmux를 열려면 `--tmux=on` 사용 |
| `owt worktree delete <TARGET>` | branch, 이름, path로 worktree 삭제 |
| `owt worktree prune` | stale metadata를 정리하고 `HEAD`에 merge된 non-current clean worktree를 제거 |
| `owt pr status` | `gh`를 통해 GitHub PR 상태 확인 |
| `owt commit tree` | 최근 commit graph 출력 |
| `owt search <QUERY>` | worktree 검색 |
| `owt --version` | version 출력 |

## Requirements

- Git 2.5+
- regular Git repository 또는 `.bare` worktree layout
- 선택: PR 상태 표시용 GitHub CLI `gh`
- 선택: worktree pane mode와 post-add setup script용 tmux

## License

MIT
