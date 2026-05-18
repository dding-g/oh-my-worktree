---
title: owt Documentation / Release Asset 정책
description: README, docs site, npm README, screenshots, favicon/release asset 동기화 계약
ref:
  - README.md
  - README.ko.md
  - npm/README.md
  - docs/index.md
  - docs/index.html
  - docs/_config.yml
  - assets/
  - package.json
  - npm/package.json
  - Cargo.toml
---

# 1. 문서 목적

이 문서는 `owt`의 user-facing documentation과 release/distribution asset이 서로 drift하지 않도록 하는 정책의 정본이다.

```yaml
document_contract:
  source_of_truth_for:
    - documentation_sync_policy
    - package_readme_positioning_policy
    - homepage_asset_policy
    - release_version_file_policy
  not_source_of_truth_for:
    - changelog prose
    - exact visual design of homepage CSS
```

# 2. User-facing 문서 동기화 정책

| 변경 종류 | 함께 확인할 문서 |
|---|---|
| 제품 포지셔닝 | `README.md`, `README.ko.md`, `npm/README.md`, `docs/index.md`, `docs/ssot/01-repository-worktree-policy.md` |
| keybinding | `README.md`, `README.ko.md`, `docs/reference/keybindings.md`, `src/ui/help_modal.rs`, `docs/ssot/03-cli-tui-use-case-contract.md` |
| config option | `README.md`, `README.ko.md`, `docs/reference/configuration.md`, `docs/ssot/02-configuration-trust-boundary-policy.md` |
| shell integration | `docs/getting-started/shell-integration.md`, `docs/usage/navigation.md`, `docs/ssot/05-shell-integration-exit-contract.md` |
| Git operation | `docs/usage/git-operations.md`, `docs/usage/worktrees.md`, `docs/ssot/04-git-operation-safety-policy.md` |

# 3. npm README 정책

`npm/README.md`는 package registry용 축약 문서다. 짧아도 제품 포지셔닝은 root README와 충돌하면 안 된다.

```yaml
npm_readme_policy:
  must_include:
    - regular_repository_support
    - dot_bare_layout_support
    - regular_repo_default_worktree_root
    - basic_keybindings
  may_omit:
    - full_manual_conversion_steps
    - detailed_SSOT_links
```

# 4. Homepage / Asset 정책

| Asset 종류 | Source/archive | Published location | 정책 |
|---|---|---|---|
| README screenshot | `owt.png` | root README | README image와 docs image가 의도적으로 일치해야 한다. |
| docs screenshot | `docs/owt.png`, `docs/assets/owt-darkmode.png` | GitHub Pages | homepage에서 참조되는 asset만 docs 아래에 둔다. |
| dark screenshot archive | `assets/owt-darkmode.png` | archive/source | 원본성 asset은 `assets/` 아래 보관한다. |
| favicon package archive | `assets/owt-favicon-package/` | archive/source | 생성 원본/variant는 archive에 보관한다. |
| published favicon files | `docs/favicon.ico`, `docs/favicon-*.png`, `docs/site.webmanifest` | GitHub Pages root | browser가 접근해야 하는 파일만 docs root에 둔다. |

# 5. Release Version File 정책

Release bump는 다음 파일을 함께 확인한다.

```yaml
release_version_files:
  rust:
    - Cargo.toml
    - Cargo.lock
  npm:
    - package.json
    - npm/package.json
  docs:
    - docs/_config.yml
```

# 6. 검증 규칙

- docs-only 변경은 `git diff --check`를 실행한다.
- GitHub Pages/Jekyll local build는 dependency가 설치된 경우에만 필수로 본다.
- release asset을 옮기면 `docs/index.html`, `docs/site.webmanifest`, favicon references를 함께 확인한다.
- homepage screenshot이나 favicon 변경은 source archive와 published asset의 목적을 commit message나 compound doc에 남긴다.
