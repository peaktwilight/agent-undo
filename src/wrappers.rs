use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::paths::ProjectPaths;

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
