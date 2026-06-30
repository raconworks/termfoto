use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FavoriteEntry {
    pub path: PathBuf,
    pub added_at_ms: u64,
}

#[derive(Debug, Clone)]
pub struct FavoriteStore {
    path: PathBuf,
    entries: Vec<FavoriteEntry>,
}

impl FavoriteStore {
    pub fn load_default() -> Self {
        let path = Self::default_path();
        Self::load(path.clone()).unwrap_or_else(|_| Self::empty_at(path))
    }

    pub fn default_path() -> PathBuf {
        if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME") {
            return PathBuf::from(config_home)
                .join("termfoto")
                .join("favorites.tsv");
        }
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join(".config")
                .join("termfoto")
                .join("favorites.tsv");
        }
        PathBuf::from(".termfoto-favorites.tsv")
    }

    pub fn empty_at(path: PathBuf) -> Self {
        Self {
            path,
            entries: Vec::new(),
        }
    }

    pub fn load(path: PathBuf) -> io::Result<Self> {
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(err) if err.kind() == io::ErrorKind::NotFound => String::new(),
            Err(err) => return Err(err),
        };

        let mut seen = HashSet::new();
        let mut entries = Vec::new();
        for line in content.lines() {
            let Some((added_at_ms, path_text)) = line.split_once('\t') else {
                continue;
            };
            let Ok(added_at_ms) = added_at_ms.parse::<u64>() else {
                continue;
            };
            let Some(path_text) = unescape_field(path_text) else {
                continue;
            };
            let normalized = Self::normalize_path(Path::new(&path_text));
            if seen.insert(normalized.clone()) {
                entries.push(FavoriteEntry {
                    path: normalized,
                    added_at_ms,
                });
            }
        }

        Ok(Self { path, entries })
    }

    #[cfg(test)]
    pub fn entries(&self) -> &[FavoriteEntry] {
        &self.entries
    }

    pub fn entries_newest_first(&self) -> Vec<FavoriteEntry> {
        let mut entries = self.entries.clone();
        entries.sort_by(|a, b| {
            b.added_at_ms
                .cmp(&a.added_at_ms)
                .then_with(|| a.path.cmp(&b.path))
        });
        entries
    }

    pub fn is_favorite(&self, path: &Path) -> bool {
        let key = Self::normalize_path(path);
        self.entries.iter().any(|entry| entry.path == key)
    }

    pub fn add_now(&mut self, path: &Path) -> io::Result<bool> {
        self.add_at(path, now_ms())
    }

    pub fn add_at(&mut self, path: &Path, added_at_ms: u64) -> io::Result<bool> {
        let key = Self::normalize_path(path);
        if self.entries.iter().any(|entry| entry.path == key) {
            return Ok(false);
        }
        self.entries.push(FavoriteEntry {
            path: key,
            added_at_ms,
        });
        self.save()?;
        Ok(true)
    }

    pub fn remove(&mut self, path: &Path) -> io::Result<Option<FavoriteEntry>> {
        let key = Self::normalize_path(path);
        let Some(idx) = self.entries.iter().position(|entry| entry.path == key) else {
            return Ok(None);
        };
        let removed = self.entries.remove(idx);
        self.save()?;
        Ok(Some(removed))
    }

    pub fn update_path(&mut self, old_path: &Path, new_path: &Path) -> io::Result<bool> {
        let old_key = Self::normalize_path(old_path);
        let new_key = Self::normalize_path(new_path);
        let Some(idx) = self.entries.iter().position(|entry| entry.path == old_key) else {
            return Ok(false);
        };
        let added_at_ms = self.entries[idx].added_at_ms;
        self.entries.retain(|entry| entry.path != new_key);
        let idx = self
            .entries
            .iter()
            .position(|entry| entry.path == old_key)
            .unwrap_or(self.entries.len());
        if idx == self.entries.len() {
            self.entries.push(FavoriteEntry {
                path: new_key,
                added_at_ms,
            });
        } else {
            self.entries[idx] = FavoriteEntry {
                path: new_key,
                added_at_ms,
            };
        }
        self.save()?;
        Ok(true)
    }

    pub fn save(&self) -> io::Result<()> {
        if let Some(parent) = self
            .path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        let mut file = File::create(&self.path)?;
        for entry in &self.entries {
            writeln!(
                file,
                "{}\t{}",
                entry.added_at_ms,
                escape_field(&entry.path.to_string_lossy())
            )?;
        }
        Ok(())
    }

    pub fn normalize_path(path: &Path) -> PathBuf {
        if let Ok(path) = fs::canonicalize(path) {
            return path;
        }
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .map(|cwd| cwd.join(path))
                .unwrap_or_else(|_| path.to_path_buf())
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

fn escape_field(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '\t' => escaped.push_str("\\t"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn unescape_field(value: &str) -> Option<String> {
    let mut unescaped = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            unescaped.push(ch);
            continue;
        }
        match chars.next()? {
            '\\' => unescaped.push('\\'),
            't' => unescaped.push('\t'),
            'n' => unescaped.push('\n'),
            'r' => unescaped.push('\r'),
            _ => return None,
        }
    }
    Some(unescaped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn favorite_store_persists_deduplicates_and_sorts_newest_first() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("favorites.tsv");
        let one = dir.path().join("one.png");
        let two = dir.path().join("two.png");

        let mut store = FavoriteStore::empty_at(path.clone());
        store.add_at(&one, 10).unwrap();
        store.add_at(&two, 20).unwrap();
        store.add_at(&one, 30).unwrap();

        let loaded = FavoriteStore::load(path).unwrap();
        assert_eq!(loaded.entries().len(), 2);
        let newest: Vec<_> = loaded
            .entries_newest_first()
            .into_iter()
            .map(|entry| entry.path)
            .collect();
        assert_eq!(
            newest,
            vec![
                FavoriteStore::normalize_path(&two),
                FavoriteStore::normalize_path(&one)
            ]
        );
    }

    #[test]
    fn favorite_store_ignores_bad_lines() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("favorites.tsv");
        fs::write(
            &path,
            format!(
                "bad\nnot-a-number\t{}\n42\t{}\n",
                dir.path().join("bad.png").display(),
                dir.path().join("ok.png").display()
            ),
        )
        .unwrap();

        let loaded = FavoriteStore::load(path).unwrap();

        assert_eq!(loaded.entries().len(), 1);
        assert_eq!(loaded.entries()[0].added_at_ms, 42);
    }

    #[test]
    fn favorite_store_updates_path_and_keeps_added_time() {
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("favorites.tsv");
        let old = dir.path().join("old.png");
        let new = dir.path().join("new.png");
        let mut store = FavoriteStore::empty_at(store_path);

        store.add_at(&old, 55).unwrap();
        store.update_path(&old, &new).unwrap();

        assert!(!store.is_favorite(&old));
        assert!(store.is_favorite(&new));
        assert_eq!(store.entries()[0].added_at_ms, 55);
    }
}
