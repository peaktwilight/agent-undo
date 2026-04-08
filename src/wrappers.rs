use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::paths::ProjectPaths;

#[derive(Debug, Clone, Copy)]
pub struct WrapperPreset {
    pub name: &'static str,
    pub agent: &'static str,
    pub binary: &'static str,
}

const PRESETS: &[WrapperPreset] = &[
    WrapperPreset {
        name: "codex",
        agent: "codex",
        binary: "codex",
    },
    WrapperPreset {
        name: "aider",
        agent: "aider",
        binary: "aider",
    },
    WrapperPreset {
        name: "claude",
        agent: "claude-code",
        binary: "claude",
    },
];

pub fn install_wrapper(
    paths: &ProjectPaths,
    au_bin: &std::path::Path,
    agent: &str,
    binary: &str,
    force: bool,
) -> Result<PathBuf> {
    std::fs::create_dir_all(&paths.bin_dir)
        .with_context(|| format!("creating {}", paths.bin_dir.display()))?;

    let wrapper_path = paths.bin_dir.join(binary);
    if wrapper_path.exists() && !force {
        anyhow::bail!(
            "{} already exists. Re-run with --force to overwrite it.",
            wrapper_path.display()
        );
    }

    let body = render_wrapper(au_bin, agent, binary);
    std::fs::write(&wrapper_path, body.as_bytes())
        .with_context(|| format!("writing {}", wrapper_path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&wrapper_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&wrapper_path, perms)?;
    }

    Ok(wrapper_path)
}

pub fn shellenv(paths: &ProjectPaths) -> String {
    format!("export PATH=\"{}:$PATH\"", paths.bin_dir.display())
}

pub fn presets() -> &'static [WrapperPreset] {
    PRESETS
}

pub fn preset(name: &str) -> Option<WrapperPreset> {
    PRESETS
        .iter()
        .copied()
        .find(|preset| preset.name.eq_ignore_ascii_case(name))
}

pub fn detect_presets_in_path() -> Vec<WrapperPreset> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut found = Vec::new();

    for preset in PRESETS.iter().copied() {
        if binary_in_path(preset.binary, &path) {
            found.push(preset);
        }
    }

    found
}

pub fn list_wrappers(paths: &ProjectPaths) -> Result<Vec<PathBuf>> {
    if !paths.bin_dir.exists() {
        return Ok(vec![]);
    }

    let mut out = Vec::new();
    for entry in std::fs::read_dir(&paths.bin_dir)
        .with_context(|| format!("reading {}", paths.bin_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

pub fn installed_wrapper_names(paths: &ProjectPaths) -> Result<BTreeSet<String>> {
    let mut names = BTreeSet::new();
    for path in list_wrappers(paths)? {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            let _ = names.insert(name.to_string());
        }
    }
    Ok(names)
}

pub fn remove_wrapper(paths: &ProjectPaths, binary: &str) -> Result<bool> {
    let path = paths.bin_dir.join(binary);
    if !path.exists() {
        return Ok(false);
    }
    std::fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
    Ok(true)
}

fn binary_in_path(binary: &str, path: &std::ffi::OsStr) -> bool {
    std::env::split_paths(path).any(|dir| dir.join(binary).is_file())
}

fn render_wrapper(au_bin: &std::path::Path, agent: &str, binary: &str) -> String {
    format!(
        r#"#!/usr/bin/env sh
set -eu

WRAPPER_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
OLD_IFS="$IFS"
IFS=:
CLEAN_PATH=""
for entry in $PATH; do
  [ "$entry" = "$WRAPPER_DIR" ] && continue
  if [ -z "$CLEAN_PATH" ]; then
    CLEAN_PATH="$entry"
  else
    CLEAN_PATH="$CLEAN_PATH:$entry"
  fi
done
IFS="$OLD_IFS"

PATH="$CLEAN_PATH" exec "{au_bin}" exec --agent "{agent}" -- "{binary}" "$@"
"#,
        au_bin = au_bin.display(),
        agent = agent,
        binary = binary,
    )
}
