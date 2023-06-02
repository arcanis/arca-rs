use std::fmt::{Debug, Formatter};

use radix_trie::TrieCommon;

pub mod path;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Path {
    path: String,
}

impl Path {
    pub fn new() -> Self {
        Path::from("")
    }

    pub fn dirname<'a>(&'a self) -> Path {
        let mut slice_len = self.path.len();
        if self.path.ends_with('/') {
            if self.path.len() > 1 {
                slice_len -= 1;
            } else {
                return Path::from("/");
            }
        }

        let slice = &self.path[..slice_len];
        if let Some(last_slash) = slice.rfind('/') {
            Path::from(&slice[..last_slash])
        } else {
            Path::new()
        }
    }

    pub fn basename<'a>(&'a self) -> Option<&'a str> {
        let mut slice_len = self.path.len();
        if self.path.ends_with('/') {
            slice_len -= 1;
        }

        let slice = &self.path[..slice_len];
        if let Some(last_slash) = slice.rfind('/') {
            if last_slash != self.path.len() - 1 {
                Some(&slice[last_slash + 1..])
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn extname<'a>(&'a self) -> Option<&'a str> {
        self.basename().and_then(|basename| {
            if let Some(last_dot) = basename.rfind('.') {
                if last_dot != 0 {
                    Some(&basename[last_dot..])
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    pub fn as_str<'a>(&'a self) -> &'a str {
        self.path.as_str()
    }

    pub fn to_path_buf(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(&self.path)
    }

    pub fn is_root(&self) -> bool {
        self.path == "/"
    }

    pub fn is_absolute(&self) -> bool {
        self.path.starts_with('/')
    }

    pub fn is_relative(&self) -> bool {
        !self.is_absolute()
    }

    pub fn is_extern(&self) -> bool {
        self.path.starts_with("../") || self.path == ".."
    }

    pub fn join(&self, other: &Path) -> Path {
        let mut copy = self.clone();
        copy.go_to(other);
        copy
    }

    pub fn join_str<T>(&self, other: T) -> Path
    where
        T: AsRef<str>,
    {
        let mut copy = self.clone();
        copy.go_to(&Path::from(other.as_ref()));
        copy
    }

    pub fn go_to(&mut self, other: &Path) {
        if other.path.starts_with('/') {
            self.path = other.path.clone();
        } else {
            if !self.path.ends_with('/') {
                self.path.push('/');
            }
            self.path.push_str(&other.path);
            self.normalize()
        }
    }

    pub fn go_to_str<T>(&mut self, other: T)
    where
        T: AsRef<str>,
    {
        self.go_to(&Path::from(other.as_ref()));
    }

    pub fn relative_to(&self, other: &Path) -> Path {
        assert!(self.is_absolute() && other.is_absolute());

        let self_components: Vec<&str> = self.path.trim_start_matches('/').split('/').collect();
        let other_components: Vec<&str> = other.path.trim_start_matches('/').split('/').collect();

        let common_prefix_length = self_components.iter()
            .zip(other_components.iter())
            .take_while(|(a, b)| a == b)
            .count();

        let mut relative_path = vec![];

        for _ in common_prefix_length..self_components.len() {
            if self_components[common_prefix_length..].len() > 0 {
                relative_path.push("..");
            }
        }

        for component in other_components[common_prefix_length..].iter() {
            relative_path.push(*component);
        }

        if relative_path.is_empty() {
            Path::from(".")
        } else {
            Path::from(relative_path.join("/"))
        }
    }

    fn normalize(&mut self) {
        self.path = resolve_path(&self.path);
    }
}

impl Debug for Path {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Path({})", self.path)
    }
}

impl Default for Path {
    fn default() -> Self {
        Path::new()
    }
}

impl<T: AsRef<str>> From<T> for Path {
    fn from(path: T) -> Self {
        Path {
            path: resolve_path(path.as_ref()),
        }
    }
}

impl ToString for Path {
    fn to_string(&self) -> String {
        self.path.clone()
    }
}

pub trait ToArcaPath {
    fn to_arca(&self) -> Path;
}

impl ToArcaPath for std::path::Path {
    fn to_arca(&self) -> Path {
        Path::from(self.to_string_lossy().to_owned())
    }
}

impl ToArcaPath for std::path::PathBuf {
    fn to_arca(&self) -> Path {
        Path::from(self.to_string_lossy().to_owned())
    }
}

#[cfg(feature = "napi")]
impl napi::bindgen_prelude::TypeName for Path {
    fn type_name() -> &'static str {
        String::type_name()
    }
  
    fn value_type() -> napi::ValueType {
        String::value_type()
    }
}

