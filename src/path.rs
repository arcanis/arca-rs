use path_slash::PathBufExt;
use radix_trie::TrieCommon;
use std::path::{PathBuf, Path};

#[derive(Debug, Default, Clone)]
pub struct Trie<T> {
    inner: radix_trie::Trie<String, (PathBuf, T)>,
}

impl<T> Trie<T> {
    fn key<P: AsRef<Path>>(&self, key: &P) -> String {
        let mut p = normalize_path(key.as_ref().to_string_lossy());

        if !p.ends_with('/') {
            p.push('/');
        }

        p
    }

    pub fn get<P: AsRef<Path>>(&self, key: &P) -> Option<&T> {
        self.inner.get(&self.key(&key)).map(|t| &t.1)
    }

    pub fn get_mut<P: AsRef<Path>>(&mut self, key: &P) -> Option<&mut T> {
        self.inner.get_mut(&self.key(&key)).map(|t| &mut t.1)
    }

    pub fn get_ancestor_record<P: AsRef<Path>>(&self, key: &P) -> Option<(&String, &PathBuf, &T)> {
        self.inner.get_ancestor(&self.key(&key)).map(|e| {
            let k = e.key().unwrap();
            let v = e.value().unwrap();
            
            (k, &v.0, &v.1)
        })
    }

    pub fn get_ancestor_key<P: AsRef<Path>>(&self, key: &P) -> Option<&String> {
        self.inner.get_ancestor(&self.key(&key)).and_then(|e| e.key())
    }

    pub fn get_ancestor_path<P: AsRef<Path>>(&self, key: &P) -> Option<&PathBuf> {
        self.inner.get_ancestor_value(&self.key(&key)).map(|t| &t.0)
    }

    pub fn get_ancestor_value<P: AsRef<Path>>(&self, key: &P) -> Option<&T> {
        self.inner.get_ancestor_value(&self.key(&key)).map(|t| &t.1)
    }

    pub fn insert<P: AsRef<Path>>(&mut self, key: P, value: T) -> () {
        let k = self.key(&key);
        let p = PathBuf::from(k.clone());

        self.inner.insert(k, (p, value)).map(|t| t.1);
    }

    pub fn remove<P: AsRef<Path>>(&mut self, key: &P) -> () {
        self.inner.remove(&self.key(&key));
    }
}

pub fn normalize_path<P: AsRef<str>>(original: P) -> String {
    let original_str = original.as_ref();

    let p = PathBuf::from(original_str);
    let mut str = clean_path::clean(p)
        .to_slash_lossy()
        .to_string();

    if original_str.ends_with('/') && !str.ends_with('/') {
        str.push('/');
    }

    str
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path(""), ".");
        assert_eq!(normalize_path("/"), "/");
        assert_eq!(normalize_path("foo"), "foo");
        assert_eq!(normalize_path("foo/bar"), "foo/bar");
        assert_eq!(normalize_path("foo//bar"), "foo/bar");
        assert_eq!(normalize_path("foo/./bar"), "foo/bar");
        assert_eq!(normalize_path("foo/../bar"), "bar");
        assert_eq!(normalize_path("foo/bar/.."), "foo");
        assert_eq!(normalize_path("foo/../../bar"), "../bar");
        assert_eq!(normalize_path("../foo/../../bar"), "../../bar");
        assert_eq!(normalize_path("./foo"), "foo");
        assert_eq!(normalize_path("../foo"), "../foo");
        assert_eq!(normalize_path("/foo/bar"), "/foo/bar");
        assert_eq!(normalize_path("/foo/bar/"), "/foo/bar/");
    }
}
