pub mod app;
pub mod config;
pub mod discogs_client;
pub mod img_to_ascii;
pub mod record;

use std::io::Result;

use app::App;

fn main() -> Result<()> {
    let mut app = App::init()?;
    app.run()?;
    app.quit()?;

    Ok(())
}
