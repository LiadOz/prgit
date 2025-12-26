mod commands;
mod error;
mod p4;

pub use error::P4Error;
pub use p4::P4;

pub use commands::P4Command;

pub use commands::change::{ChangeSpec, ChangeType};
pub use commands::changes::ChangeData;
pub use commands::edit::{EditAction, EditResult};
pub use commands::info::InfoResponse;
pub use commands::opened::{OpenAction, OpenedFile};
pub use commands::revert::RevertResult;

pub use commands::types::{BaseFileType, ChangeStatus, FileType};

#[cfg(feature = "extensible")]
pub mod extensible {
    pub use crate::commands::process::{CmdType, P4Process};
}
