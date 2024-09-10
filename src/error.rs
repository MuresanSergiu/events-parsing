pub enum ProjectError {
    ParseError(String),
    NotifyError(String),
    IoError(String),
}

impl std::fmt::Debug for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectError::ParseError(err) => write!(f, "{}", err),
            ProjectError::NotifyError(err) => write!(f, "{}", err),
            ProjectError::IoError(err) => write!(f, "{}", err),
        }
    }
}

