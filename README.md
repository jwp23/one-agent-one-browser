# One Agent - One Browser

This is an experiment to see if an agent using LLMs, could build a functional browser by itself with minimal guidance, and without using any 3rd party libraries.

## Ideal Result / Goals

- A binary that can render a .html file to the display

## Running

Platform: Linux/X11 only (requires an X server and `$DISPLAY`).

### System dependencies

This project uses FFI to link against common system libraries. The exact package names vary across distros, but you generally need: X11/Xft/Cairo, librsvg, libcurl, libpng, libjpeg-turbo, and libwebp.

Arch Linux (Wayland via XWayland):

```sh
sudo pacman -S --needed xorg-xwayland libx11 libxft cairo librsvg curl libpng libjpeg-turbo libwebp
```

Ubuntu (Wayland via XWayland):

```sh
sudo apt-get update
sudo apt-get install -y xwayland libx11-6 libxft2 libcairo2 librsvg2-2 libcurl4 libpng16-16 libturbojpeg0 libwebp7
```

RHEL (Wayland via XWayland):

```sh
sudo dnf install -y xorg-x11-server-Xwayland libX11 libXft cairo librsvg2 libcurl libpng libjpeg-turbo libwebp
```

If you run an Xorg session (not Wayland), install an Xorg server package instead of XWayland (`xorg-server` / `xorg` / `xorg-x11-server-Xorg`).

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
