use std::fmt::{Debug, Display, Formatter};
use std::fs::ReadDir;
use std::io::Read;
use std::str::FromStr;
use std::{fs, io};

use radix_trie::TrieCommon;

pub mod path;

#[derive(Debug)]
pub enum ImmutableErr {
    Immutable,
    Io(std::io::Error),
}

pub trait OkMissing<T, E> {
    fn ok_missing(self) -> Result<Option<T>, E>;
}

impl<T> OkMissing<T, std::io::Error> for Result<T, std::io::Error> {
    fn ok_missing(self) -> Result<Option<T>, std::io::Error> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err),
        }
    }
}

pub struct PathIterator<'a> {
    components: Vec<&'a str>,
    front_idx: usize,
    back_idx: usize,
    has_trailing_slash: bool,
}

impl<'a> PathIterator<'a> {
    pub fn new(path: &'a Path) -> Self {
        let path_str = path.as_str();

        let has_leading_slash = path_str.starts_with('/');
        let has_trailing_slash = path_str.ends_with('/') && path_str.len() > 1;

        let components_path = path_str;
        let components_path = components_path.strip_prefix('/').unwrap_or(components_path);
        let components_path = components_path.strip_suffix('/').unwrap_or(components_path);

        let mut components = Vec::new();

        if has_leading_slash {
            components.push("");
        }

        if components_path.len() > 0 {
            components.extend(components_path.split('/'));
        }

        let front_idx = 1;
        let back_idx = components.len();

        Self {
            components,
            front_idx,
            back_idx,
            has_trailing_slash,
        }
    }
}

impl<'a> Iterator for PathIterator<'a> {
    type Item = Path;

    fn next(&mut self) -> Option<Self::Item> {
        if self.front_idx > self.back_idx {
            return None;
        }

        let front_idx = self.front_idx;
        self.front_idx += 1;

        if front_idx == 1 {
            return Some(Path::root());
        }

        let mut path
            = self.components[0..front_idx].join("/");

        if self.has_trailing_slash {
            path.push('/');
        }

        Some(Path::from(path))
    }
}

impl<'a> DoubleEndedIterator for PathIterator<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.front_idx > self.back_idx {
            return None;
        }

        let back_idx = self.back_idx;
        self.back_idx -= 1;

        if back_idx == 1 {
            return Some(Path::from("/"));
        }

        let components
            = self.components[0..back_idx].join("/");

        Some(Path::from(components))
    }
}

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "bincode", derive(bincode_derive::Decode, bincode_derive::Encode))]
#[cfg_attr(feature = "serde", derive(serde_derive::Serialize, serde_derive::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Path {
    path: String,
}

impl Path {
    pub fn temp_dir_pattern(str: &str) -> std::io::Result<Path> {
        let index = str.find("<>")
            .unwrap();

        let before = &str[..index];
        let after = &str[index + 2..];

        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let mut dir = std::env::temp_dir().to_arca();
        let name = format!("{}{:032x}{}", before, nonce, after);

        dir.join_str(name);
        dir.fs_create_dir_all()?;

        Ok(dir)
    }

    pub fn temp_dir() -> std::io::Result<Path> {
        Self::temp_dir_pattern("temp-<>")
    }

    pub fn current_exe() -> std::io::Result<Path> {
        Ok(std::env::current_exe()?.to_arca())
    }

    pub fn current_dir() -> std::io::Result<Path> {
        Ok(std::env::current_dir()?.to_arca())
    }

    pub fn home_dir() -> Result<Path, std::env::VarError> {
        Ok(Path::from(std::env::var("HOME")?))
    }

    /** @deprecated Prefer Path::empty() */
    pub fn new() -> Self {
        Path {path: "".to_string()}
    }

    pub fn empty() -> Self {
        Path {path: "".to_string()}
    }

    pub fn root() -> Self {
        Path {path: "/".to_string()}
    }

    pub fn iter_path(&self) -> PathIterator {
        PathIterator::new(self)
    }

