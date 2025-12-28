use thiserror::Error;

#[derive(Error, Debug)]
pub enum MirrorError {
    #[error("P4 error: {0}")]
    P4(#[from] p4rs::P4Error),
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),
}