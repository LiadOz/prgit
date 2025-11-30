
use super::SyncError;

pub struct GitPerforcePair<State = Dirty> {
    repository: git2::Repository,
    perforce_client: p4_cmd::P4,
    last_synced_cn: Option<(String, String)>,
    _marker: std::marker::PhantomData<State>,
}


pub struct Sync;
pub struct Dirty;

impl GitPerforcePair<Dirty> {
    pub fn new(repository: git2::Repository, perforce_client: p4_cmd::P4) -> Self {
        return Self {
            repository: repository,
            perforce_client: perforce_client,
            last_synced_cn: None, // later this will be determined by db or something
            _marker: std::marker::PhantomData::<Dirty>,
        };
    }

    /// Repository is synced if all changelists have a matching commit in the repository
    pub fn check_synced(self) -> Result<GitPerforcePair<Sync>, SyncError> {
        if true {
            return Err(SyncError::InvalidState("Not implemented".to_string()));
        }
        Ok(self.into_state::<Sync>())
    }

}

impl GitPerforcePair<Sync> {
    pub fn into_dirty(self) -> GitPerforcePair<Dirty> {
        return self.into_state::<Dirty>();
    }
}

impl<State> GitPerforcePair<State> {
    fn into_state<S>(self) -> GitPerforcePair<S> {
        return GitPerforcePair::<S> {
            repository: self.repository,
            perforce_client: self.perforce_client,
            last_synced_cn: self.last_synced_cn,
            _marker: std::marker::PhantomData::<S>,
        };
    }

    fn get_last_cn(&self) -> Option<String> {
        None
    }

    pub fn then<A: Action<InputState = State>>(self, action: A) -> Result<GitPerforcePair<A::OutputState>, SyncError> {
        action.run(self)
    }
}

impl From<GitPerforcePair<Sync>> for GitPerforcePair<Dirty> {
    fn from(pair: GitPerforcePair<Sync>) -> GitPerforcePair<Dirty> {
        return pair.into_state::<Dirty>();
    }
}

pub trait Action {
    type InputState;
    type OutputState;

    fn run(self, pair: GitPerforcePair<Self::InputState>) -> Result<GitPerforcePair<Self::OutputState>, SyncError>;
}