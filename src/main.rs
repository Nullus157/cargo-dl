use anyhow::Error;
use clap::Parser;
use tracing_subscriber::EnvFilter;

const APP_NAME: &str = env!("CARGO_BIN_NAME");

#[derive(Debug, Parser)]
#[clap(name = APP_NAME, version, about)]
#[clap(global_setting(clap::AppSettings::DisableHelpSubcommand))]
#[clap(global_setting(clap::AppSettings::PropagateVersion))]
struct App {
}

impl App {
    #[fehler::throws]
    #[tracing::instrument(fields(%self))]
    pub(crate) fn run(self) {
        tracing::trace!("starting app")
    }
}

impl std::fmt::Display for App {
    #[fehler::throws(std::fmt::Error)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) {
        write!(f, "{}", APP_NAME)?;
    }
}

#[fehler::throws]
#[tracing::instrument(err)]
fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("CARGO_DL_LOG"))
        .with_writer(std::io::stderr)
        .pretty()
        .init();
    App::parse().run()?;
}
