use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub enum IntegrateStrategy {
    MergeOurs,
    Rebase,
}

#[derive(Debug)]
pub struct MirrorData {
    last_sync_change: usize,
    branch_mapping: HashMap<usize, String>,
    user_mapping: HashMap<String, String>,
    pub p4_client: String,
    pub integrate_strategy: IntegrateStrategy,
    pub max_changes_query: Option<usize>,
}

impl MirrorData {
    pub fn new(p4_client: String, integrate_strategy: IntegrateStrategy, max_changes_query: Option<usize>) -> Self {
        Self {
            last_sync_change: 0,
            branch_mapping: HashMap::new(),
            user_mapping: HashMap::new(),
            p4_client,
            integrate_strategy,
            max_changes_query,
        }
    }
    pub fn last_sync_change(&self) -> usize {
        self.last_sync_change
    }

    pub fn set_last_sync_change(&mut self, change: usize) {
        self.last_sync_change = change;
    }

    pub fn get_related_branch(&self, change: usize) -> Option<&String> {
        self.branch_mapping.get(&change)
    }

    pub fn get_user_email(&self, user: &str) -> Option<&String> {
        self.user_mapping.get(user)
    }

    pub fn set_user_email(&mut self, user: &str, email: String) {
        self.user_mapping.insert(user.to_string(), email);
    }

}