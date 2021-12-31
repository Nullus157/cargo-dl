use anyhow::{Context, Error};
use clap::Parser;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

const APP_NAME: &str = env!("CARGO_BIN_NAME");

#[derive(Debug, Parser)]
#[clap(name = APP_NAME, version, about)]
#[clap(global_setting(clap::AppSettings::DisableHelpSubcommand))]
#[clap(global_setting(clap::AppSettings::PropagateVersion))]
struct App {
    /// Specify this flag to have the crate extracted automatically.
    ///
    /// Note that unless changed via the --output flag, this will extract the files to a new
    /// subdirectory bearing the name of the downloaded crate archive.
    #[clap(short, long)]
    extract: bool,

    /// Normally, the compressed crate is written to a file (or directory if --extract is used)
    /// based on its name and version.  This flag allows to change that by providing an explicit
    /// file or directory path.
    #[clap(short, long)]
    output: Option<String>,

    /// Increase logging verbosity
    #[clap(short, parse(from_occurrences))]
    verbosity: i32,

    /// Decrease logging verbosity
    #[clap(short, parse(from_occurrences))]
    quietness: i32,

    /// The crate to download.
    #[clap(name = "CRATE")]
    krate: String,

    /// Which version of the crate to download, in the standard semver constraint format used in
    /// Cargo.toml. If unspecified the newest version will be fetched.
    version: Option<String>,
}

impl App {
    #[fehler::throws]
    #[tracing::instrument(fields(%self))]
    fn run(self) {
        tracing::trace!("starting app");
    }

    fn log_level(&self) -> LevelFilter {
        const LEVELS: [LevelFilter; 6] = [
            LevelFilter::OFF,
            LevelFilter::ERROR,
            LevelFilter::WARN,
            LevelFilter::INFO,
            LevelFilter::DEBUG,
            LevelFilter::TRACE,
        ];
        LEVELS[usize::try_from(
            (2 - self.quietness + self.verbosity)
                .clamp(0, i32::try_from(LEVELS.len() - 1).unwrap()),
        )
        .expect("clamped into range")]
    }

    #[fehler::throws]
    fn env_filter(&self) -> EnvFilter {
        let mut filter = EnvFilter::new("WARN").add_directive(self.log_level().into());
        if let Some(directive) = get_env_directive("CARGO_DL_LOG")? {
            filter = filter.add_directive(directive);
        }
        filter
    }
}

impl std::fmt::Display for App {
    #[fehler::throws(std::fmt::Error)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) {
        write!(f, "{}", APP_NAME)?;
        if self.extract {
            write!(f, " --extract")?;
        }
        if let Some(output) = &self.output {
            write!(f, " --output={:?}", output)?;
        }
        for _ in 0..self.verbosity {
            write!(f, " -v")?;
        }
        for _ in 0..self.quietness {
            write!(f, " -q")?;
        }
        write!(f, " {}", self.krate)?;
        if let Some(version) = &self.version {
            write!(f, " {}", version)?;
        }
    }
}

#[fehler::throws]
#[fn_error_context::context("parsing directive {:?}", directive)]
fn parse_directive(directive: &str) -> tracing_subscriber::filter::Directive {
    directive.parse()?
}

#[fehler::throws]
#[fn_error_context::context("getting directive from env var {:?}", var)]
fn get_env_directive(var: &str) -> Option<tracing_subscriber::filter::Directive> {
    if let Some(var) = std::env::var_os("CARGO_DL_LOG") {
        let s = var.to_str().context("CARGO_DL_LOG not unicode")?;
        Some(parse_directive(s)?)
    } else {
        None
    }
}

fn base_env_filter() -> EnvFilter {
    let mut filter = EnvFilter::new("WARN");
    // Silently ignore any errors at this point, they will be caught later when reconstructing the
    // filter
    if let Ok(Some(directive)) = get_env_directive("CARGO_DL_LOG") {
        filter = filter.add_directive(directive);
    }
    filter
}

#[fehler::throws]
fn main() {
    let (app, filter) = tracing::subscriber::with_default(
        tracing_subscriber::fmt()
            .with_env_filter(base_env_filter())
            .with_writer(std::io::stderr)
            .pretty()
            .finish(),
        || {
            let app = App::parse();
            app.env_filter().map(|filter| (app, filter))
        },
    )?;
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .pretty()
        .init();
    app.run()?;
}
