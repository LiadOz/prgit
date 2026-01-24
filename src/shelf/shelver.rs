use std::path::PathBuf;
use p4rs::{P4, P4Error};
use git2::Repository;
use crate::shelf::shelve_client::ShelveClient;
pub struct Shelver {
    shelve_client: ShelveClient,
    repo: Repository,
    commit_hash: String,
    base_commit_hash: String,
}



impl Shelver {
    pub fn new(p4: P4, client_name: String, client_root: PathBuf, repo: Repository, commit_hash: String, base_commit_hash: String) -> Result<Self, P4Error> {
        let shelve_client = ShelveClient::new(p4, &client_name, client_root)?;
        Ok(Self {
            shelve_client,
            repo,
            commit_hash,
            base_commit_hash,
        })
    }

    //pub fn shelve(&self) -> Result<(), P4Error> {
    //    self.shelve_client.run()
    //}
}