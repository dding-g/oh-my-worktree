use anyhow::{Context, Result};
use std::fs;
use std::path::{Component, Path};

pub(crate) fn copy_configured_files(
    source: &Path,
    destination: &Path,
    files: &[String],
) -> Vec<String> {
    files
        .iter()
        .filter_map(|file| {
            copy_configured_file(source, destination, file)
                .err()
                .map(|error| error.to_string())
        })
        .collect()
}

fn copy_configured_file(source: &Path, destination: &Path, file: &str) -> Result<()> {
    let relative = validate_relative_file(file)?;
    let source_root = source
        .canonicalize()
        .with_context(|| format!("could not resolve source root {}", source.display()))?;
    let destination_root = canonical_directory_without_symlink(destination)?;
    let src = source.join(relative);
    let dst = destination.join(relative);

    if !src.exists() {
        anyhow::bail!("{} source file missing at {}", file, src.display());
    }
    if !src.is_file() {
        anyhow::bail!("{} source is not a file at {}", file, src.display());
    }

    let resolved_src = src
        .canonicalize()
        .with_context(|| format!("could not resolve source file {}", src.display()))?;
    if !resolved_src.starts_with(&source_root) {
        anyhow::bail!("{} source must stay inside {}", file, source.display());
    }

    ensure_destination_parent(&destination_root, relative)?;
    match fs::symlink_metadata(&dst) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            anyhow::bail!(
                "{} destination must not be a symlink at {}",
                file,
                dst.display()
            );
        }
        Ok(metadata) if !metadata.is_file() => {
            anyhow::bail!("{} destination is not a file at {}", file, dst.display());
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error)
                .with_context(|| format!("could not inspect destination {}", dst.display()));
        }
    }

    fs::copy(&resolved_src, &dst).with_context(|| {
        format!(
            "could not copy {} to {}",
            resolved_src.display(),
            dst.display()
        )
    })?;
    Ok(())
}

fn validate_relative_file(file: &str) -> Result<&Path> {
    let path = Path::new(file);
    if path.as_os_str().is_empty()
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        anyhow::bail!("{} must be a relative path inside the worktree", file);
    }
    Ok(path)
}

fn canonical_directory_without_symlink(path: &Path) -> Result<std::path::PathBuf> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("could not inspect destination root {}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        anyhow::bail!(
            "destination root must be a real directory at {}",
            path.display()
        );
    }
    path.canonicalize()
        .with_context(|| format!("could not resolve destination root {}", path.display()))
}

fn ensure_destination_parent(destination_root: &Path, relative: &Path) -> Result<()> {
    let mut current = destination_root.to_path_buf();
    let Some(parent) = relative.parent() else {
        return Ok(());
    };

    for component in parent.components() {
        let Component::Normal(name) = component else {
            anyhow::bail!("destination path must stay inside the worktree");
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                anyhow::bail!(
                    "destination parent must not be a symlink at {}",
                    current.display()
                );
            }
            Ok(metadata) if !metadata.is_dir() => {
                anyhow::bail!(
                    "destination parent is not a directory at {}",
                    current.display()
                );
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current)
                    .with_context(|| format!("could not create {}", current.display()))?;
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("could not inspect {}", current.display()));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::copy_configured_files;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("owt_copy_{name}_{unique}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn copies_nested_relative_file_inside_destination() {
        let base = temp_dir("nested");
        let source = base.join("source");
        let destination = base.join("destination");
        fs::create_dir_all(source.join("config")).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(source.join("config/local.env"), "TOKEN=secret\n").unwrap();

        let warnings =
            copy_configured_files(&source, &destination, &["config/local.env".to_string()]);

        assert!(warnings.is_empty());
        assert_eq!(
            fs::read_to_string(destination.join("config/local.env")).unwrap(),
            "TOKEN=secret\n"
        );
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn rejects_parent_directory_escape_without_writing() {
        let base = temp_dir("parent_escape");
        let source = base.join("source/repo");
        let destination = base.join("destination/worktree");
        let escaped_destination = base.join("destination/outside.env");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(base.join("source/outside.env"), "attacker-controlled\n").unwrap();
        fs::write(&escaped_destination, "keep-me\n").unwrap();

        let warnings =
            copy_configured_files(&source, &destination, &["../outside.env".to_string()]);

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("must be a relative path inside the worktree"));
        assert_eq!(
            fs::read_to_string(escaped_destination).unwrap(),
            "keep-me\n"
        );
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn rejects_absolute_path() {
        let base = temp_dir("absolute");
        let source = base.join("source");
        let destination = base.join("destination");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&destination).unwrap();
        let absolute = base.join("outside.env");
        fs::write(&absolute, "keep-me\n").unwrap();

        let warnings =
            copy_configured_files(&source, &destination, &[absolute.display().to_string()]);

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("must be a relative path inside the worktree"));
        assert_eq!(fs::read_to_string(absolute).unwrap(), "keep-me\n");
        let _ = fs::remove_dir_all(base);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_destination() {
        use std::os::unix::fs::symlink;

        let base = temp_dir("symlink_destination");
        let source = base.join("source");
        let destination = base.join("destination");
        let outside = base.join("outside.env");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(source.join("config.env"), "replacement\n").unwrap();
        fs::write(&outside, "keep-me\n").unwrap();
        symlink(&outside, destination.join("config.env")).unwrap();

        let warnings = copy_configured_files(&source, &destination, &["config.env".to_string()]);

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("destination must not be a symlink"));
        assert_eq!(fs::read_to_string(outside).unwrap(), "keep-me\n");
        let _ = fs::remove_dir_all(base);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_source_symlink_escape() {
        use std::os::unix::fs::symlink;

        let base = temp_dir("source_symlink_escape");
        let source = base.join("source");
        let destination = base.join("destination");
        let outside = base.join("outside.env");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(&outside, "secret\n").unwrap();
        symlink(&outside, source.join("config.env")).unwrap();

        let warnings = copy_configured_files(&source, &destination, &["config.env".to_string()]);

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("source must stay inside"));
        assert!(!destination.join("config.env").exists());
        let _ = fs::remove_dir_all(base);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_destination_parent() {
        use std::os::unix::fs::symlink;

        let base = temp_dir("symlink_destination_parent");
        let source = base.join("source");
        let destination = base.join("destination");
        let outside = base.join("outside");
        fs::create_dir_all(source.join("nested")).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(source.join("nested/config.env"), "replacement\n").unwrap();
        fs::write(outside.join("config.env"), "keep-me\n").unwrap();
        symlink(&outside, destination.join("nested")).unwrap();

        let warnings =
            copy_configured_files(&source, &destination, &["nested/config.env".to_string()]);

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("destination parent must not be a symlink"));
        assert_eq!(
            fs::read_to_string(outside.join("config.env")).unwrap(),
            "keep-me\n"
        );
        let _ = fs::remove_dir_all(base);
    }
}
