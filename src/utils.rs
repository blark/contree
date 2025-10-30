//! Common utility functions

/// Split a path into (directory, basename)
/// Examples:
///   "foo/bar" -> ("foo", "bar")
///   "foo/bar/baz" -> ("foo/bar", "baz")
///   "file" -> ("", "file")
///   "foo/bar/" -> ("foo", "bar")
pub fn split_path(path: &str) -> (&str, &str) {
    let path = path.trim_end_matches('/');
    if let Some(pos) = path.rfind('/') {
        (&path[..pos], &path[pos + 1..])
    } else {
        ("", path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_path() {
        assert_eq!(split_path("foo/bar"), ("foo", "bar"));
        assert_eq!(split_path("foo/bar/baz"), ("foo/bar", "baz"));
        assert_eq!(split_path("file"), ("", "file"));
        assert_eq!(split_path("foo/bar/"), ("foo", "bar"));
        assert_eq!(split_path("/root/file"), ("/root", "file"));
        assert_eq!(split_path("/file"), ("", "file"));
    }
}