    pub fn dirname<'a>(&'a self) -> Option<Path> {
        let mut slice_len = self.path.len();
        if self.path.ends_with('/') {
            if self.path.len() > 1 {
                slice_len -= 1;
            } else {
                return None;
            }
        }

        let slice = &self.path[..slice_len];
        if let Some(last_slash) = slice.rfind('/') {
            if last_slash > 0 {
                return Some(Path::from(&slice[..last_slash]));
            } else {
                return Some(Path::from("/"));
            }
        }

        None
    }

    pub fn basename<'a>(&'a self) -> Option<&'a str> {
        let has_trailing_slash = self.path.ends_with('/');

        let initial_slice = if has_trailing_slash {
            &self.path[..self.path.len() - 1]
        } else {
            &self.path
        };

        let first_basename_char = initial_slice
            .rfind('/')
            .map(|i| i + 1)
            .unwrap_or(0);

        if first_basename_char < initial_slice.len() {
            Some(&initial_slice[first_basename_char..])
        } else {
            None
        }
    }

    pub fn extname<'a>(&'a self) -> Option<&'a str> {
        self.basename().and_then(|basename| {
            if let Some(mut last_dot) = basename.rfind('.') {
                if last_dot > 2 && &basename[last_dot - 2..] == ".d.ts" {
                    last_dot -= 2;
                }

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

    pub fn is_forward(&self) -> bool {
        self.is_relative() && !self.is_extern()
    }

    pub fn is_extern(&self) -> bool {
        self.path.starts_with("../") || self.path == ".."
    }

    pub fn fs_create_parent(&self) -> io::Result<&Self> {
        if let Some(parent) = self.dirname() {
            parent.fs_create_dir_all()?;
        }

        Ok(self)
    }

    pub fn fs_create_dir_all(&self) -> io::Result<&Self> {
        fs::create_dir_all(&self.path)?;
        Ok(self)
    }

    pub fn fs_create_dir(&self) -> io::Result<&Self> {
        fs::create_dir(&self.path)?;
        Ok(self)
    }

    pub fn fs_set_permissions(&self, permissions: fs::Permissions) -> io::Result<&Self> {
        fs::set_permissions(&self.path, permissions)?;
        Ok(self)
    }

    pub fn fs_metadata(&self) -> io::Result<fs::Metadata> {
        fs::metadata(&self.path)
    }

    pub fn fs_exists(&self) -> bool {
        self.fs_metadata().is_ok()
    }

    pub fn fs_is_file(&self) -> bool {
        self.fs_metadata().map(|m| m.is_file()).unwrap_or(false)
    }

    pub fn fs_is_dir(&self) -> bool {
        self.fs_metadata().map(|m| m.is_dir()).unwrap_or(false)
    }

    pub fn if_exists(&self) -> Option<Path> {
        if self.fs_exists() {
            Some(self.clone())
        } else {
            None
        }
    }

    pub fn if_file(&self) -> Option<Path> {
        if self.fs_is_file() {
            Some(self.clone())
        } else {
            None
        }
    }

    pub fn if_dir(&self) -> Option<Path> {
        if self.fs_is_dir() {
            Some(self.clone())
        } else {
            None
        }
    }

    pub fn fs_read(&self) -> io::Result<Vec<u8>> {
        fs::read(&self.to_path_buf())
    }

    pub fn fs_read_prealloc(&self) -> io::Result<Vec<u8>> {
        let metadata = self.fs_metadata()?;

        self.fs_read_with_size(metadata.len())
    }

    pub fn fs_read_with_size(&self, size: u64) -> io::Result<Vec<u8>> {
        let mut data = Vec::with_capacity(size as usize);

        let mut file = std::fs::File::open(&self.to_path_buf())?;
        file.read_to_end(&mut data)?;

        Ok(data)
    }

    pub fn fs_read_text(&self) -> io::Result<String> {
        fs::read_to_string(self.to_path_buf())
    }

    pub fn fs_read_text_prealloc(&self) -> io::Result<String> {
        let metadata = self.fs_metadata()?;

        self.fs_read_text_with_size(metadata.len())
    }

    pub fn fs_read_text_with_size(&self, size: u64) -> io::Result<String> {
        let mut data = String::with_capacity(size as usize);

        let mut file = std::fs::File::open(&self.to_path_buf())?;
        file.read_to_string(&mut data)?;

        Ok(data)
    }

    #[cfg(feature = "tokio")]
    pub async fn fs_read_text_async(&self) -> io::Result<String> {
        tokio::fs::read_to_string(self.to_path_buf()).await
    }

    pub fn fs_read_dir(&self) -> io::Result<ReadDir> {
        fs::read_dir(&self.to_path_buf())
    }

    pub fn fs_write<T: AsRef<[u8]>>(&self, data: T) -> io::Result<&Self> {
        fs::write(self.to_path_buf(), data)?;
        Ok(self)
    }

    pub fn fs_write_text<T: AsRef<str>>(&self, text: T) -> io::Result<&Self> {
        fs::write(self.to_path_buf(), text.as_ref())?;
        Ok(self)
    }

    pub fn fs_expect<T: AsRef<[u8]>>(&self, data: T, permissions: fs::Permissions) -> Result<&Self, ImmutableErr> {
        let path_buf = self.to_path_buf();

        let update_content = std::fs::read(&path_buf)
            .ok_missing()
            .map(|current| current.map(|current| current.ne(data.as_ref())).unwrap_or(true))
            .map_err(ImmutableErr::Io)?;

        if update_content {
            return Err(ImmutableErr::Immutable);
        }

        let update_permissions = update_content ||
            std::fs::metadata(&path_buf).map_err(ImmutableErr::Io)?.permissions() != permissions;

        if update_permissions {
            return Err(ImmutableErr::Immutable);
        }

        Ok(self)
    }

    pub fn fs_change<T: AsRef<[u8]>>(&self, data: T, permissions: fs::Permissions) -> io::Result<&Self> {
        let path_buf = self.to_path_buf();

        let update_content = std::fs::read(&path_buf)
            .ok_missing()
            .map(|current| current.map(|current| current.ne(data.as_ref())).unwrap_or(true))?;

        if update_content {
            std::fs::write(&path_buf, data)?;
        }

        let update_permissions = update_content ||
            std::fs::metadata(&path_buf)?.permissions() != permissions;

        if update_permissions {
            std::fs::set_permissions(&path_buf, permissions)?;
        }

        Ok(self)
    }

    pub fn fs_rename(&self, new_path: &Path) -> io::Result<&Self> {
        fs::rename(self.to_path_buf(), new_path.to_path_buf())?;
        Ok(self)
    }

    pub fn fs_rm_file(&self) -> io::Result<&Self> {
        fs::remove_file(self.to_path_buf())?;
        Ok(self)
    }

    pub fn fs_rm(&self) -> io::Result<&Self> {
        match self.fs_is_dir() {
            true => fs::remove_dir_all(self.to_path_buf()),
            false => fs::remove_file(self.to_path_buf()),
        }?;

        Ok(self)
    }

    pub fn without_ext(&self) -> Path {
        self.with_ext("")
    }

    pub fn with_ext(&self, ext: &str) -> Path {
        let mut copy = self.clone();
        copy.set_ext(ext);
        copy
    }

    pub fn set_ext(&mut self, ext: &str) -> &mut Self {
        let has_trailing_slash = self.path.ends_with('/');

        let initial_slice = if has_trailing_slash {
            &self.path[..self.path.len() - 1]
        } else {
            &self.path
        };

        let first_basename_char = initial_slice
            .rfind('/')
            .map(|i| i + 1)
            .unwrap_or(0);

        let mut ext_char = self.path[first_basename_char..]
            .rfind('.')
            .map(|i| i + first_basename_char)
            .unwrap_or(initial_slice.len());

        if ext_char == first_basename_char {
            ext_char = self.path.len();
        }

        if ext_char > 2 && &self.path[ext_char - 2..] == ".d.ts" {
            ext_char -= 2;
        }

        let mut copy = self.path[..ext_char].to_string();
        copy.push_str(ext);

        if has_trailing_slash {
            copy.push('/');
        }

        self.path = copy;
        self
    }

    pub fn with_join(&self, other: &Path) -> Path {
        let mut copy = self.clone();
        copy.join(other);
        copy
    }

    pub fn with_join_str<T>(&self, other: T) -> Path
    where
        T: AsRef<str>,
    {
        let mut copy = self.clone();
        copy.join_str(other);
        copy
    }

    pub fn join(&mut self, other: &Path) -> &mut Self {
        if !other.path.is_empty() {
            if self.path.is_empty() || other.is_absolute() {
                self.path = other.path.clone();
            } else {
                if !self.path.ends_with('/') {
                    self.path.push('/');
                }
                self.path.push_str(&other.path);
                self.normalize();
            }
        }

        self
    }

    pub fn join_str<T>(&mut self, other: T) -> &mut Self
    where
        T: AsRef<str>,
    {
        self.join(&Path::from(other.as_ref()))
    }

    pub fn contains(&self, other: &Path) -> bool {
        other.as_str().starts_with(self.as_str()) || other == self
    }

    pub fn relative_to(&self, other: &Path) -> Path {
        assert!(self.is_absolute());
        assert!(other.is_absolute());

        let ends_with_slash = self.path.ends_with('/');

        let self_components: Vec<&str> = self.path.trim_end_matches('/').split('/').collect();
        let other_components: Vec<&str> = other.path.trim_end_matches('/').split('/').collect();

        let common_prefix_length = self_components.iter()
            .zip(other_components.iter())
            .take_while(|(a, b)| a == b)
            .count();

        let mut relative_path = vec![];

        for _ in common_prefix_length..other_components.len() {
            if other_components[common_prefix_length..].len() > 0 {
                relative_path.push("..");
            }
        }

        for component in self_components[common_prefix_length..].iter() {
            relative_path.push(*component);
        }

        if ends_with_slash {
            relative_path.push("");
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

impl FromStr for Path {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Path::from(s))
    }
}

impl<T: AsRef<str>> From<T> for Path {
    fn from(path: T) -> Self {
        Path {
            path: resolve_path(path.as_ref()),
        }
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path)
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
    fn test_join() {
        assert_eq!(Path::from("/usr/local").with_join(&Path::from("bin")), Path::from("/usr/local/bin"));
        assert_eq!(Path::from("/usr/local").with_join(&Path::from("bin/")), Path::from("/usr/local/bin/"));
        assert_eq!(Path::from("/usr/local/").with_join(&Path::from("bin")), Path::from("/usr/local/bin"));
        assert_eq!(Path::from("/usr/local/").with_join(&Path::from("bin/")), Path::from("/usr/local/bin/"));
        assert_eq!(Path::from("/usr/local").with_join(&Path::from("/bin")), Path::from("/bin"));
        assert_eq!(Path::from("usr/local").with_join(&Path::from("bin")), Path::from("usr/local/bin"));
        assert_eq!(Path::from("usr/local").with_join(&Path::from("bin/")), Path::from("usr/local/bin/"));
        assert_eq!(Path::new().with_join(&Path::from("bin")), Path::from("bin"));
    }

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
        assert_eq!(path2.relative_to(&path1), Path::from(""));
    }

    #[test]
    fn test_subdirectory() {
        let path1 = Path { path: "/home/user/docs".to_string() };
        let path2 = Path { path: "/home/user/docs/reports".to_string() };
        assert_eq!(path2.relative_to(&path1), Path::from("reports"));
    }

    #[test]
    fn test_subdirectory_trailing_slash() {
        let path1 = Path { path: "/home/user/docs/".to_string() };
        let path2 = Path { path: "/home/user/docs/reports".to_string() };
        assert_eq!(path2.relative_to(&path1), Path::from("reports"));
    }

    #[test]
    fn test_subdirectory_trailing_slash_subject() {
        let path1 = Path { path: "/home/user/docs".to_string() };
        let path2 = Path { path: "/home/user/docs/reports/".to_string() };
        assert_eq!(path2.relative_to(&path1), Path::from("reports/"));
    }

    #[test]
    fn test_parent_directory() {
        let path1 = Path { path: "/home/user/docs/reports".to_string() };
        let path2 = Path { path: "/home/user/docs".to_string() };
        assert_eq!(path2.relative_to(&path1), Path::from(".."));
    }

    #[test]
    fn test_different_directory() {
        let path1 = Path { path: "/home/user/docs".to_string() };
        let path2 = Path { path: "/home/user/music".to_string() };
        assert_eq!(path2.relative_to(&path1), Path::from("../music"));
    }

    #[test]
    fn test_different_root() {
        let path1 = Path { path: "/home/user/docs".to_string() };
        let path2 = Path { path: "/var/log".to_string() };
        assert_eq!(path2.relative_to(&path1), Path::from("../../../var/log"));
    }

    #[test]
    #[should_panic]
    fn test_relative_path() {
        let path1 = Path { path: "/home/user/docs".to_string() };
        let path2 = Path { path: "var/log".to_string() };
        path2.relative_to(&path1);
    }

    #[test]
    fn test_dirname_with_extension() {
        let path = Path { path: "/usr/local/bin/test.txt".to_string() };
        assert_eq!(path.dirname(), Some(Path::from("/usr/local/bin")));
    }

    #[test]
    fn test_dirname_without_extension() {
        let path = Path { path: "/usr/local/bin/test".to_string() };
        assert_eq!(path.dirname(), Some(Path::from("/usr/local/bin")));
    }

    #[test]
    fn test_dirname_with_trailing_slash() {
        let path = Path { path: "/usr/local/bin/".to_string() };
        assert_eq!(path.dirname(), Some(Path::from("/usr/local")));
    }

    #[test]
    fn test_dirname_with_single_slash() {
        let path = Path { path: "/".to_string() };
        assert_eq!(path.dirname(), None);
    }

    #[test]
    fn test_dirname_with_root_folder() {
        let path = Path { path: "/usr".to_string() };
        assert_eq!(path.dirname(), Some(Path::from("/")));
    }

    #[test]
    fn test_dirname_with_empty_string() {
        let path = Path { path: "".to_string() };
        assert_eq!(path.dirname(), None);
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
    fn test_basename_with_relative() {
        let path = Path { path: "foo".to_string() };
        assert_eq!(path.basename(), Some("foo"));
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
    fn test_extname_with_d_ts() {
        let path = Path { path: "/usr/local/bin/foo.d.ts".to_string() };
        assert_eq!(path.extname(), Some(".d.ts"));
    }

    #[test]
    fn test_extname_with_d_ts_out_of_range() {
        let path = Path { path: "x.ts".to_string() };
        assert_eq!(path.extname(), Some(".ts"));
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

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_serialization() {
        let path = Path::from("/usr/local/bin/test.txt");
        let serialized = serde_json::to_string(&path).unwrap();
        assert_eq!(serialized, "\"/usr/local/bin/test.txt\"");

        let deserialized: Path = serde_json::from_str(&serialized).unwrap();
        assert_eq!(path, deserialized);
    }

    #[test]
    fn test_set_ext_with_extension() {
        let mut path = Path { path: "/usr/local/bin/test.txt".to_string() };
        path.set_ext(".log");
        assert_eq!(path.as_str(), "/usr/local/bin/test.log");
    }

    #[test]
    fn test_set_ext_without_extension() {
        let mut path = Path { path: "/usr/local/bin/test".to_string() };
        path.set_ext(".log");
        assert_eq!(path.as_str(), "/usr/local/bin/test.log");
    }

    #[test]
    fn test_set_ext_with_empty_extension() {
        let mut path = Path { path: "/usr/local/bin/test.txt".to_string() };
        path.set_ext("");
        assert_eq!(path.as_str(), "/usr/local/bin/test");
    }

    #[test]
    fn test_set_ext_with_dot_extension() {
        let mut path = Path { path: "/usr/local/bin/test.txt".to_string() };
        path.set_ext(".");
        assert_eq!(path.as_str(), "/usr/local/bin/test.");
    }

    #[test]
    fn test_set_ext_with_dot_basename() {
        let mut path = Path { path: "/usr/local/bin/.htaccess".to_string() };
        path.set_ext(".log");
        assert_eq!(path.as_str(), "/usr/local/bin/.htaccess.log");
    }

    #[test]
    fn test_set_ext_with_no_extension() {
        let mut path = Path { path: "/usr/local/bin/".to_string() };
        path.set_ext(".log");
        assert_eq!(path.as_str(), "/usr/local/bin.log/");
    }

    #[test]
    fn test_set_ext_with_d_ts() {
        let mut path = Path { path: "/usr/local/bin/foo.d.ts".to_string() };
        path.set_ext(".log");
        assert_eq!(path.as_str(), "/usr/local/bin/foo.log");
    }

    #[test]
    fn test_set_ext_with_d_ts_out_of_range() {
        let mut path = Path { path: "x.ts".to_string() };
        path.set_ext(".log");
        assert_eq!(path.as_str(), "x.log");
    }

    #[test]
    fn test_set_ext_relative() {
        let mut path = Path { path: "test.txt".to_string() };
        path.set_ext(".log");
        assert_eq!(path.as_str(), "test.log");
    }

    #[test]
    fn test_iter_root() {
        let path = Path::root();
        let mut iter = path.iter_path();

        assert_eq!(iter.next(), Some(Path::from("/")));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_iter_path() {
        let path = Path::from("/usr/local/bin/test.txt");
        let mut iter = path.iter_path();

        assert_eq!(iter.next(), Some(Path::from("/")));
        assert_eq!(iter.next(), Some(Path::from("/usr")));
        assert_eq!(iter.next(), Some(Path::from("/usr/local")));
        assert_eq!(iter.next(), Some(Path::from("/usr/local/bin")));
        assert_eq!(iter.next(), Some(Path::from("/usr/local/bin/test.txt")));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_iter_trailing() {
        let path = Path::from("/usr/local/bin/");
        let mut iter = path.iter_path();

        assert_eq!(iter.next(), Some(Path::from("/")));
        assert_eq!(iter.next(), Some(Path::from("/usr/")));
        assert_eq!(iter.next(), Some(Path::from("/usr/local/")));
        assert_eq!(iter.next(), Some(Path::from("/usr/local/bin/")));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_iter_path_rev() {
        let path = Path::from("/usr/local/bin/test.txt");
        let mut iter = path.iter_path().rev();

        assert_eq!(iter.next(), Some(Path::from("/usr/local/bin/test.txt")));
        assert_eq!(iter.next(), Some(Path::from("/usr/local/bin")));
        assert_eq!(iter.next(), Some(Path::from("/usr/local")));
        assert_eq!(iter.next(), Some(Path::from("/usr")));
        assert_eq!(iter.next(), Some(Path::from("/")));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_relative_to_root() {
        let root = Path { path: "/".to_string() };
        
        // Test single file at root
        let path1 = Path { path: "/file.txt".to_string() };
        assert_eq!(path1.relative_to(&root), Path::from("file.txt"));
        
        // Test directory at root
        let path2 = Path { path: "/usr".to_string() };
        assert_eq!(path2.relative_to(&root), Path::from("usr"));
        
        // Test nested path
        let path3 = Path { path: "/usr/local/bin".to_string() };
        assert_eq!(path3.relative_to(&root), Path::from("usr/local/bin"));
        
        // Test path with trailing slash
        let path4 = Path { path: "/usr/local/".to_string() };
        assert_eq!(path4.relative_to(&root), Path::from("usr/local/"));
        
        // Root relative to root should be empty path
        assert_eq!(root.relative_to(&root), Path::from("."));
    }

    #[test]
    fn test_relative_to_root_subject() {
        let path1 = Path { path: "/usr/local/bin".to_string() };
        assert_eq!(Path::root().relative_to(&path1), Path::from("../../../"));
    }
    
}

