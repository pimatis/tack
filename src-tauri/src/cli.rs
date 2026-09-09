use std::path::PathBuf;

// resolvers for the bundled cli binary and its symlink target (macOS only)
const PATH_CANDIDATES: [&str; 2] = ["/usr/local/bin", "/opt/homebrew/bin"];

fn bundled_cli_bin() -> Option<PathBuf> {
    // sidecar is copied next to the executable in both dev and bundled apps
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    let cli_bin = exe_dir.join("tack-cli");
    cli_bin.exists().then_some(cli_bin)
}

// pick a directory on PATH we can write to: a system bin dir, else the user's ~/.local/bin
fn cli_bin_dir() -> Option<PathBuf> {
    let system = PATH_CANDIDATES
        .iter()
        .map(PathBuf::from)
        .find(|dir| dir.exists() && dir.join("tack").exists());
    if let Some(dir) = system {
        return Some(dir);
    }

    let home = dirs::home_dir()?;
    let local = home.join(".local").join("bin");
    if std::fs::create_dir_all(&local).is_ok() {
        Some(local)
    } else {
        None
    }
}

fn cli_symlink_target() -> Option<PathBuf> {
    cli_bin_dir().map(|dir| dir.join("tack"))
}

// make sure the chosen bin dir is on PATH (only needed when we fell back to ~/.local/bin)
fn ensure_cli_on_path(target: &std::path::Path) {
    let Some(dir) = target.parent() else {
        return;
    };
    let Some(home) = dirs::home_dir() else {
        return;
    };
    let dir_str = dir.to_string_lossy().to_string();
    for rc in [".zshrc", ".bashrc"] {
        let shell_rc = home.join(rc);
        let already = std::fs::read_to_string(&shell_rc)
            .map(|c| c.contains(&dir_str))
            .unwrap_or(false);
        if already {
            continue;
        }
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&shell_rc) {
            use std::io::Write;
            let _ = writeln!(f, "export PATH=\"{dir_str}:$PATH\"");
        }
    }
}

// place a symlink to the bundled cli on PATH so `tack` works from any terminal
pub(crate) fn install_cli_link() -> Result<bool, String> {
    if !cfg!(target_os = "macos") {
        return Ok(false);
    }
    let Some(cli_bin) = bundled_cli_bin() else {
        return Err("bundled cli not found".to_string());
    };
    let Some(target) = cli_symlink_target() else {
        return Err("no writable bin directory found".to_string());
    };
    if !target.exists() {
        std::os::unix::fs::symlink(&cli_bin, &target)
            .map_err(|e| format!("failed to link cli: {}", e))?;
    }
    ensure_cli_on_path(&target);
    Ok(true)
}

#[tauri::command]
pub fn install_cli() -> Result<bool, String> {
    install_cli_link()
}

#[tauri::command]
pub fn cli_installed() -> bool {
    cli_symlink_target()
        .map(|target| target.exists())
        .unwrap_or(false)
}
