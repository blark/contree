/// Docker whiteout handling for layer deletions

const WHITEOUT_PREFIX: &str = ".wh.";
const OPAQUE_WHITEOUT: &str = ".wh..wh..opq";

/// Check if a path is a Docker whiteout marker
pub fn is_whiteout(path: &str) -> bool {
    let basename = path.rsplit('/').next().unwrap_or(path);
    basename.starts_with(WHITEOUT_PREFIX)
}

/// Check if a path is an opaque whiteout marker
pub fn is_opaque(path: &str) -> bool {
    let basename = path.rsplit('/').next().unwrap_or(path);
    basename == OPAQUE_WHITEOUT
}

/// Extract the target path from a whiteout marker path
/// Example: "dir/.wh.file" -> "dir/file"
pub fn whiteout_target(path: &str) -> String {
    let (dir_path, basename) = split_path(path);

    // Strip the ".wh." prefix
    let target_basename = if basename.starts_with(WHITEOUT_PREFIX) {
        &basename[WHITEOUT_PREFIX.len()..]
    } else {
        basename
    };

    if dir_path.is_empty() {
        target_basename.to_string()
    } else {
        format!("{}/{}", dir_path, target_basename)
    }
}

/// Get the directory path from an opaque whiteout marker
/// Example: "foo/bar/.wh..wh..opq" -> "foo/bar"
pub fn opaque_dir(path: &str) -> &str {
    let (dir_path, _) = split_path(path);
    dir_path
}

fn split_path(path: &str) -> (&str, &str) {
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
    fn test_is_whiteout() {
        assert!(is_whiteout(".wh.file"));
        assert!(is_whiteout("dir/.wh.file"));
        assert!(is_whiteout(".wh..wh..opq"));
        assert!(!is_whiteout("normal_file"));
        assert!(!is_whiteout("dir/normal_file"));
    }

    #[test]
    fn test_is_opaque() {
        assert!(is_opaque(".wh..wh..opq"));
        assert!(is_opaque("dir/.wh..wh..opq"));
        assert!(!is_opaque(".wh.file"));
        assert!(!is_opaque("normal_file"));
    }

    #[test]
    fn test_whiteout_target() {
        assert_eq!(whiteout_target(".wh.file"), "file");
        assert_eq!(whiteout_target("dir/.wh.file"), "dir/file");
        assert_eq!(whiteout_target("a/b/c/.wh.test"), "a/b/c/test");
    }

    #[test]
    fn test_opaque_dir() {
        assert_eq!(opaque_dir("dir/.wh..wh..opq"), "dir");
        assert_eq!(opaque_dir("a/b/c/.wh..wh..opq"), "a/b/c");
        assert_eq!(opaque_dir(".wh..wh..opq"), "");
    }
}
