use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Helper to create a temporary directory with unique ID
fn temp_dir(name: &str) -> PathBuf {
    let id = std::process::id();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("owt_test_{}_{}_{}", name, id, ts));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

/// Helper to clean up test directory
fn cleanup(path: &PathBuf) {
    let _ = fs::remove_dir_all(path);
}

/// Create a bare repo with initial commit
fn create_test_bare_repo(path: &PathBuf) {
    // Create a temp regular repo first
    let temp = path.parent().unwrap().join("temp_init");
    fs::create_dir_all(&temp).unwrap();

    // Use current_dir instead of -C for init
    let init_output = Command::new("git")
        .current_dir(&temp)
        .args(["init"])
        .output()
        .expect("Failed to run git init");
    assert!(init_output.status.success(),
        "git init failed: {}", String::from_utf8_lossy(&init_output.stderr));

    let config_email = Command::new("git")
        .current_dir(&temp)
        .args(["config", "user.email", "test@test.com"])
        .output()
        .expect("Failed to run git config email");
    assert!(config_email.status.success(),
        "git config email failed: {}", String::from_utf8_lossy(&config_email.stderr));

    let config_name = Command::new("git")
        .current_dir(&temp)
        .args(["config", "user.name", "Test"])
        .output()
        .expect("Failed to run git config name");
    assert!(config_name.status.success(),
        "git config name failed: {}", String::from_utf8_lossy(&config_name.stderr));

    // Create initial commit
    let readme = temp.join("README.md");
    fs::write(&readme, "# Test").unwrap();

    let add_output = Command::new("git")
        .current_dir(&temp)
        .args(["add", "."])
        .output()
        .expect("Failed to run git add");
    assert!(add_output.status.success(),
        "git add failed: {}", String::from_utf8_lossy(&add_output.stderr));

    let commit_output = Command::new("git")
        .current_dir(&temp)
        .args(["commit", "-m", "Initial commit"])
        .output()
        .expect("Failed to run git commit");
    assert!(commit_output.status.success(),
        "git commit failed: {}", String::from_utf8_lossy(&commit_output.stderr));

    // Clone as bare
    let clone_output = Command::new("git")
        .args(["clone", "--bare", &temp.to_string_lossy(), &path.to_string_lossy()])
        .output()
        .expect("Failed to run git clone");
    assert!(clone_output.status.success(),
        "git clone --bare failed: {}", String::from_utf8_lossy(&clone_output.stderr));

    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn test_is_bare_repo() {
    let base = temp_dir("is_bare");
    let bare_path = base.join("test.git");

    create_test_bare_repo(&bare_path);

    // Check is_bare_repo via git command
    let output = Command::new("git")
        .args(["-C", &bare_path.to_string_lossy(), "rev-parse", "--is-bare-repository"])
        .output()
        .unwrap();

    let is_bare = String::from_utf8_lossy(&output.stdout).trim() == "true";
    assert!(is_bare, "Should be a bare repository");

    cleanup(&base);
}

#[test]
fn test_worktree_list() {
    let base = temp_dir("worktree_list");
    let bare_path = base.join("test.git");

    create_test_bare_repo(&bare_path);

    // List worktrees
    let output = Command::new("git")
        .args(["-C", &bare_path.to_string_lossy(), "worktree", "list", "--porcelain"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("worktree "), "Should contain worktree entries");
    assert!(stdout.contains("bare"), "Should show bare indicator");

    cleanup(&base);
}

#[test]
fn test_add_and_remove_worktree() {
    let base = temp_dir("add_remove_wt");
    let bare_path = base.join("test.git");
    let worktree_path = base.join("main");

    create_test_bare_repo(&bare_path);

    // Add worktree
    let add_result = Command::new("git")
        .args([
            "-C", &bare_path.to_string_lossy(),
            "worktree", "add",
            &worktree_path.to_string_lossy(),
            "main"
        ])
        .output()
        .unwrap();

    assert!(add_result.status.success(), "Should add worktree successfully");
    assert!(worktree_path.exists(), "Worktree directory should exist");

    // Verify worktree is listed
    let list_output = Command::new("git")
        .args(["-C", &bare_path.to_string_lossy(), "worktree", "list"])
        .output()
        .unwrap();

    let list_stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(list_stdout.contains("main"), "Should list the new worktree");

    // Remove worktree
    let remove_result = Command::new("git")
        .args([
            "-C", &bare_path.to_string_lossy(),
            "worktree", "remove",
            &worktree_path.to_string_lossy()
        ])
        .output()
        .unwrap();

    assert!(remove_result.status.success(), "Should remove worktree successfully");

    cleanup(&base);
}

#[test]
fn test_git_status_clean() {
    let base = temp_dir("status_clean");
    let bare_path = base.join("test.git");
    let worktree_path = base.join("main");

    create_test_bare_repo(&bare_path);

    // Add worktree
    Command::new("git")
        .args([
            "-C", &bare_path.to_string_lossy(),
            "worktree", "add",
            &worktree_path.to_string_lossy(),
            "main"
        ])
        .output()
        .unwrap();

    // Check status
    let status_output = Command::new("git")
        .args(["-C", &worktree_path.to_string_lossy(), "status", "--porcelain"])
        .output()
        .unwrap();

    let status = String::from_utf8_lossy(&status_output.stdout);
    assert!(status.trim().is_empty(), "Clean worktree should have empty status");

    cleanup(&base);
}

#[test]
fn test_git_status_dirty() {
    let base = temp_dir("status_dirty");
    let bare_path = base.join("test.git");
    let worktree_path = base.join("main");

    create_test_bare_repo(&bare_path);

    // Add worktree
    Command::new("git")
        .args([
            "-C", &bare_path.to_string_lossy(),
            "worktree", "add",
            &worktree_path.to_string_lossy(),
            "main"
        ])
        .output()
        .unwrap();

    // Create a new file (unstaged)
    fs::write(worktree_path.join("new_file.txt"), "test content").unwrap();

    // Check status
    let status_output = Command::new("git")
        .args(["-C", &worktree_path.to_string_lossy(), "status", "--porcelain"])
        .output()
        .unwrap();

    let status = String::from_utf8_lossy(&status_output.stdout);
    assert!(!status.trim().is_empty(), "Dirty worktree should have non-empty status");
    assert!(status.contains("??"), "Should show untracked file");

    cleanup(&base);
}

#[test]
fn test_extract_repo_name() {
    // Test various URL formats
    let cases = vec![
        ("https://github.com/user/repo.git", "repo"),
        ("git@github.com:user/repo.git", "repo"),
        ("https://github.com/user/repo", "repo"),
        ("repo.git", "repo"),
        ("/path/to/repo.git", "repo"),
    ];

    for (url, expected) in cases {
        let url = url.trim_end_matches('/');
        let name = url
            .rsplit('/')
            .next()
            .or_else(|| url.rsplit(':').next())
            .unwrap_or(url)
            .trim_end_matches(".git");

        assert_eq!(name, expected, "Failed for URL: {}", url);
    }
}
