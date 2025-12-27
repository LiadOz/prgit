use chrono::{DateTime, NaiveDateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;
use std::marker::PhantomData;

#[derive(Deserialize, Debug)]
pub struct GenericResponse {
    pub data: String,
    pub level: usize,
}

/// P4 outputs list fields as numbered keys (e.g. `Files0`, `Files1`, `View0`, `View1`)
/// instead of arrays. `NumberedVec` deserializes these into a `Vec<T>` transparently.
///
/// Usage with `#[serde(flatten)]`:
/// ```ignore
/// #[derive(Deserialize)]
/// struct ChangeSpec {
///     #[serde(flatten)]
///     pub files: NumberedVec<FilesPrefix>,
/// }
/// ```
///
/// The type derefs to `Vec<T>`, so it can be used exactly like a vector.
pub trait Prefix {
    const VALUE: &'static str;
}

macro_rules! define_prefix {
    ($name:ident, $prefix:literal) => {
        #[derive(Debug, Default)]
        pub struct $name;
        impl Prefix for $name {
            const VALUE: &'static str = $prefix;
        }
    };
}

define_prefix!(FilesPrefix, "Files");
define_prefix!(ViewPrefix, "View");

#[derive(Debug, Default)]
pub struct NumberedVec<P: Prefix, T: std::str::FromStr = String>(Vec<T>, PhantomData<P>);

impl<P: Prefix, T: std::str::FromStr> NumberedVec<P, T> {
    pub fn new(v: Vec<T>) -> Self {
        Self(v, PhantomData)
    }
}

impl<P: Prefix, T: std::str::FromStr> std::ops::Deref for NumberedVec<P, T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<P: Prefix, T: std::str::FromStr> std::ops::DerefMut for NumberedVec<P, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<P: Prefix, T: std::str::FromStr> From<Vec<T>> for NumberedVec<P, T> {
    fn from(v: Vec<T>) -> Self {
        Self(v, PhantomData)
    }
}

impl<P: Prefix, T: std::str::FromStr + PartialEq> PartialEq<Vec<T>> for NumberedVec<P, T> {
    fn eq(&self, other: &Vec<T>) -> bool {
        self.0 == *other
    }
}

impl<'de, P: Prefix, T: std::str::FromStr> Deserialize<'de> for NumberedVec<P, T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let map: HashMap<String, String> = HashMap::deserialize(deserializer)?;
        let mut items: Vec<_> = map
            .into_iter()
            .filter_map(|(k, v)| {
                k.strip_prefix(P::VALUE)?
                    .parse::<usize>()
                    .ok()
                    .map(|i| (i, v))
            })
            .collect();
        items.sort_by_key(|(i, _)| *i);
        Ok(NumberedVec(
            items
                .into_iter()
                .filter_map(|(_, v)| v.parse().ok())
                .collect(),
            PhantomData,
        ))
    }
}

pub fn deserialize_p4_date<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    NaiveDateTime::parse_from_str(&s, "%Y/%m/%d %H:%M:%S")
        .map(|dt| dt.and_utc())
        .map_err(serde::de::Error::custom)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BaseFileType {
    Text,
    Binary,
    Symlink,
    Apple,
    Resource,
    Unicode,
    Utf8,
    Utf16,
}

impl std::fmt::Display for BaseFileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BaseFileType::Text => write!(f, "text"),
            BaseFileType::Binary => write!(f, "binary"),
            BaseFileType::Symlink => write!(f, "symlink"),
            BaseFileType::Apple => write!(f, "apple"),
            BaseFileType::Resource => write!(f, "resource"),
            BaseFileType::Unicode => write!(f, "unicode"),
            BaseFileType::Utf8 => write!(f, "utf8"),
            BaseFileType::Utf16 => write!(f, "utf16"),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FileType {
    pub base: Option<BaseFileType>,
    pub writable: bool,
    pub executable: bool,
    pub keyword_expansion: bool,
    pub exclusive_lock: bool,
    pub full_revisions: bool,
    pub compressed: bool,
    pub rcs_deltas: bool,
}

impl FileType {
    pub fn new(base: BaseFileType) -> Self {
        Self {
            base: Some(base),
            ..Default::default()
        }
    }