#[cfg(feature = "napi")]
impl napi::bindgen_prelude::ValidateNapiValue for Path {
    unsafe fn validate(env: napi::sys::napi_env, napi_val: napi::sys::napi_value) -> napi::Result<napi::sys::napi_value> {
        let mut result = -1;
        napi::check_status!(
            unsafe { napi::sys::napi_typeof(env, napi_val, &mut result) },
            "Failed to detect napi value type",
        )?;

        let received_type = napi::ValueType::from(result);
        if let Ok(validate_ret) = unsafe { String::validate(env, napi_val) } {
            Ok(validate_ret)
        } else {
            Err(napi::Error::new(
                napi::Status::InvalidArg,
                format!(
                    "Expect value to be String, but received {}",
                    received_type
                ),
            ))
        }
    }
}

#[cfg(feature = "napi")]
impl napi::bindgen_prelude::FromNapiValue for Path {
  unsafe fn from_napi_value(env: napi::sys::napi_env, napi_val: napi::sys::napi_value) -> napi::Result<Self> {
    let mut val_type = 0;

    napi::check_status!(
        unsafe { napi::sys::napi_typeof(env, napi_val, &mut val_type) },
        "Failed to convert napi value into rust type `Path`",
    )?;

    Ok(Path::from(unsafe { String::from_napi_value(env, napi_val)? }))
  }
}

#[cfg(feature = "napi")]
impl napi::bindgen_prelude::ToNapiValue for Path {
    unsafe fn to_napi_value(env: napi::sys::napi_env, val: Self) -> napi::Result<napi::sys::napi_value> {
        unsafe { String::to_napi_value(env, val.path) }
    }
}

fn resolve_path(input: &str) -> String {
    if input.is_empty() {
        return "".to_string();
    }

    let mut path = Vec::new();
    for component in input.split('/') {
        match component {
            ".." => {
                let last = path.last();
                if last == Some(&"") {
                    // Do nothing
                } else if last != None && last != Some(&"..") {
                    path.pop();
                } else {
                    path.push("..");
                }
            },
            "." => {},
            "" => {
                if path.is_empty() {
                    path.push("");
                }
            },
            _ => {
                path.push(component);
            },
        }
    }

    if input.ends_with("/") {
        path.push("");
    }

    if path == vec![""] {
        return "/".to_string();
    } else {
        format!("{}", path.join("/"))
    }
}

#[derive(Debug, Default, Clone)]
pub struct Trie<T> {
    inner: radix_trie::Trie<String, (Path, T)>,
}

impl<T> Trie<T> {
    fn key(&self, key: &Path) -> String {
        let mut p = key.to_string();

        if !p.ends_with('/') {
            p.push('/');
        }

        p
    }

    pub fn get(&self, key: &Path) -> Option<&T> {
        self.inner.get(&self.key(&key)).map(|t| &t.1)
    }

    pub fn get_mut(&mut self, key: &Path) -> Option<&mut T> {
        self.inner.get_mut(&self.key(&key)).map(|t| &mut t.1)
    }

    pub fn get_ancestor_record(&self, key: &Path) -> Option<(&String, &Path, &T)> {
        self.inner.get_ancestor(&self.key(&key)).map(|e| {
            let k = e.key().unwrap();
            let v = e.value().unwrap();

            (k, &v.0, &v.1)
        })
    }

    pub fn get_ancestor_key(&self, key: &Path) -> Option<&String> {
        self.inner.get_ancestor(&self.key(&key)).and_then(|e| e.key())
    }

    pub fn get_ancestor_path(&self, key: &Path) -> Option<&Path> {
        self.inner.get_ancestor_value(&self.key(&key)).map(|t| &t.0)
    }

