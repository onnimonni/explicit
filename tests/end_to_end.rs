//! Runs the whole engine on `tests/fixtures/sample` and snapshots the diagnostics.

use std::path::PathBuf;

use explicit::config::Config;
use explicit::engine::{self, Options};

#[test]
fn sample_project() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample");
    let config = Config {
        root: root.canonicalize().expect("fixture dir exists"),
        ..Config::default()
    };
    let files = engine::discover(std::slice::from_ref(&config.root), &config).expect("discover");
    let ws = engine::build_workspace(&files, &config);
    let diags = engine::check(&ws, &files, &config, &Options { remote: false });
    let summary: Vec<String> = diags
        .iter()
        .map(|d| {
            format!(
                "{}:{}:{} {} {}",
                d.path.display(),
                d.line,
                d.column,
                d.severity.as_str(),
                d.rule
            )
        })
        .collect();
    insta::assert_snapshot!(summary.join("\n"));
}
