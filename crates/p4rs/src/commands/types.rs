use chrono::{DateTime, NaiveDateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
pub struct GenericResponse {
    pub data: String,
    pub level: usize,
}

/// Extracts numbered fields from a P4 response map into a sorted vector.
///
/// P4 outputs list fields as numbered keys (e.g. `Files0`, `Files1`, `View0`, `View1`)
/// instead of arrays. This function extracts all fields matching the given prefix,
/// parses them into the target type, and returns them sorted by index.
///
/// # Arguments
/// * `map` - The flattened extra fields from a P4 response
/// * `prefix` - The field name prefix to match (e.g. "Files", "View")
///
/// # Returns
/// A vector of parsed values, sorted by their numeric suffix. Values that fail
/// to parse are silently skipped.
#[cfg(feature = "extensible")]
pub fn extract_numbered<T>(map: &HashMap<String, String>, prefix: &str) -> Vec<T>
where
    T: std::str::FromStr,
{
    extract_numbered_inner(map, prefix)
}

#[cfg(not(feature = "extensible"))]
pub(crate) fn extract_numbered<T>(map: &HashMap<String, String>, prefix: &str) -> Vec<T>
where
    T: std::str::FromStr,
{
    extract_numbered_inner(map, prefix)
}

fn extract_numbered_inner<T>(map: &HashMap<String, String>, prefix: &str) -> Vec<T>
where
    T: std::str::FromStr,
{
    let mut v: Vec<(usize, T)> = map
        .iter()
        .filter_map(|(k, v)| {
            k.strip_prefix(prefix)
                .and_then(|n| n.parse::<usize>().ok())
                .and_then(|i| v.parse::<T>().ok().map(|val| (i, val)))
        })
        .collect();
    v.sort_by_key(|(i, _)| *i);
    v.into_iter().map(|(_, v)| v).collect()
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

    #[test]
    fn test_extract_numbered_strings() {
        let mut map = HashMap::new();
        map.insert("Files0".to_string(), "a.txt".to_string());
        map.insert("Files1".to_string(), "b.txt".to_string());
        map.insert("Files2".to_string(), "c.txt".to_string());
        map.insert("Other".to_string(), "ignored".to_string());
        let result: Vec<String> = extract_numbered(&map, "Files");
        assert_eq!(result, vec!["a.txt", "b.txt", "c.txt"]);
    }

    #[test]
    fn test_extract_numbered_usize() {
        let mut map = HashMap::new();
        map.insert("Num0".to_string(), "42".to_string());
        map.insert("Num1".to_string(), "100".to_string());
        map.insert("Num2".to_string(), "7".to_string());
        let result: Vec<usize> = extract_numbered(&map, "Num");
        assert_eq!(result, vec![42, 100, 7]);
    }

    #[test]
    fn test_extract_numbered_out_of_order() {
        let mut map = HashMap::new();
        map.insert("Item2".to_string(), "third".to_string());
        map.insert("Item0".to_string(), "first".to_string());
        map.insert("Item1".to_string(), "second".to_string());
        let result: Vec<String> = extract_numbered(&map, "Item");
        assert_eq!(result, vec!["first", "second", "third"]);
    }

    #[test]
    fn test_extract_numbered_empty() {
        let map: HashMap<String, String> = HashMap::new();
        let result: Vec<String> = extract_numbered(&map, "Files");
        assert!(result.is_empty());
    }

    #[test]
    fn test_extract_numbered_no_matches() {
        let mut map = HashMap::new();
        map.insert("Other0".to_string(), "value".to_string());
        let result: Vec<String> = extract_numbered(&map, "Files");
        assert!(result.is_empty());
    }

    #[test]
    fn test_extract_numbered_skips_invalid_parse() {
        let mut map = HashMap::new();
        map.insert("Num0".to_string(), "42".to_string());
        map.insert("Num1".to_string(), "not_a_number".to_string());
        map.insert("Num2".to_string(), "7".to_string());
        let result: Vec<usize> = extract_numbered(&map, "Num");
        assert_eq!(result, vec![42, 7]);
    }
}
