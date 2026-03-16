mod client_pool;
mod shelve_client;
mod shelver;

pub use client_pool::{get_shelve_client, ShelveClientError, ShelveClientHandle};
pub use shelve_client::{FileAction, FileChange, ShelveClient};
pub use shelver::{PendingShelve, Shelver, ShelverError};