    pub fn get_ancestor_value(&self, key: &Path) -> Option<&T> {
        self.inner.get_ancestor_value(&self.key(&key)).map(|t| &t.1)
    }

    pub fn insert(&mut self, key: Path, value: T) -> () {
        let k = self.key(&key);
        let p = Path::from(k.clone());

        self.inner.insert(k, (p, value)).map(|t| t.1);
    }

    pub fn remove(&mut self, key: &Path) -> () {
        self.inner.remove(&self.key(&key));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_path() {
        assert_eq!(resolve_path("/a/b/c/./../d/"), "/a/b/d/");
        assert_eq!(resolve_path("../foo"), "../foo");
        assert_eq!(resolve_path("./../foo"), "../foo");
        assert_eq!(resolve_path("/a/./b/../../c"), "/c");
        assert_eq!(resolve_path("/a/.."), "/");
        assert_eq!(resolve_path("/../../a"), "/a");
        assert_eq!(resolve_path("./a/"), "a/");
        assert_eq!(resolve_path(""), "");
        assert_eq!(resolve_path("a/b/../../c"), "c");
        assert_eq!(resolve_path("../a/./b/c/../../"), "../a/");
        assert_eq!(resolve_path("/.."), "/");
        assert_eq!(resolve_path("/."), "/");
        assert_eq!(resolve_path("./."), "");
        assert_eq!(resolve_path("../../../foo"), "../../../foo");
        assert_eq!(resolve_path("./././a"), "a");
        assert_eq!(resolve_path("b/./c/././d"), "b/c/d");
        assert_eq!(resolve_path("foo/../../bar"), "../bar");
        assert_eq!(resolve_path("/foo/bar/../../../baz"), "/baz");
    }

    #[test]
    fn test_same_path() {
        let path1 = Path { path: "/home/user/docs".to_string() };
        let path2 = Path { path: "/home/user/docs".to_string() };
        assert_eq!(path1.relative_to(&path2), Path::from(""));
    }

    #[test]
    fn test_subdirectory() {
        let path1 = Path { path: "/home/user/docs".to_string() };
        let path2 = Path { path: "/home/user/docs/reports".to_string() };
        assert_eq!(path1.relative_to(&path2), Path::from("reports"));
    }

    #[test]
    fn test_parent_directory() {
        let path1 = Path { path: "/home/user/docs/reports".to_string() };
        let path2 = Path { path: "/home/user/docs".to_string() };
        assert_eq!(path1.relative_to(&path2), Path::from(".."));
    }

    #[test]
    fn test_different_directory() {
        let path1 = Path { path: "/home/user/docs".to_string() };
        let path2 = Path { path: "/home/user/music".to_string() };
        assert_eq!(path1.relative_to(&path2), Path::from("../music"));
    }

    #[test]
    fn test_different_root() {
        let path1 = Path { path: "/home/user/docs".to_string() };
        let path2 = Path { path: "/var/log".to_string() };
        assert_eq!(path1.relative_to(&path2), Path::from("../../../var/log"));
    }

    #[test]
    #[should_panic]
    fn test_relative_path() {
        let path1 = Path { path: "/home/user/docs".to_string() };
        let path2 = Path { path: "var/log".to_string() };
        path1.relative_to(&path2);
    }

    #[test]
    fn test_dirname_with_extension() {
        let path = Path { path: "/usr/local/bin/test.txt".to_string() };
        assert_eq!(path.dirname(), Path::from("/usr/local/bin"));
    }

    #[test]
    fn test_dirname_without_extension() {
        let path = Path { path: "/usr/local/bin/test".to_string() };
        assert_eq!(path.dirname(), Path::from("/usr/local/bin"));
    }

    #[test]
    fn test_dirname_with_trailing_slash() {
        let path = Path { path: "/usr/local/bin/".to_string() };
        assert_eq!(path.dirname(), Path::from("/usr/local"));
    }

    #[test]
    fn test_dirname_with_single_slash() {
        let path = Path { path: "/".to_string() };
        assert_eq!(path.dirname(), Path::from("/"));
    }

    #[test]
    fn test_dirname_with_empty_string() {
        let path = Path { path: "".to_string() };
        assert_eq!(path.dirname(), Path::from(""));
    }

    #[test]
    fn test_basename_with_extension() {
        let path = Path { path: "/usr/local/bin/test.txt".to_string() };
        assert_eq!(path.basename(), Some("test.txt"));
    }

    #[test]
    fn test_basename_without_extension() {
        let path = Path { path: "/usr/local/bin/test".to_string() };
        assert_eq!(path.basename(), Some("test"));
    }

    #[test]
    fn test_basename_with_trailing_slash() {
        let path = Path { path: "/usr/local/bin/".to_string() };
        assert_eq!(path.basename(), Some("bin"));
    }

    #[test]
    fn test_basename_with_single_slash() {
        let path = Path { path: "/".to_string() };
        assert_eq!(path.basename(), None);
    }

    #[test]
    fn test_basename_with_empty_string() {
        let path = Path { path: "".to_string() };
        assert_eq!(path.basename(), None);
    }

    #[test]
    fn test_extname_with_extension() {
        let path = Path { path: "/usr/local/bin/test.txt".to_string() };
        assert_eq!(path.extname(), Some(".txt"));
    }

    #[test]
    fn test_extname_with_double_extension() {
        let path = Path { path: "/usr/local/bin/test.foo.txt".to_string() };
        assert_eq!(path.extname(), Some(".txt"));
    }

    #[test]
    fn test_extname_without_extension() {
        let path = Path { path: "/usr/local/bin/test".to_string() };
        assert_eq!(path.extname(), None);
    }

    #[test]
    fn test_extname_with_trailing_slash() {
        let path = Path { path: "/usr/local/bin/.htaccess".to_string() };
        assert_eq!(path.extname(), None);
    }

    #[test]
    fn test_extname_with_single_slash() {
        let path = Path { path: "/".to_string() };
        assert_eq!(path.extname(), None);
    }

    #[test]
    fn test_extname_with_empty_string() {
        let path = Path { path: "".to_string() };
        assert_eq!(path.extname(), None);
    }

    #[test]
    fn test_trie_insert() {
        let mut trie = Trie::default();
        let path = Path::from("/path/to/item/");
        let item = "item";

        trie.insert(path.clone(), item.to_string());

        assert_eq!(trie.get(&path).unwrap(), item);
    }

    #[test]
    fn test_trie_remove() {
        let mut trie = Trie::default();
        let path = Path::from("/path/to/item/");
        let item = "item";

        trie.insert(path.clone(), item.to_string());
        assert_eq!(trie.get(&path).unwrap(), item);

        trie.remove(&path);
        assert_eq!(trie.get(&path), None);
    }

    #[test]
    fn test_get_ancestor_record() {
        let mut trie = Trie::default();
        let path = Path::from("/path/to/item/");
        let item = "item";

        trie.insert(path.clone(), item.to_string());

        let ancestor_path = Path::from("/path/to/item/child");
        assert_eq!(trie.get_ancestor_record(&ancestor_path).unwrap().2, item);
    }

    #[test]
    fn test_get_ancestor_key() {
        let mut trie = Trie::default();
        let path = Path::from("/path/to/item/");
        let item = "item";

        trie.insert(path.clone(), item.to_string());

        let ancestor_path = Path::from("/path/to/item/child");
        assert_eq!(trie.get_ancestor_key(&ancestor_path).unwrap(), "/path/to/item/");
    }

    #[test]
    fn test_get_ancestor_path() {
        let mut trie = Trie::default();
        let path = Path::from("/path/to/item/");
        let item = "item";

        trie.insert(path.clone(), item.to_string());

        let ancestor_path = Path::from("/path/to/item/child");
        assert_eq!(trie.get_ancestor_path(&ancestor_path).unwrap(), &path);
    }

    #[test]
    fn test_get_ancestor_value() {
        let mut trie = Trie::default();
        let path = Path::from("/path/to/item/");
        let item = "item";

        trie.insert(path.clone(), item.to_string());

        let ancestor_path = Path::from("/path/to/item/child");
        assert_eq!(trie.get_ancestor_value(&ancestor_path).unwrap(), item);
    }
}
