use one_agent_one_browser::{browser, cli, platform};

fn main() {
    let args = match cli::parse_args(std::env::args_os().skip(1)) {
        Ok(args) => args,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(2);
        }
    };

    let app = match args.html_path {
        Some(path) => browser::BrowserApp::from_file(&path),
        None => browser::BrowserApp::from_html("Hello World", "<p>Hello World</p>"),
    };

    let mut app = match app {
        Ok(app) => app,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };

    let title = app.title().to_owned();
    let options = platform::WindowOptions {
        screenshot_path: args.screenshot_path,
    };
    if let Err(err) = platform::run_window(&title, options, |painter, viewport| {
        app.render(painter, viewport)
    }) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
