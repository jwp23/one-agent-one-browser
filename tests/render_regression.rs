use std::path::PathBuf;
use std::process::Command;

#[test]
fn render_regression_suite() {
    let harness = PathBuf::from(env!("CARGO_BIN_EXE_render-test"));
    let cases_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("cases");

    let cases = [
        cases_dir.join("hello-strong.html"),
        cases_dir.join("long-word.html"),
        cases_dir.join("multi-paragraph.html"),
        cases_dir.join("adjacent-inline.html"),
        cases_dir.join("inline-margin-right.html"),
        cases_dir.join("display-none.html"),
        cases_dir.join("visibility-hidden.html"),
        cases_dir.join("blog-test.html"),
        cases_dir.join("hn-frontpage.html"),
        cases_dir.join("medium-home.html"),
        cases_dir
            .join("hn-frontpage-2026-01-16")
            .join("hn-frontpage-2026-01-16.html"),
    ];

    let mut cmd = Command::new(&harness);
    cmd.args(&cases);

    let output = cmd
        .output()
        .unwrap_or_else(|err| panic!("Failed to run {}: {err}", harness.display()));

    if !output.status.success() {
        panic!(
            "{}\nstdout:\n{}\nstderr:\n{}\n",
            format!(
                "Render regression harness failed (exit={}).",
                output.status.code().unwrap_or(-1)
            ),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
}