    pub fn text() -> Self {
        Self::new(BaseFileType::Text)
    }
    pub fn binary() -> Self {
        Self::new(BaseFileType::Binary)
    }
    pub fn symlink() -> Self {
        Self::new(BaseFileType::Symlink)
    }
    pub fn unicode() -> Self {
        Self::new(BaseFileType::Unicode)
    }
    pub fn utf8() -> Self {
        Self::new(BaseFileType::Utf8)
    }
    pub fn utf16() -> Self {
        Self::new(BaseFileType::Utf16)
    }

    pub fn writable(mut self) -> Self {
        self.writable = true;
        self
    }
    pub fn executable(mut self) -> Self {
        self.executable = true;
        self
    }
    pub fn keyword_expansion(mut self) -> Self {
        self.keyword_expansion = true;
        self
    }
    pub fn exclusive_lock(mut self) -> Self {
        self.exclusive_lock = true;
        self
    }
    pub fn full_revisions(mut self) -> Self {
        self.full_revisions = true;
        self
    }
    pub fn compressed(mut self) -> Self {
        self.compressed = true;
        self
    }
    pub fn rcs_deltas(mut self) -> Self {
        self.rcs_deltas = true;
        self
    }
}

impl std::fmt::Display for FileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(base) = &self.base {
            write!(f, "{}", base)?;
        }
        if self.writable {
            write!(f, "+w")?;
        }
        if self.executable {
            write!(f, "+x")?;
        }
        if self.keyword_expansion {
            write!(f, "+k")?;
        }
        if self.exclusive_lock {
            write!(f, "+l")?;
        }
        if self.full_revisions {
            write!(f, "+F")?;
        }
        if self.compressed {
            write!(f, "+C")?;
        }
        if self.rcs_deltas {
            write!(f, "+D")?;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChangeStatus {
    Pending,
    Submitted,
    Shelved,
}

impl std::fmt::Display for ChangeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ChangeStatus::Pending => "pending",
            ChangeStatus::Submitted => "submitted",
            ChangeStatus::Shelved => "shelved",
        };
        f.write_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_file_type_display() {
        assert_eq!(BaseFileType::Text.to_string(), "text");
        assert_eq!(BaseFileType::Binary.to_string(), "binary");
        assert_eq!(BaseFileType::Symlink.to_string(), "symlink");
        assert_eq!(BaseFileType::Unicode.to_string(), "unicode");
    }

    #[test]
    fn test_file_type_display() {
        assert_eq!(FileType::text().to_string(), "text");
        assert_eq!(FileType::binary().to_string(), "binary");
        assert_eq!(FileType::text().writable().to_string(), "text+w");
        assert_eq!(FileType::text().executable().to_string(), "text+x");
        assert_eq!(
            FileType::binary().writable().executable().to_string(),
            "binary+w+x"
        );
    }

    #[test]
    fn test_file_type_all_modifiers() {
        let ft = FileType::text()
            .writable()
            .executable()
            .keyword_expansion()
            .exclusive_lock()
            .full_revisions()
            .compressed()
            .rcs_deltas();
        assert_eq!(ft.to_string(), "text+w+x+k+l+F+C+D");
    }

    #[test]
    fn test_change_status_display() {
        assert_eq!(ChangeStatus::Pending.to_string(), "pending");
        assert_eq!(ChangeStatus::Submitted.to_string(), "submitted");
        assert_eq!(ChangeStatus::Shelved.to_string(), "shelved");
    }

    define_prefix!(TestPrefix, "Num");

    #[test]
    fn test_numbered_vec_with_usize() {
        let json = r#"{"Num0": "42", "Num1": "100", "Num2": "7"}"#;
        let nv: NumberedVec<TestPrefix, usize> = serde_json::from_str(json).unwrap();
        assert_eq!(nv.len(), 3);
        assert_eq!(nv[0], 42);
        assert_eq!(nv[1], 100);
        assert_eq!(nv[2], 7);
    }

    #[test]
    fn test_numbered_vec_with_strings() {
        let json = r#"{"Files0": "a.txt", "Files1": "b.txt"}"#;
        let nv: NumberedVec<FilesPrefix> = serde_json::from_str(json).unwrap();
        assert_eq!(nv.len(), 2);
        assert_eq!(nv[0], "a.txt");
        assert_eq!(nv[1], "b.txt");
    }
}
