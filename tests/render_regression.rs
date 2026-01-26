use std::path::PathBuf;
use std::process::Command;

#[test]
fn render_regression_suite() {
    let harness = PathBuf::from(env!("CARGO_BIN_EXE_render-test"));
    let cases_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("cases");

    let cases = [
        cases_dir.join("hello-strong.html"),
        cases_dir.join("long-word.html"),
        cases_dir.join("multi-paragraph.html"),
        cases_dir.join("adjacent-inline.html"),
        cases_dir.join("inline-margin-right.html"),
        cases_dir.join("border-bottom.html"),
        cases_dir.join("border-radius.html"),
        cases_dir.join("image-webp.html"),
        cases_dir.join("inline-svg.html"),
        cases_dir.join("svg-attr-px.html"),
        cases_dir.join("svg-auto-width-flex.html"),
        cases_dir.join("opacity.html"),
        cases_dir.join("vw-vh.html"),
        cases_dir.join("unset-position.html"),
        cases_dir.join("display-none.html"),
        cases_dir.join("visibility-hidden.html"),
        cases_dir.join("float-columns.html"),
        cases_dir.join("css-vars-root.html"),
        cases_dir.join("linear-gradient.html"),
        cases_dir.join("flex-column-auto-margin.html"),
        cases_dir.join("percent-width.html"),
        cases_dir.join("input-controls.html"),
        cases_dir.join("blog-test.html"),
        cases_dir.join("simonwillison-kakapo-cam.url"),
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
