use libforge_core::build_plan::{BuildPlan, BuiltArtifact};

pub type BuildResult<T> = Result<T, BuildError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildError {
    pub message: String,
}

impl BuildError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "build execution failed: {}", self.message)
    }
}

impl std::error::Error for BuildError {}

pub trait BuildExecutor {
    fn execute(&self, plan: &BuildPlan) -> BuildResult<Vec<BuiltArtifact>>;
}
