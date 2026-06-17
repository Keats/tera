use std::path::PathBuf;

use globset::GlobBuilder;
use walkdir::WalkDir;

use crate::errors::{Error, TeraResult};

/// Loads the glob and find all files matching that glob,
/// returning a list of (path, filename)
pub fn load_from_glob(glob: &str) -> TeraResult<Vec<(PathBuf, String)>> {
    let Some(first_star) = glob.find('*') else {
        return Err(Error::message(format!(
            "Not a valid glob: no `*` were found in `{glob}`"
        )));
    };

    // https://github.com/Keats/tera/pull/991
    let split_at = glob[..first_star]
        .rfind(std::path::is_separator)
        .map_or(0, |i| i + 1);
    let (parent_dir, glob_end) = glob.split_at(split_at);
    // If no directory, we default to cwd
    let parent_dir = if parent_dir.is_empty() {
        "."
    } else {
        parent_dir
    };

    // If canonicalize fails, just abort it and resume with the given path.
    // Consumers expect invalid globs to just return the empty set instead of failing.
    // See https://github.com/Keats/tera/issues/819#issuecomment-1480392230
    let parent_dir =
        std::fs::canonicalize(parent_dir).unwrap_or_else(|_| std::path::PathBuf::from(parent_dir));

    let canonical_glob = {
        let mut p = parent_dir.clone();
        p.push(glob_end);
        p.to_string_lossy().to_string()
    };

    let glob_matcher = GlobBuilder::new(&canonical_glob)
        .literal_separator(true)
        .build()
        .map_err(|e| Error::message(format!("Glob is invalid: {e}")))?
        .compile_matcher();

    let mut paths = Vec::new();
    for entry in WalkDir::new(&parent_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let mut path = entry.path().to_path_buf();
        if path.is_dir() || !glob_matcher.is_match(&path) {
            continue;
        }

        if path.starts_with("./") {
            path = path.strip_prefix("./").unwrap().to_path_buf();
        }

        let Ok(relative) = path.strip_prefix(&parent_dir) else {
            continue;
        };
        // unify on forward slash
        let filepath = relative.to_string_lossy().replace('\\', "/");

        paths.push((path, filepath));
    }

    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::load_from_glob;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn can_load_from_glob() {
        let data = load_from_glob("examples/basic/templates/**/*").unwrap();
        assert_eq!(data.len(), 3);
        assert!(data.iter().any(|(_, y)| y == "base.html"));
        assert!(data.iter().any(|(_, y)| y == "users/profile.html"));
    }

    #[test]
    fn can_load_from_glob_with_patterns() {
        let data = load_from_glob("examples/basic/templates/**/*.{html,xml}").unwrap();
        assert_eq!(data.len(), 3);
        assert!(data.iter().any(|(_, y)| y == "base.html"));
        assert!(data.iter().any(|(_, y)| y == "users/profile.html"));
    }

    // https://github.com/Keats/tera/issues/380
    #[test]
    fn glob_work_with_absolute_paths() {
        let tmp_dir = tempdir().expect("create temp dir");
        let cwd = tmp_dir.path().canonicalize().unwrap();
        File::create(cwd.join("hey.html")).expect("Failed to create a test file");
        File::create(cwd.join("ho.html")).expect("Failed to create a test file");
        let glob = cwd.join("*.html").into_os_string().into_string().unwrap();
        let data = load_from_glob(&glob).unwrap();
        assert_eq!(data.len(), 2);
    }

    #[test]
    fn glob_work_with_absolute_paths_and_double_star() {
        let tmp_dir = tempdir().expect("create temp dir");
        let cwd = tmp_dir.path().canonicalize().unwrap();
        File::create(cwd.join("hey.html")).expect("Failed to create a test file");
        File::create(cwd.join("ho.html")).expect("Failed to create a test file");
        let glob = cwd
            .join("**")
            .join("*.html")
            .into_os_string()
            .into_string()
            .unwrap();
        let data = load_from_glob(&glob).unwrap();
        assert_eq!(data.len(), 2);
    }

    // Test for https://github.com/Keats/tera/issues/574
    #[test]
    fn glob_work_with_paths_starting_with_dots() {
        use std::path::PathBuf;

        let this_dir = std::env::current_dir()
            .expect("Could not retrieve the executable's current directory.");

        let scratch_dir = tempfile::Builder::new()
            .prefix("tera_test_scratchspace")
            .tempdir_in(&this_dir)
            .unwrap_or_else(|_| {
                panic!(
                    "Could not create temporary directory for test in current directory ({}).",
                    this_dir.display()
                )
            });
        dbg!(&scratch_dir.path().display());

        File::create(scratch_dir.path().join("hey.html")).expect("Failed to create a test file");
        File::create(scratch_dir.path().join("ho.html")).expect("Failed to create a test file");
        let glob = PathBuf::from("./")
            .join(scratch_dir.path().file_name().unwrap())
            .join("**")
            .join("*.html")
            .into_os_string()
            .into_string()
            .unwrap();
        let data = load_from_glob(&glob).unwrap();
        assert_eq!(data.len(), 2);
    }

    // https://github.com/Keats/tera/issues/819
    #[test]
    fn empty_list_on_invalid_glob() {
        let data = load_from_glob("\\dev/null/*").unwrap();
        assert!(data.is_empty());
    }

    #[test]
    fn glob_without_directory_resolves_against_cwd() {
        let data = load_from_glob("*.toml").unwrap();
        assert!(data.iter().any(|(_, name)| name == "Cargo.toml"));
    }

    // https://github.com/Keats/tera/issues/740
    // A wildcard in a directory component (eg `templ*/`) must split the base dir on the path
    // boundary, not mid-component. Ported from Keats/tera#991.
    #[test]
    fn glob_with_wildcard_in_directory_component() {
        let tmp_dir = tempdir().expect("create temp dir");
        let root = tmp_dir.path().canonicalize().unwrap();
        std::fs::create_dir(root.join("templates")).unwrap();
        File::create(root.join("templates").join("hey.html")).unwrap();

        let glob = root
            .join("templ*")
            .join("*.html")
            .to_string_lossy()
            .to_string();
        let data = load_from_glob(&glob).unwrap();
        assert_eq!(data.len(), 1);
        assert!(data.iter().any(|(_, name)| name == "templates/hey.html"));
    }
}
