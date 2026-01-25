# One Agent - One Browser

This is an experiment to see if an agent using LLMs, could build a functional browser by itself with minimal guidance, and without using any 3rd party libraries.

## Ideal Result / Goals

- A binary that can render a .html file to the display

## Running

Platform: Linux/X11 only (requires an X server and `$DISPLAY`).

```sh
# Built-in "Hello World"
cargo run

# Render a local file
cargo run -- test-file.html

# Render a URL
cargo run -- https://example.com

# Save a PNG screenshot and exit once the page is ready
cargo run -- test-file.html --screenshot out.png

# Headless mode (no mapped window; still requires an X server, e.g. via xvfb-run)
cargo run -- --headless test-file.html --screenshot out.png
```

### Arguments

- `<target>` (optional): path to an HTML file, or an `http(s)://...` URL.
- `--screenshot <path>` / `--screenshot=<path>`: write a PNG screenshot and exit.
- `--headless`: don't map a window; useful for automation/tests.

## Tests

```sh
cargo test

# If you don't have an X server (CI/headless), use Xvfb:
xvfb-run -a cargo test
```
