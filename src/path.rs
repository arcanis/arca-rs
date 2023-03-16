use path_slash::PathBufExt;
use std::{path::{PathBuf, Path}};

pub struct Trie<T> {
    inner: radix_trie::Trie<String, T>,
}

impl<T> Trie<T> {
    fn key<P: AsRef<Path>>(&self, key: &P) -> String {
        let mut p = normalize_path(key.as_ref().to_string_lossy());

        if p.ends_with('/') {
            p.push('/');
        }

        p
    }

    pub fn get<P: AsRef<Path>>(&self, key: &P) -> Option<&T> {
        self.inner.get(&self.key(&key))
    }

    pub fn get_mut<P: AsRef<Path>>(&mut self, key: &P) -> Option<&mut T> {
        self.inner.get_mut(&self.key(&key))
    }

    pub fn insert<P: AsRef<Path>>(&mut self, key: &P, value: T) -> Option<T> {
        self.inner.insert(self.key(&key), value)
    }

    pub fn remove<P: AsRef<Path>>(&mut self, key: &P) -> Option<T> {
        self.inner.remove(&self.key(&key))
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
