mod headless;
mod painter;
mod scaled;
mod scale;
mod svg;
mod windowed;

use super::WindowOptions;
use crate::app::App;

pub fn run_window<A: App>(title: &str, options: WindowOptions, app: &mut A) -> Result<(), String> {
    if options.headless {
        return headless::run(options, app);
    }
    windowed::run(title, options, app)
}
