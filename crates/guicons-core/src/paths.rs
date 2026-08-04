use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn find_workspace_root(manifest_path: &Path) -> Option<PathBuf> {
    let start = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    find_workspace_root_from(start)
}

pub(crate) fn resolve_workspace_path(workspace_root: &Path, value: &str) -> PathBuf {
    resolve_entry_path(workspace_root, value)
}

pub(crate) fn resolve_entry_path(root: &Path, value: &str) -> PathBuf {
    let path = Path::new(value);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

/// `dunce::canonicalize`, not `std::fs::canonicalize` - on Windows the
/// latter prefixes the result with the `\\?\` verbatim path marker, which
/// then leaks into any `file://` URI built from it (`Url::from_file_path`
/// has no idea it's not a real path segment) and stops matching the
/// plain, non-canonicalized URI a real LSP client sends for the same
/// file. Hit exactly this: `textDocument/rename`'s workspace edit came
/// back keyed by a `\\?\`-derived URI the client never actually opened,
/// so the edit silently applied to nothing. `dunce` canonicalizes the
/// same way otherwise, just without ever emitting that prefix.
pub fn canonicalize_or_self(path: &Path) -> PathBuf {
    dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Walks up from `start` looking for the nearest `Cargo.toml` that declares
/// `[workspace]` or `[package]`, i.e. the crate/workspace root.
pub fn find_workspace_root_from(start: &Path) -> Option<PathBuf> {
    let mut current = canonicalize_or_self(start);
    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = fs::read_to_string(&cargo_toml).ok()?;
            if content.contains("[workspace]") || content.contains("[package]") {
                return Some(current);
            }
        }
        current = current.parent()?.to_path_buf();
    }
}

/// The `icons.gui.toml` governing `rust_file` - its own crate's manifest
/// (via `find_workspace_root_from`, which stops at the nearest ancestor
/// `Cargo.toml` rather than climbing to a cargo *workspace* root), not
/// any manifest that happens to exist elsewhere. In a multi-crate
/// workspace, a `.rs` file must only ever resolve against its own
/// crate's manifest - never a different crate's, even if one is sitting
/// right next to it (a real bug once, in `guicons-lsp`'s hover).
/// `None` if there's no `Cargo.toml` above `rust_file`, or no
/// `icons.gui.toml` beside it. Follows [`resolve_manifest_redirect`] if
/// that `icons.gui.toml` turns out to be a pointer rather than a real
/// manifest.
pub fn manifest_path_for_rust_file(rust_file: &Path) -> Option<PathBuf> {
    let crate_root = find_workspace_root_from(rust_file.parent()?)?;
    let manifest = crate_root.join("icons.gui.toml");
    if !manifest.is_file() {
        return None;
    }
    Some(resolve_manifest_redirect(&manifest))
}

/// A crate's own `icons.gui.toml` can be a *pointer* instead of a real
/// manifest - just a `root_manifest = "<path>"` line, nothing else - for
/// the monorepo case where several crates actually share one manifest
/// that doesn't live next to any of them (e.g. at the repo root). This
/// resolves `path` to whatever it points at (relative to `path`'s own
/// directory), unchanged if `path` isn't a pointer (missing, unreadable,
/// invalid TOML, or just a real manifest with no `root_manifest` key).
/// Both `manifest_path_for_rust_file` and `crate::load`'s entry points
/// funnel through this, so build.rs's `IconBuild::auto()`, the LSP, and
/// the IDE plugin all follow the same pointer transparently - no
/// separate lookup mechanism, no `IconBuild::new(path)` override needed
/// once the pointer file exists.
pub fn resolve_manifest_redirect(path: &Path) -> PathBuf {
    match fs::read_to_string(path) {
        Ok(content) => resolve_manifest_redirect_content(path, &content),
        Err(_) => path.to_path_buf(),
    }
}

/// Like [`resolve_manifest_redirect`], but checking already-in-memory
/// `content` instead of reading `path` from disk - for editor tooling
/// that's showing an unsaved buffer (the pointer file itself might be
/// what's currently open).
pub fn resolve_manifest_redirect_content(path: &Path, content: &str) -> PathBuf {
    let Ok(root) = toml_span::parse(content) else { return path.to_path_buf() };
    let Some(target) = root.pointer("/root_manifest").and_then(|v| v.as_str()) else {
        return path.to_path_buf();
    };
    // Canonicalized (unlike the pass-through branches above, which return
    // `path` completely unchanged on purpose - see `load()`'s use of this
    // to decide whether `content_override` still applies) because a
    // `root_manifest` value is almost always a `../`-relative path, and
    // `Path::join` never collapses those - leaving them in would make two
    // pointers resolving to the very same real file compare unequal.
    canonicalize_or_self(&path.parent().unwrap_or_else(|| Path::new(".")).join(target))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn resolve_manifest_redirect_follows_a_pointer_to_a_relative_target() {
        let dir = tempdir().unwrap();
        let stub = dir.path().join("crates/app/icons.gui.toml");
        fs::create_dir_all(stub.parent().unwrap()).unwrap();
        fs::write(&stub, "root_manifest = \"../../icons.gui.toml\"\n").unwrap();
        // The target doesn't need to exist for resolution itself, but
        // canonicalize_or_self only cleans up `..` when it does.
        fs::write(dir.path().join("icons.gui.toml"), "").unwrap();

        let resolved = resolve_manifest_redirect(&stub);
        assert_eq!(resolved, canonicalize_or_self(&dir.path().join("icons.gui.toml")));
    }

    #[test]
    fn resolve_manifest_redirect_leaves_a_real_manifest_untouched() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("icons.gui.toml");
        fs::write(&manifest, "[docker]\nfile = \"docker.svg\"\n").unwrap();

        assert_eq!(resolve_manifest_redirect(&manifest), manifest);
    }

    #[test]
    fn resolve_manifest_redirect_leaves_a_missing_file_untouched() {
        let missing = Path::new("/does/not/exist/icons.gui.toml");
        assert_eq!(resolve_manifest_redirect(missing), missing);
    }

    #[test]
    fn resolve_manifest_redirect_content_resolves_a_relative_pointer() {
        let dir = tempdir().unwrap();
        let stub_path = dir.path().join("crates/app/icons.gui.toml");
        fs::write(dir.path().join("icons.gui.toml"), "").unwrap();

        let resolved = resolve_manifest_redirect_content(&stub_path, "root_manifest = \"../../icons.gui.toml\"\n");
        assert_eq!(resolved, canonicalize_or_self(&dir.path().join("icons.gui.toml")));
    }
}
