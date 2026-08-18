use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Tag -> paths. `BTreeMap`/`BTreeSet` keep everything sorted and de-duplicated,
/// so listing is just iteration and the on-disk file stays stable.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Store {
    #[serde(default)]
    tags: BTreeMap<String, BTreeSet<String>>,
}

impl Store {
    /// Location of the store file: `$PMUX_STORE`, else `$XDG_DATA_HOME/pmux/store.json`,
    /// else `~/.local/share/pmux/store.json`.
    pub fn default_path() -> io::Result<PathBuf> {
        if let Some(p) = env_path("PMUX_STORE") {
            return Ok(p);
        }
        let base = match env_path("XDG_DATA_HOME") {
            Some(p) => p,
            None => env_path("HOME")
                .ok_or_else(|| io::Error::other("cannot locate store: set $HOME or $PMUX_STORE"))?
                .join(".local/share"),
        };
        Ok(base.join("pmux").join("store.json"))
    }

    /// An absent store file is an empty store, not an error.
    pub fn load(path: &Path) -> io::Result<Self> {
        match fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{}: invalid store file: {e}", path.display()),
                )
            }),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e),
        }
    }

    /// Write via a temporary file + rename so an interrupted run cannot truncate the store.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        let mut text = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        text.push('\n');
        fs::write(&tmp, text)?;
        fs::rename(&tmp, path)
    }

    /// Returns false when the tag already held that path.
    pub fn add(&mut self, tag: &str, path: String) -> bool {
        self.tags.entry(tag.to_string()).or_default().insert(path)
    }

    pub fn tags(&self) -> impl Iterator<Item = &str> {
        self.tags.keys().map(String::as_str)
    }

    pub fn paths(&self, tag: &str) -> Option<impl Iterator<Item = &str>> {
        self.tags.get(tag).map(|set| set.iter().map(String::as_str))
    }
}

fn env_path(key: &str) -> Option<PathBuf> {
    match std::env::var_os(key) {
        Some(v) if !v.is_empty() => Some(PathBuf::from(v)),
        _ => None,
    }
}

/// Absolute, tidied form of `input` so the same directory always maps to one entry.
/// Falls back to lexical normalization when the path does not exist yet.
pub fn normalize(input: &str) -> io::Result<String> {
    let expanded = expand_tilde(input);
    let absolute = if expanded.is_absolute() {
        expanded
    } else {
        std::env::current_dir()?.join(expanded)
    };
    let resolved = fs::canonicalize(&absolute).unwrap_or_else(|_| lexical_clean(&absolute));
    resolved
        .into_os_string()
        .into_string()
        .map_err(|_| io::Error::other(format!("path is not valid UTF-8: {input}")))
}

fn expand_tilde(input: &str) -> PathBuf {
    let rest = match input.strip_prefix('~') {
        Some("") => "",
        Some(rest) if rest.starts_with('/') => &rest[1..],
        // `~user` is left alone: we cannot resolve other users' homes.
        _ => return PathBuf::from(input),
    };
    match env_path("HOME") {
        Some(home) => home.join(rest),
        None => PathBuf::from(input),
    }
}

fn lexical_clean(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                // Nothing to pop: keep `..` for a relative path, drop it above the root.
                if !out.pop() && !out.has_root() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_reports_duplicates() {
        let mut store = Store::default();
        assert!(store.add("work", "/a".into()));
        assert!(!store.add("work", "/a".into()));
    }

    #[test]
    fn listings_are_sorted() {
        let mut store = Store::default();
        for (tag, path) in [("b", "/z"), ("a", "/y"), ("b", "/a")] {
            store.add(tag, path.into());
        }
        assert_eq!(store.tags().collect::<Vec<_>>(), ["a", "b"]);
        assert_eq!(store.paths("b").unwrap().collect::<Vec<_>>(), ["/a", "/z"]);
        assert!(store.paths("missing").is_none());
    }

    #[test]
    fn save_then_load_roundtrips() {
        let path = std::env::temp_dir().join("pmux-store-roundtrip/store.json");
        let _ = fs::remove_dir_all(path.parent().unwrap());
        let mut store = Store::default();
        store.add("work", "/srv/app".into());
        store.save(&path).unwrap();

        let loaded = Store::load(&path).unwrap();
        assert_eq!(
            loaded.paths("work").unwrap().collect::<Vec<_>>(),
            ["/srv/app"]
        );
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn missing_file_is_an_empty_store() {
        let store = Store::load(Path::new("/nonexistent/pmux/store.json")).unwrap();
        assert_eq!(store.tags().count(), 0);
    }

    #[test]
    fn normalize_makes_paths_absolute_and_tidy() {
        let cwd = std::env::current_dir().unwrap();
        assert_eq!(normalize("src").unwrap(), cwd.join("src").to_str().unwrap());
        // Non-existent paths are cleaned lexically instead of failing.
        assert_eq!(normalize("/tmp/./a/b/../c").unwrap(), "/tmp/a/c");
    }

    #[test]
    fn lexical_clean_keeps_parent_dir_above_root() {
        assert_eq!(lexical_clean(Path::new("/../a")), PathBuf::from("/a"));
        assert_eq!(lexical_clean(Path::new("../a")), PathBuf::from("../a"));
    }
}
