mod commands;
mod error;
mod p4;

pub use error::P4Error;
pub use p4::P4;

pub use commands::P4Command;

pub use commands::change::{ChangeSpec, ChangeType};
pub use commands::changes::ChangeData;
pub use commands::client::{ClientMapping, ClientSpec};
pub use commands::delete::DeleteResult;
pub use commands::describe::{DescribeFile, DescribeResult};
pub use commands::edit::{EditAction, EditResult};
pub use commands::files::FileInfo;
pub use commands::info::InfoResponse;
pub use commands::move_file::MoveResult;
pub use commands::opened::{OpenAction, OpenedFile};
pub use commands::print::{PrintFileInfo, PrintResult};
pub use commands::reopen::ReopenResult;
pub use commands::revert::RevertResult;
pub use commands::shelve::ShelveResult;
pub use commands::submit::SubmitResult;
pub use commands::sync::{SyncAction, SyncResult};
pub use commands::user::UserInfo;
pub use commands::where_cmd::WhereResult;

pub use commands::types::{
    BaseFileType, ChangeListType, ChangeStatus, FileAction, FileType, LineEnding,
};

#[cfg(feature = "extensible")]
pub mod extensible {
    pub use crate::commands::process::{CmdType, P4Process};
    pub use crate::commands::types::extract_numbered;
}

#[cfg(feature = "testkit")]
pub mod testkit;
