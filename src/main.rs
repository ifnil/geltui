mod app;
mod config;
mod jellyfin;

use anyhow::Result;
use app::App;
use config::Config;
use jellyfin::Session;

fn main() -> Result<()> {
    let config = Config::load()?;
    let session = Session::connect(&config)?;
    App::new(config, session)?.run()
}
