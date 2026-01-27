---
layout: default
title: Navigation
parent: Usage
nav_order: 1
---

# Navigation

owt uses vim-style keybindings for efficient navigation.

## Basic Movement

| Key | Action |
|:----|:-------|
| `j` or `↓` | Move down |
| `k` or `↑` | Move up |
| `gg` or `Home` | Go to top |
| `G` or `End` | Go to bottom |
| `Ctrl+d` | Half page down |
| `Ctrl+u` | Half page up |

## Searching

Press `/` to enter search mode:

1. Type to filter worktrees by name or branch
2. The list filters as you type
3. Press `Enter` to enter the selected worktree
4. Press `Esc` to cancel search

## Jumping to Current Worktree

Press `g` (single press) to jump back to the worktree where you launched owt.

This is useful when you've scrolled through the list and want to return to your starting point.

## Sorting

Press `s` to cycle through sort modes:

1. **Name** - Alphabetical by worktree folder name
2. **Recent** - Most recently committed first
3. **Status** - Dirty worktrees first (conflicts, then unstaged, then staged, then clean)

The current sort mode is shown in the status bar.

## Entering a Worktree

Press `Enter` on any worktree to:

1. Exit owt
2. Change your shell's directory to that worktree

{: .note }
This requires [shell integration](/oh-my-worktree/getting-started/shell-integration) to be set up.

## Status Indicators

Each worktree shows a status icon:

| Icon | Meaning |
|:-----|:--------|
| `✓` | Clean - no changes |
| `+` | Staged changes |
| `~` | Unstaged changes |
| `!` | Merge conflicts |
| `*` | Both staged and unstaged |

## Ahead/Behind Indicators

If a worktree is ahead or behind its remote:

| Indicator | Meaning |
|:----------|:--------|
| `↑3` | 3 commits ahead of remote |
| `↓2` | 2 commits behind remote |
| `↑3↓2` | 3 ahead, 2 behind |
