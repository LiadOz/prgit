use std::path::{Path, PathBuf};

pub struct P4 {
    p4_path: Option<PathBuf>,
    port: Option<String>,
    user: Option<String>,
    password: Option<String>,
    client: Option<String>,
    retries: Option<usize>,
}

impl P4 {
    pub fn new() -> Self {
        return Self {p4_path: None, port: None, user: None, password: None, client: None, retries: None};
    }

    pub fn with_p4_path(mut self, p4_path: impl AsRef<Path>) -> Self {
        self.p4_path = Some(p4_path.as_ref().to_path_buf());
        self
    }

    pub fn with_port(mut self, port: impl AsRef<str>) -> Self {
        self.port = Some(port.as_ref().to_string());
        self
    }

    pub fn with_user(mut self, user: impl AsRef<str>) -> Self {
        self.user = Some(user.as_ref().to_string());
        self
    }

    pub fn with_password(mut self, password: impl AsRef<str>) -> Self {
        self.password = Some(password.as_ref().to_string());
        self
    }
    
    pub fn with_client(mut self, client: impl AsRef<str>) -> Self {
        self.client = Some(client.as_ref().to_string());
        self
    }

    pub fn with_retries(mut self, retries: usize) -> Self {
        self.retries = Some(retries);
        self
    }
}