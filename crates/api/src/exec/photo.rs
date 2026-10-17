//! Where exec photos are stored.

pub const BUCKET: &str = "kcsu-admin-public";
pub const FOLDER: &str = "exec/";

/// The public URL of a photo stored as `name` in [`FOLDER`].
pub fn url(name: &str) -> String {
    format!("https://storage.googleapis.com/{BUCKET}/{FOLDER}{name}")
}

pub fn is_photo_url(url: &str) -> bool {
    url.strip_prefix(&self::url("")).is_some_and(|name| {
        !name.is_empty()
            && !name.starts_with('.')
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    })
}
