use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub enum IntegrateStrategy {
    MergeOurs,
}

impl IntegrateStrategy {
    pub fn from_db(_value: i64) -> Self {
        Self::MergeOurs
    }

    pub fn to_db(self) -> i64 {
        match self {
            Self::MergeOurs => 0,
        }
    }
}

pub trait MirrorData {
    fn last_sync_change(&self) -> usize;
    fn set_last_sync_change(&mut self, change: usize);
    fn get_related_branch(&self, change: usize) -> Option<String>;
    fn get_user_email(&self, user: &str) -> Option<String>;
    fn set_user_email(&mut self, user: &str, email: &str);
    fn p4_client(&self) -> &str;
    fn integrate_strategy(&self) -> IntegrateStrategy;
    fn max_changes_query(&self) -> Option<usize>;
    fn map_commit_to_change(&self, commit: &str, change: usize);
}

#[derive(Debug)]
pub struct HashMapMirrorData {
    last_sync_change: usize,
    branch_mapping: HashMap<usize, String>,
    user_mapping: HashMap<String, String>,
    p4_client: String,
    integrate_strategy: IntegrateStrategy,
    max_changes_query: Option<usize>,
}

impl HashMapMirrorData {
    pub fn new(
        p4_client: String,
        integrate_strategy: IntegrateStrategy,
        max_changes_query: Option<usize>,
    ) -> Self {
        Self {
            last_sync_change: 0,
            branch_mapping: HashMap::new(),
            user_mapping: HashMap::new(),
            p4_client,
            integrate_strategy,
            max_changes_query,
        }
    }

    pub fn set_branch_mapping(&mut self, change: usize, branch: String) {
        self.branch_mapping.insert(change, branch);
    }
}

impl MirrorData for HashMapMirrorData {
    fn last_sync_change(&self) -> usize {
        self.last_sync_change
    }

    fn set_last_sync_change(&mut self, change: usize) {
        self.last_sync_change = change;
    }

    fn get_related_branch(&self, change: usize) -> Option<String> {
        self.branch_mapping.get(&change).cloned()
    }

    fn get_user_email(&self, user: &str) -> Option<String> {
        self.user_mapping.get(user).cloned()
    }

    fn set_user_email(&mut self, user: &str, email: &str) {
        self.user_mapping
            .insert(user.to_string(), email.to_string());
    }

    fn p4_client(&self) -> &str {
        &self.p4_client
    }

    fn integrate_strategy(&self) -> IntegrateStrategy {
        self.integrate_strategy
    }

    fn max_changes_query(&self) -> Option<usize> {
        self.max_changes_query
    }

    fn map_commit_to_change(&self, _commit: &str, _change: usize) {
        // No-op for in-memory test data
    }
}
