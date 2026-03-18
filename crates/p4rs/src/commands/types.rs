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

pub struct NumberedFields<'a> {
    map: &'a HashMap<String, String>,
}

impl<'a> NumberedFields<'a> {
    pub fn new(map: &'a HashMap<String, String>) -> Self {
        Self { map }
    }

    pub fn at(&self, index: usize) -> IndexedField<'_> {
        IndexedField {
            map: self.map,
            index,
        }
    }

    pub fn map_each<T, F>(&self, primary_prefix: &str, f: F) -> Vec<T>
    where
        F: Fn(IndexedField<'_>) -> T,
    {
        (0..)
            .take_while(|i| self.map.contains_key(&format!("{}{}", primary_prefix, i)))
            .map(|i| f(self.at(i)))
            .collect()
    }
}

pub struct IndexedField<'a> {
    map: &'a HashMap<String, String>,
    index: usize,
}

impl<'a> IndexedField<'a> {
    pub fn get<T: std::str::FromStr>(&self, prefix: &str) -> Option<T> {
        self.map
            .get(&format!("{}{}", prefix, self.index))?
            .parse()
            .ok()
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

pub fn deserialize_unix_timestamp<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    let ts: i64 = s.parse().map_err(serde::de::Error::custom)?;
    DateTime::from_timestamp(ts, 0).ok_or_else(|| serde::de::Error::custom("invalid timestamp"))
}

pub fn deserialize_optional_rev<'de, D>(deserializer: D) -> Result<Option<usize>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    if s == "none" {
        Ok(None)
    } else {
        s.parse().map(Some).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeListType {
    Public,
    Restricted,
}

impl std::fmt::Display for ChangeListType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangeListType::Public => write!(f, "public"),
            ChangeListType::Restricted => write!(f, "restricted"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
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

impl std::str::FromStr for BaseFileType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "text" => Ok(BaseFileType::Text),
            "binary" => Ok(BaseFileType::Binary),
            "symlink" => Ok(BaseFileType::Symlink),
            "apple" => Ok(BaseFileType::Apple),
            "resource" => Ok(BaseFileType::Resource),
            "unicode" => Ok(BaseFileType::Unicode),
            "utf8" => Ok(BaseFileType::Utf8),
            "utf16" => Ok(BaseFileType::Utf16),
            _ => Err(format!("unknown base file type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileType {
    pub base: BaseFileType,
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
            base,
            writable: false,
            executable: false,
            keyword_expansion: false,
            exclusive_lock: false,
            full_revisions: false,
            compressed: false,
            rcs_deltas: false,
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
        write!(f, "{}", self.base)?;
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

impl std::str::FromStr for FileType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('+');
        let base_str = parts.next().ok_or("empty file type")?;
        let mut ft = FileType::new(base_str.parse()?);
        for modifier in parts {
            for c in modifier.chars() {
                match c {
                    'w' => ft.writable = true,
                    'x' => ft.executable = true,
                    'k' => ft.keyword_expansion = true,
                    'l' => ft.exclusive_lock = true,
                    'F' => ft.full_revisions = true,
                    'C' => ft.compressed = true,
                    'D' => ft.rcs_deltas = true,
                    _ => {}
                }
            }
        }
        Ok(ft)
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

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineEnding {
    Local,
    Unix,
    Mac,
    Win,
    Share,
}

impl std::fmt::Display for LineEnding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            LineEnding::Local => "local",
            LineEnding::Unix => "unix",
            LineEnding::Mac => "mac",
            LineEnding::Win => "win",
            LineEnding::Share => "share",
        };
        f.write_str(s)
    }
}

impl std::str::FromStr for LineEnding {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "local" => Ok(LineEnding::Local),
            "unix" => Ok(LineEnding::Unix),
            "mac" => Ok(LineEnding::Mac),
            "win" => Ok(LineEnding::Win),
            "share" => Ok(LineEnding::Share),
            _ => Err(format!("Unknown LineEnding: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileAction {
    Add,
    Edit,
    Delete,
    Branch,
    Integrate,
    #[serde(rename = "move/add")]
    MoveAdd,
    #[serde(rename = "move/delete")]
    MoveDelete,
}

impl std::fmt::Display for FileAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            FileAction::Add => "add",
            FileAction::Edit => "edit",
            FileAction::Delete => "delete",
            FileAction::Branch => "branch",
            FileAction::Integrate => "integrate",
            FileAction::MoveAdd => "move/add",
            FileAction::MoveDelete => "move/delete",
        };
        f.write_str(s)
    }
}

