use chrono::{DateTime, NaiveDateTime, Utc};
use serde::Deserialize;

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
        Self { base: Some(base), ..Default::default() }
    }

    pub fn text() -> Self { Self::new(BaseFileType::Text) }
    pub fn binary() -> Self { Self::new(BaseFileType::Binary) }
    pub fn symlink() -> Self { Self::new(BaseFileType::Symlink) }
    pub fn unicode() -> Self { Self::new(BaseFileType::Unicode) }
    pub fn utf8() -> Self { Self::new(BaseFileType::Utf8) }
    pub fn utf16() -> Self { Self::new(BaseFileType::Utf16) }

    pub fn writable(mut self) -> Self { self.writable = true; self }
    pub fn executable(mut self) -> Self { self.executable = true; self }
    pub fn keyword_expansion(mut self) -> Self { self.keyword_expansion = true; self }
    pub fn exclusive_lock(mut self) -> Self { self.exclusive_lock = true; self }
    pub fn full_revisions(mut self) -> Self { self.full_revisions = true; self }
    pub fn compressed(mut self) -> Self { self.compressed = true; self }
    pub fn rcs_deltas(mut self) -> Self { self.rcs_deltas = true; self }
}

impl std::fmt::Display for FileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(base) = &self.base {
            write!(f, "{}", base)?;
        }
        if self.writable { write!(f, "+w")?; }
        if self.executable { write!(f, "+x")?; }
        if self.keyword_expansion { write!(f, "+k")?; }
        if self.exclusive_lock { write!(f, "+l")?; }
        if self.full_revisions { write!(f, "+F")?; }
        if self.compressed { write!(f, "+C")?; }
        if self.rcs_deltas { write!(f, "+D")?; }
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