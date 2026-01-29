//! Unit tests for path filtering

use globset::{Glob, GlobSet, GlobSetBuilder};
use pretty_assertions::assert_eq;
use rstest::rstest;

/// Build a GlobSet from a list of patterns (mimics FilterMiddleware logic)
fn build_globset(patterns: &[&str]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        if let Ok(glob) = Glob::new(pattern) {
            builder.add(glob);
        }
    }
    builder.build().unwrap()
}

#[rstest]
#[case("/.env", true)]
#[case("/.env.local", true)]
#[case("/.env.production", true)]
#[case("/api/.env", true)]
#[case("/some/path/.env", true)]
#[case("/env", false)]
#[case("/environment", false)]
fn test_env_file_blocking(#[case] path: &str, #[case] should_block: bool) {
    let globset = build_globset(&["**/.env*"]);
    assert_eq!(globset.is_match(path), should_block, "Path: {}", path);
}

#[rstest]
#[case("/.git", true)]
#[case("/.git/config", true)]
#[case("/.git/HEAD", true)]
#[case("/.gitignore", true)]
#[case("/api/.git", true)]
#[case("/git", false)]
#[case("/github", false)]
fn test_git_blocking(#[case] path: &str, #[case] should_block: bool) {
    let globset = build_globset(&["**/.git*"]);
    assert_eq!(globset.is_match(path), should_block, "Path: {}", path);
}

#[rstest]
#[case("/.svn", true)]
#[case("/.svn/entries", true)]
#[case("/svn", false)]
fn test_svn_blocking(#[case] path: &str, #[case] should_block: bool) {
    let globset = build_globset(&["**/.svn*"]);
    assert_eq!(globset.is_match(path), should_block, "Path: {}", path);
}

#[rstest]
#[case("/backup.sql", true)]
#[case("/dump.sql", true)]
#[case("/data/backup.sql", true)]
#[case("/sql", false)]
#[case("/mysql", false)]
fn test_sql_file_blocking(#[case] path: &str, #[case] should_block: bool) {
    let globset = build_globset(&["**/*.sql"]);
    assert_eq!(globset.is_match(path), should_block, "Path: {}", path);
}

#[rstest]
#[case("/config.bak", true)]
#[case("/database.bak", true)]
#[case("/backup/file.bak", true)]
#[case("/bak", false)]
#[case("/backup", false)]
fn test_backup_file_blocking(#[case] path: &str, #[case] should_block: bool) {
    let globset = build_globset(&["**/*.bak"]);
    assert_eq!(globset.is_match(path), should_block, "Path: {}", path);
}

#[rstest]
#[case("/docker-compose.yml", true)]
#[case("/docker-compose.yaml", true)]
#[case("/docker-compose.prod.yml", true)]
#[case("/docker-compose.override.yml", true)]
#[case("/compose.yml", false)]
#[case("/docker.yml", false)]
fn test_docker_compose_blocking(#[case] path: &str, #[case] should_block: bool) {
    let globset = build_globset(&["**/docker-compose*.yml", "**/docker-compose*.yaml"]);
    assert_eq!(globset.is_match(path), should_block, "Path: {}", path);
}

#[test]
fn test_combined_default_patterns() {
    let patterns = vec![
        "**/.env*",
        "**/.git*",
        "**/.svn*",
        "**/.hg*",
        "**/*.sql",
        "**/*.bak",
        "**/docker-compose*.yml",
    ];
    let globset = build_globset(&patterns);

    // Should block
    assert!(globset.is_match("/.env"));
    assert!(globset.is_match("/.git/config"));
    assert!(globset.is_match("/backup.sql"));
    assert!(globset.is_match("/docker-compose.yml"));

    // Should allow
    assert!(!globset.is_match("/"));
    assert!(!globset.is_match("/api/users"));
    assert!(!globset.is_match("/static/app.js"));
    assert!(!globset.is_match("/_next/static/chunks/main.js"));
}

#[test]
fn test_empty_pattern_list() {
    let globset = build_globset(&[]);
    // Empty globset should not match anything
    assert!(!globset.is_match("/anything"));
    assert!(!globset.is_match("/.env"));
}

#[test]
fn test_root_path_not_blocked() {
    let globset = build_globset(&["**/.env*", "**/.git*"]);
    assert!(!globset.is_match("/"));
}

#[test]
fn test_api_paths_not_blocked() {
    let globset = build_globset(&["**/.env*", "**/.git*", "**/*.sql"]);
    assert!(!globset.is_match("/api/v1/users"));
    assert!(!globset.is_match("/api/health"));
    assert!(!globset.is_match("/graphql"));
}

#[test]
fn test_static_assets_not_blocked() {
    let globset = build_globset(&["**/.env*", "**/.git*"]);
    assert!(!globset.is_match("/static/app.js"));
    assert!(!globset.is_match("/static/style.css"));
    assert!(!globset.is_match("/_next/static/chunks/main.js"));
    assert!(!globset.is_match("/images/logo.png"));
}

#[rstest]
#[case("/wp-config.php", true)]
#[case("/wordpress/wp-config.php", true)]
fn test_php_config_blocking(#[case] path: &str, #[case] should_block: bool) {
    let globset = build_globset(&["**/wp-config.php"]);
    assert_eq!(globset.is_match(path), should_block, "Path: {}", path);
}

#[test]
fn test_case_sensitivity() {
    let globset = build_globset(&["**/.env*"]);
    // Glob matching is case-sensitive by default
    assert!(globset.is_match("/.env"));
    assert!(!globset.is_match("/.ENV")); // Different case
}

#[test]
fn test_nested_paths() {
    let globset = build_globset(&["**/.env*"]);
    assert!(globset.is_match("/deeply/nested/path/.env"));
    assert!(globset.is_match("/a/b/c/d/e/.env.local"));
}