impl std::str::FromStr for FileAction {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "add" => Ok(FileAction::Add),
            "edit" => Ok(FileAction::Edit),
            "delete" => Ok(FileAction::Delete),
            "branch" => Ok(FileAction::Branch),
            "integrate" => Ok(FileAction::Integrate),
            "move/add" => Ok(FileAction::MoveAdd),
            "move/delete" => Ok(FileAction::MoveDelete),
            _ => Err(format!("unknown action: {}", s)),
        }
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
    fn test_base_file_type_from_str() {
        assert_eq!("text".parse::<BaseFileType>().unwrap(), BaseFileType::Text);
        assert_eq!(
            "binary".parse::<BaseFileType>().unwrap(),
            BaseFileType::Binary
        );
        assert_eq!(
            "symlink".parse::<BaseFileType>().unwrap(),
            BaseFileType::Symlink
        );
        assert_eq!(
            "unicode".parse::<BaseFileType>().unwrap(),
            BaseFileType::Unicode
        );
        assert_eq!("utf8".parse::<BaseFileType>().unwrap(), BaseFileType::Utf8);
        assert_eq!(
            "utf16".parse::<BaseFileType>().unwrap(),
            BaseFileType::Utf16
        );
        assert!("invalid".parse::<BaseFileType>().is_err());
    }

    #[test]
    fn test_file_type_from_str() {
        assert_eq!("text".parse::<FileType>().unwrap(), FileType::text());
        assert_eq!("binary".parse::<FileType>().unwrap(), FileType::binary());
        assert_eq!("symlink".parse::<FileType>().unwrap(), FileType::symlink());
        assert_eq!(
            "text+w".parse::<FileType>().unwrap(),
            FileType::text().writable()
        );
        assert_eq!(
            "binary+x".parse::<FileType>().unwrap(),
            FileType::binary().executable()
        );
        assert_eq!(
            "text+w+x+k+l+F+C+D".parse::<FileType>().unwrap(),
            FileType::text()
                .writable()
                .executable()
                .keyword_expansion()
                .exclusive_lock()
                .full_revisions()
                .compressed()
                .rcs_deltas()
        );
        assert_eq!(
            "symlink+l".parse::<FileType>().unwrap(),
            FileType::symlink().exclusive_lock()
        );
    }

    #[test]
    fn test_file_type_from_str_combined_modifiers() {
        // P4 outputs combined modifiers like "text+kx" (not "text+k+x")
        assert_eq!(
            "text+kx".parse::<FileType>().unwrap(),
            FileType::text().keyword_expansion().executable()
        );
        assert_eq!(
            "text+wx".parse::<FileType>().unwrap(),
            FileType::text().writable().executable()
        );
        assert_eq!(
            "text+xlk".parse::<FileType>().unwrap(),
            FileType::text()
                .executable()
                .exclusive_lock()
                .keyword_expansion()
        );
        // Mixed: some combined, some separate
        assert_eq!(
            "binary+kx+l".parse::<FileType>().unwrap(),
            FileType::binary()
                .keyword_expansion()
                .executable()
                .exclusive_lock()
        );
    }

    #[test]
    fn test_change_status_display() {
        assert_eq!(ChangeStatus::Pending.to_string(), "pending");
        assert_eq!(ChangeStatus::Submitted.to_string(), "submitted");
        assert_eq!(ChangeStatus::Shelved.to_string(), "shelved");
    }

    #[test]
    fn test_file_action_display() {
        assert_eq!(FileAction::Add.to_string(), "add");
        assert_eq!(FileAction::Edit.to_string(), "edit");
        assert_eq!(FileAction::Delete.to_string(), "delete");
        assert_eq!(FileAction::Branch.to_string(), "branch");
        assert_eq!(FileAction::Integrate.to_string(), "integrate");
        assert_eq!(FileAction::MoveAdd.to_string(), "move/add");
        assert_eq!(FileAction::MoveDelete.to_string(), "move/delete");
    }

    #[test]
    fn test_file_action_from_str() {
        assert_eq!("add".parse::<FileAction>().unwrap(), FileAction::Add);
        assert_eq!("edit".parse::<FileAction>().unwrap(), FileAction::Edit);
        assert_eq!("delete".parse::<FileAction>().unwrap(), FileAction::Delete);
        assert_eq!(
            "move/add".parse::<FileAction>().unwrap(),
            FileAction::MoveAdd
        );
        assert_eq!(
            "move/delete".parse::<FileAction>().unwrap(),
            FileAction::MoveDelete
        );
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

    #[test]
    fn test_numbered_fields_map_each() {
        let mut map = HashMap::new();
        map.insert("file0".to_string(), "a.txt".to_string());
        map.insert("rev0".to_string(), "1".to_string());
        map.insert("file1".to_string(), "b.txt".to_string());
        map.insert("rev1".to_string(), "2".to_string());

        let fields = NumberedFields::new(&map);
        let results: Vec<(String, usize)> =
            fields.map_each("file", |f| (f.get("file").unwrap(), f.get("rev").unwrap()));

        assert_eq!(results.len(), 2);
        assert_eq!(results[0], ("a.txt".to_string(), 1));
        assert_eq!(results[1], ("b.txt".to_string(), 2));
    }

    #[test]
    fn test_numbered_fields_map_each_empty() {
        let map: HashMap<String, String> = HashMap::new();
        let fields = NumberedFields::new(&map);
        let results: Vec<String> = fields.map_each("file", |f| f.get("file").unwrap_or_default());
        assert!(results.is_empty());
    }

    #[test]
    fn test_indexed_field_get_optional() {
        let mut map = HashMap::new();
        map.insert("name0".to_string(), "test".to_string());

        let fields = NumberedFields::new(&map);
        let result: Vec<(String, Option<usize>)> =
            fields.map_each("name", |f| (f.get("name").unwrap(), f.get("size")));

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "test");
        assert_eq!(result[0].1, None);
    }
}
