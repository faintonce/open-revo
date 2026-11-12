use std::path::{Path, PathBuf};

pub fn validate_file_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.ends_with(['.', ' '])
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
    {
        return Err("Invalid file name".into());
    }
    let stem = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end()
        .to_ascii_uppercase();
    if matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ["COM", "LPT"].iter().any(|prefix| {
        stem.strip_prefix(prefix).is_some_and(|n| {
            matches!(
                n,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
    }) {
        return Err("Reserved Windows device name".into());
    }
    Ok(())
}



/// Resolve an existing or new path without following links outside its owning directory.
pub fn resolve_within(base: &Path, relative: &str) -> Result<PathBuf, String> {
    let normalized = relative.replace('\\', "/");
    if normalized.starts_with('/') || normalized.is_empty() {
        return Err("Path must be relative".into());
    }
    let mut has_name = false;
    for part in normalized
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
    {
        validate_file_name(part)?;
        has_name = true;
    }
    if !has_name {
        return Err("Path must name a file or subdirectory".into());
    }

    let root = base
        .canonicalize()
        .map_err(|e| format!("Invalid base directory: {e}"))?;
    let target = base.join(&normalized);
    let mut ancestor = target.as_path();
    loop {
        match std::fs::symlink_metadata(ancestor) {
            Ok(_) => break,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                ancestor = ancestor.parent().ok_or("Invalid path")?;
            }
            Err(e) => return Err(format!("Cannot inspect path: {e}")),
        }
    }
    let resolved = ancestor
        .canonicalize()
        .map_err(|e| format!("Invalid path: {e}"))?;
    if !resolved.starts_with(&root) || (ancestor == target && resolved == root) {
        return Err("Path escapes its owning directory".into());
    }
    Ok(target)
}

/// Drops the `\\?\` prefix that `canonicalize` adds on Windows, so a stored data path stays a
/// normal `D:\...` or `\\server\share\...` path for the UI, Explorer and the registry.
/// Any other path is returned unchanged.
pub fn without_verbatim_prefix(path: &Path) -> PathBuf {
    use std::path::{Component, Prefix};
    let mut components = path.components();
    let mut plain = match components.next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::VerbatimDisk(drive) => PathBuf::from(format!("{}:\\", drive as char)),
            Prefix::VerbatimUNC(server, share) => {
                let mut unc = std::ffi::OsString::from(r"\\");
                unc.push(server);
                unc.push(r"\");
                unc.push(share);
                unc.push(r"\");
                PathBuf::from(unc)
            }
            _ => return path.to_path_buf(),
        },
        _ => return path.to_path_buf(),
    };
    for component in components {
        if !matches!(component, Component::RootDir) {
            plain.push(component.as_os_str());
        }
    }
    plain
}

pub fn validate_migration_dirs(source: &Path, destination: &Path) -> Result<(), String> {
    if source.starts_with(destination) || destination.starts_with(source) {
        return Err("Source and destination data directories must not overlap".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regression_rejects_windows_file_aliases() {
        for name in [
            "", ".", "..", "file.", "file ", "C:foo", "a/b", "a\\b", "a\0b", "CON", "nul.json",
            "COM1.txt", "LPT²",
        ] {
            assert!(validate_file_name(name).is_err(), "{name:?}");
        }
        assert!(validate_file_name("自定义方案.json").is_ok());
    }

    #[test]
    fn regression_contains_new_paths_and_rejects_traversal() {
        let base = std::env::temp_dir().join(format!("openrevo-path-test-{}", std::process::id()));
        std::fs::create_dir(&base).unwrap();
        assert_eq!(
            resolve_within(&base, "curves/new.json").unwrap(),
            base.join("curves/new.json")
        );
        assert!(resolve_within(&base, "./config.json").is_ok());
        for path in [
            "../secret",
            "..\\secret",
            "a/../../secret",
            "/absolute",
            "\\server\\share",
            "C:/outside",
            "a/file:stream",
            "a/.. /secret",
            ".",
        ] {
            assert!(resolve_within(&base, path).is_err(), "{path:?}");
        }
        std::fs::remove_dir(base).unwrap();
    }

    #[test]
    fn regression_rejects_nested_data_migrations() {
        let base = std::env::temp_dir().join("openrevo-migration");
        assert!(validate_migration_dirs(&base, &base.join("nested")).is_err());
        assert!(validate_migration_dirs(&base.join("nested"), &base).is_err());
        assert!(validate_migration_dirs(&base, &base.with_file_name("openrevo-other")).is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn regression_stored_data_paths_drop_the_verbatim_prefix() {
        for (input, expected) in [
            (r"\\?\D:\OpenRevo Data\用户", r"D:\OpenRevo Data\用户"),
            (r"\\?\D:\", r"D:\"),
            (r"\\?\UNC\server\share\OpenRevo", r"\\server\share\OpenRevo"),
            (r"D:\Already\Plain", r"D:\Already\Plain"),
            (r"\\server\share\Plain", r"\\server\share\Plain"),
            (r"\\?\Volume{01234567-89ab-cdef-0123-456789abcdef}\data", r"\\?\Volume{01234567-89ab-cdef-0123-456789abcdef}\data"),
        ] {
            assert_eq!(without_verbatim_prefix(Path::new(input)), PathBuf::from(expected), "{input}");
        }
        let canonical = std::env::temp_dir().canonicalize().unwrap();
        let plain = without_verbatim_prefix(&canonical);
        assert!(!plain.to_string_lossy().starts_with(r"\\?\"));
        assert_eq!(plain.canonicalize().unwrap(), canonical);
    }

    #[cfg(windows)]
    #[test]
    fn regression_rejects_directory_junction_escape() {
        use std::os::windows::process::CommandExt;
        let base =
            std::env::temp_dir().join(format!("openrevo-junction-test-{}", std::process::id()));
        let inside = base.join("inside");
        let outside = base.join("outside");
        std::fs::create_dir_all(&inside).unwrap();
        std::fs::create_dir(&outside).unwrap();
        let link = inside.join("linked");
        let result = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference = 'Stop'; New-Item -ItemType Junction -Path $env:OPENREVO_TEST_LINK -Target $env:OPENREVO_TEST_TARGET | Out-Null"])
            .env("OPENREVO_TEST_LINK", &link).env("OPENREVO_TEST_TARGET", &outside)
            .creation_flags(0x08000000).output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(resolve_within(&inside, "linked").is_err());
        assert!(resolve_within(&inside, "linked/new/file.json").is_err());
        std::fs::remove_dir(link).unwrap();
        std::fs::remove_dir(inside).unwrap();
        std::fs::remove_dir(outside).unwrap();
        std::fs::remove_dir(base).unwrap();
    }
}
