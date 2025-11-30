use super::SyncError;
use super::sync_pair::{GitPerforcePair, Action, Dirty, Sync};

struct SyncAction;

impl Action for SyncAction {
    type InputState = Dirty;
    type OutputState = Sync;

    fn run(self, _pair: GitPerforcePair<Self::InputState>) -> Result<GitPerforcePair<Self::OutputState>, SyncError> {
        return Err(SyncError::InvalidState("Not implemented".to_string()));
    }
}

struct RequireSync;
impl Action for RequireSync {
    type InputState = Sync;
    type OutputState = Sync;

    fn run(self, _pair: GitPerforcePair<Self::InputState>) -> Result<GitPerforcePair<Self::OutputState>, SyncError> {
        return Err(SyncError::InvalidState("Not implemented".to_string()));
    }
}

struct DirtyAction;

impl Action for DirtyAction {
    type InputState = Dirty;
    type OutputState = Dirty;

    fn run(self, _pair: GitPerforcePair<Self::InputState>) -> Result<GitPerforcePair<Self::OutputState>, SyncError> {
        return Err(SyncError::InvalidState("Not implemented".to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_action() {
        GitPerforcePair::new()
            .then(SyncAction).unwrap()
            .into_dirty()
            .then(DirtyAction).unwrap()
            .then(DirtyAction).unwrap();
    }
}