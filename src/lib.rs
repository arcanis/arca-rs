use std::fmt::{Debug, Formatter};

pub mod path;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Path {
    path: String,
}

impl Path {
    pub fn new() -> Self {
        Path::from("")
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
}
