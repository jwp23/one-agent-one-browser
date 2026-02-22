# One Agent - One Browser

This is an experiment to see if an agent using LLMs, could build a functional browser by itself with minimal guidance, and without using any 3rd party libraries.

## Ideal Result / Goals

- A binary that can render a .html file to the display

## Running

Platform:

- Linux (native Wayland or X11/XWayland).
- Windows 10 (1703+) or Windows 11
- macOS 11+ (double-check)

### System dependencies

This project uses system libraries/frameworks via FFI.

- Linux: Wayland client (xdg-shell protocol metadata is embedded in Rust) and/or X11/Xft, plus Cairo, librsvg, libcurl, libpng, libjpeg-turbo, libwebp.
- Windows 10/11: WinHTTP, WIC (PNG/JPEG/WebP), Direct2D/DirectWrite. If WebP decode fails, install Microsoft "WebP Image Extensions".
- macOS: system frameworks (CoreGraphics/CoreText/ImageIO/QuickLook).

Arch Linux:

```sh
sudo pacman -S --needed wayland wayland-protocols xorg-xwayland libx11 libxft cairo librsvg curl libpng libjpeg-turbo libwebp
```

Ubuntu:

```sh
sudo apt-get update
sudo apt-get install -y libwayland-dev wayland-protocols xwayland libx11-dev libxft-dev libcairo2-dev librsvg2-dev libglib2.0-dev libcurl4-openssl-dev libpng-dev libjpeg-turbo8-dev libturbojpeg0-dev libwebp-dev
```

RHEL:

```sh
sudo dnf install -y wayland wayland-devel wayland-protocols-devel xorg-x11-server-Xwayland libX11 libXft cairo librsvg2 libcurl libpng libjpeg-turbo libwebp
```

If you run only Xorg (not Wayland), install an Xorg server package (`xorg-server` / `xorg` / `xorg-x11-server-Xorg`).
If `$DISPLAY` is unset, Linux startup also probes `/tmp/.X11-unix` for available X server sockets.

```sh
# Built-in "Hello World"
cargo run

# Render a local file
cargo run -- test-file.html

# Render a URL
cargo run -- https://example.com

# Save a PNG screenshot and exit once the page is ready
cargo run -- test-file.html --screenshot out.png

# Headless mode (Linux: still requires a compositor/display server, Wayland or X11)
cargo run -- --headless test-file.html --screenshot out.png
```

### Arguments

- `<target>` (optional): path to an HTML file, or an `http(s)://...` URL.
- `--screenshot <path>` / `--screenshot=<path>`: write a PNG screenshot and exit.
- `--headless`: don't map a window; useful for automation/tests.
- `--width <px>` / `--width=<px>`: initial viewport width in CSS pixels (default: 1024).
- `--height <px>` / `--height=<px>`: initial viewport height in CSS pixels (default: 768).
- `OAB_SCALE` (env): override the DPI scale factor (e.g. `1.25` or `125%`).
- `OAB_LINUX_BACKEND` (env, Linux): `auto` (default), `wayland`, or `x11`.

## Tests

```sh
cargo test

# If you don't have an X server (Linux CI/headless), use Xvfb:
xvfb-run -a cargo test
```

Render regression tests compare screenshots to per-platform baseline PNGs in `tests/cases/`.

- `OAB_RENDER_TEST_MIN_SIMILARITY` (env): minimum required similarity ratio (default: `0.95`; set `1.0` for exact match).
