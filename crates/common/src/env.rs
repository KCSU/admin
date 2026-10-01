/// A required environment variable isn't set.
#[derive(Debug, thiserror::Error)]
#[error("environment variable {0} is not set")]
pub struct MissingEnv(String);

/// Reads the environment variable `name`.
///
/// Fails if it isn't set.
pub fn require_env(name: &str) -> Result<String, MissingEnv> {
    std::env::var(name).map_err(|_| MissingEnv(name.to_owned()))
}
