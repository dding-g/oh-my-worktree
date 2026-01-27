---
layout: default
title: Shell Integration
parent: Getting Started
nav_order: 3
---

# Shell Integration

Shell integration allows owt to change your working directory when you press `Enter` on a worktree. Without it, owt can only print the path.

## Setup

Run the setup command:

```bash
owt setup
```

This will show instructions for your shell. Add the suggested code to your shell configuration file.

### Bash

Add to `~/.bashrc`:

```bash
owt() {
    local output_file=$(mktemp)
    OWT_OUTPUT_FILE="$output_file" command owt "$@"
    local exit_code=$?

    if [ -f "$output_file" ]; then
        local target=$(cat "$output_file")
        rm -f "$output_file"
        if [ -n "$target" ] && [ -d "$target" ]; then
            cd "$target"
        fi
    fi

    return $exit_code
}
```

### Zsh

Add to `~/.zshrc`:

```zsh
owt() {
    local output_file=$(mktemp)
    OWT_OUTPUT_FILE="$output_file" command owt "$@"
    local exit_code=$?

    if [ -f "$output_file" ]; then
        local target=$(cat "$output_file")
        rm -f "$output_file"
        if [ -n "$target" ] && [ -d "$target" ]; then
            cd "$target"
        fi
    fi

    return $exit_code
}
```

### Fish

Add to `~/.config/fish/functions/owt.fish`:

```fish
function owt
    set -l output_file (mktemp)
    OWT_OUTPUT_FILE="$output_file" command owt $argv
    set -l exit_code $status

    if test -f "$output_file"
        set -l target (cat "$output_file")
        rm -f "$output_file"
        if test -n "$target" -a -d "$target"
            cd "$target"
        end
    end

    return $exit_code
end
```

## Reload Your Shell

After adding the configuration, reload your shell:

```bash
# Bash
source ~/.bashrc

# Zsh
source ~/.zshrc

# Fish
source ~/.config/fish/config.fish
```

## Verify

1. Run `owt` in a worktree directory
2. Select a different worktree
3. Press `Enter`
4. Your shell should now be in the selected worktree directory

## Troubleshooting

### "Tip: Run 'owt setup'..." message

If you see this message when starting owt, shell integration is not set up. Follow the steps above.

### Directory doesn't change

1. Make sure you reloaded your shell configuration
2. Check that the `owt` function is defined: `type owt`
3. The output should show it's a function, not just a path
