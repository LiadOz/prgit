use std::path::PathBuf;

use crate::mirror::IntegrateStrategy;

pub trait Table {
    const SCHEMA: &'static str;
}

#[derive(Debug, Clone)]
pub struct PrgitClientInfo {
    pub id: u64,
    pub client_name: String,
    pub p4_path: PathBuf,
    pub p4port: String,
    pub p4user: String,
}

impl Table for PrgitClientInfo {
    const SCHEMA: &'static str = "
        CREATE TABLE IF NOT EXISTS prgit_clients (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            client_name TEXT NOT NULL UNIQUE,
            p4_path TEXT NOT NULL,
            p4port TEXT NOT NULL,
            p4user TEXT NOT NULL
        );
    ";
}

// #[derive(Debug, Clone)]
// pub struct ClientViews {
//     pub id: u64,
//     pub prgit_client_id: u64,
//     pub depot: String,
//     pub client: String,
// }
// 
// impl Table for ClientViews {
//     const SCHEMA: &'static str = "
//         CREATE TABLE IF NOT EXISTS client_views (
//             id INTEGER PRIMARY KEY AUTOINCREMENT,
//             prgit_client_id INTEGER NOT NULL REFERENCES prgit_clients(id),
//             depot TEXT NOT NULL,
//             client TEXT NOT NULL
//         );
//     ";
// }

#[derive(Debug, Clone)]
pub struct PrgitRepo {
    pub id: u64,
    pub prgit_client_id: u64,
    pub repo_path: PathBuf,
    pub synced_branch: String,
    pub last_sync_change: usize,
    pub integrate_strategy: IntegrateStrategy,
    pub max_changes_query: Option<usize>,
}

impl Table for PrgitRepo {
    const SCHEMA: &'static str = "
        CREATE TABLE IF NOT EXISTS prgit_repos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            prgit_client_id INTEGER NOT NULL REFERENCES prgit_clients(id),
            repo_path TEXT NOT NULL,
            synced_branch TEXT NOT NULL DEFAULT 'master',
            last_sync_change INTEGER NOT NULL DEFAULT 0,
            integrate_strategy INTEGER NOT NULL,
            max_changes_query INTEGER
        );
    ";
}

#[derive(Debug, Clone)]
pub struct CommitChangeMapping {
    pub prgit_client_id: u64,
    pub change: usize,
    pub commit_hash: String,
}

impl Table for CommitChangeMapping {
    const SCHEMA: &'static str = "
        CREATE TABLE IF NOT EXISTS commit_change_mapping (
            prgit_client_id INTEGER NOT NULL REFERENCES prgit_clients(id),
            change INTEGER NOT NULL,
            commit_hash TEXT NOT NULL,
            PRIMARY KEY (prgit_client_id, change)
        );
    ";
}

#[derive(Debug, Clone)]
pub struct ShelveConfig {
    pub prgit_client_id: u64,
    pub clients_root: PathBuf,
}

impl Table for ShelveConfig {
    const SCHEMA: &'static str = "
        CREATE TABLE IF NOT EXISTS shelve_config (
            prgit_client_id INTEGER PRIMARY KEY REFERENCES prgit_clients(id),
            clients_root TEXT NOT NULL
        );
    ";
}

#[derive(Debug, Clone)]
pub struct BranchMapping {
    pub prgit_client_id: u64,
    pub change: usize,
    pub branch: String,
}

impl Table for BranchMapping {
    const SCHEMA: &'static str = "
        CREATE TABLE IF NOT EXISTS branch_mapping (
            prgit_client_id INTEGER NOT NULL REFERENCES prgit_clients(id),
            change INTEGER NOT NULL,
            branch TEXT NOT NULL,
            PRIMARY KEY (prgit_client_id, change)
        );
    ";
}

#[derive(Debug, Clone)]
pub struct UserMapping {
    pub user: String,
    pub email: String,
}

impl Table for UserMapping {
    const SCHEMA: &'static str = "
        CREATE TABLE IF NOT EXISTS user_mapping (
            user TEXT PRIMARY KEY,
            email TEXT NOT NULL
        );
    ";
}

#[derive(Debug, Clone)]
pub struct BranchShelveMapping {
    pub prgit_client_id: u64,
    pub branch: String,
    pub shelved_change: usize,
}

impl Table for BranchShelveMapping {
    const SCHEMA: &'static str = "
        CREATE TABLE IF NOT EXISTS branch_shelve_mapping (
            prgit_client_id INTEGER NOT NULL REFERENCES prgit_clients(id),
            branch TEXT NOT NULL,
            shelved_change INTEGER NOT NULL,
            PRIMARY KEY (prgit_client_id, branch)
        );
    ";
}
