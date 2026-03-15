# JS Oracle

This directory contains an optional Playwright-based development oracle for pages
that require real JavaScript execution. It is intentionally isolated from the
Rust codebase:

- No changes to `Cargo.toml`
- No runtime dependency from the browser engine to Node.js
- No integration with the Rust test harness unless you choose to use it locally

## Setup

From the repo root:

```sh
cd tools/oracle
npm install
npx playwright install chromium
cd ../..
```

If you prefer to stay in the repo root after setup:

```sh
node tools/oracle/oracle.mjs \
  --target tests/cases/js-getelementbyid-textcontent.html \
  --out-dir tmp/oracle/js-getelementbyid
```

You can also run it through `npm`:

```sh
npm --prefix tools/oracle run oracle -- \
  --target "$PWD/tests/cases/js-getelementbyid-textcontent.html" \
  --out-dir "$PWD/tmp/oracle/js-getelementbyid"
```

## Output

Each run writes these files into `--out-dir`:

- `screenshot.png`: rendered page screenshot
- `dom.html`: serialized post-JS DOM from the main document
- `text.txt`: `document.body.innerText` snapshot
- `manifest.json`: metadata about the capture
- `events.json`: console messages, page errors, and failed requests

## Usage

```sh
node tools/oracle/oracle.mjs --target <path-or-url> --out-dir <dir> [options]
```

Options:

- `--browser <chromium|firefox|webkit>`: browser engine to launch. Default: `chromium`
- `--width <px>`: viewport width. Default: `1366`
- `--height <px>`: viewport height. Default: `768`
- `--wait-until <load|domcontentloaded|networkidle|commit>`: navigation wait mode. Default: `load`
- `--delay-ms <ms>`: extra delay after page load and font readiness. Default: `0`
- `--timeout-ms <ms>`: per-step timeout. Default: `30000`
- `--full-page`: capture the full scrollable page instead of the viewport
- `--executable-path <path>`: optional browser executable override

Examples:

```sh
node tools/oracle/oracle.mjs \
  --target https://example.com \
  --out-dir tmp/oracle/example \
  --wait-until networkidle \
  --delay-ms 250
```

```sh
node tools/oracle/oracle.mjs \
  --target tests/cases/hn-frontpage.html \
  --out-dir tmp/oracle/hn-frontpage \
  --width 1440 \
  --height 900
```

## Notes

- Local paths are resolved against the current working directory.
- `file://` pages work for static fixtures with relative assets.
- For pages with delayed client-side rendering, increase `--delay-ms` or switch to
  `--wait-until networkidle`.
