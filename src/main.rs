use anyhow::Error;
use clap::Parser;
use tracing_subscriber::EnvFilter;

const _USER_AGENT: &str = concat!("cargo-dl/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Parser)]
#[clap(bin_name = "cargo", version, about)]
#[clap(global_setting(clap::AppSettings::DisableHelpSubcommand))]
#[clap(global_setting(clap::AppSettings::PropagateVersion))]
enum Command {
    /// Cargo subcommand for downloading crate sources
    Dl(App),
}

#[derive(Debug, Parser)]
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

    /// The crate to download.
    #[clap(name = "CRATE")]
    krate: String,

    // TODO: Easy way to download latest pre-release
    // TODO: Way to download yanked versions
    /// Which version of the crate to download, in the standard semver constraint format used in
    /// Cargo.toml. If unspecified the newest non-prerelease, non-yanked version will be fetched.
    version_request: Option<semver::VersionReq>,
}

impl App {
    #[fehler::throws]
    #[tracing::instrument(fields(%self))]
    fn run(self) {
        let index = crates_index::Index::new_cargo_default()?;

        // TODO: fuzzy name matching https://github.com/frewsxcv/rust-crates-index/issues/75
        let krate = match index.crate_(&self.krate) {
            Some(krate) => krate,
            None => {
                tracing::error!("could not find crate {} in the index", self.krate);
                return;
            }
        };

        tracing::debug!(
            "all available versions: {:?}",
            Vec::from_iter(krate.versions().iter().map(|v| v.version()))
        );

        let version_request = self.version_request.unwrap_or(semver::VersionReq::STAR);
        let versions = {
            let mut versions: Vec<_> = krate
                .versions()
                .iter()
                .filter(|version| !version.is_yanked())
                .filter_map(|version| match semver::Version::parse(version.version()) {
                    Ok(num) => Some((num, version)),
                    Err(err) => {
                        tracing::warn!(
                            "Ignoring non-semver version {} {err:#?}",
                            version.version()
                        );
                        None
                    }
                })
                .filter(|(num, _)| version_request.matches(num))
                .collect();
            versions.sort_by(|(a, _), (b, _)| a.cmp(b).reverse());
            versions
        };

        tracing::debug!(
            "matching versions: {:?}",
            Vec::from_iter(versions.iter().map(|(num, _)| num))
        );

        // TODO: If no matching versions, check for version matching but yanked versions and emit
        // warning

        let (version_num, _version) = match versions.first() {
            Some(val) => val,
            None => {
                tracing::error!(
                    "no version matching {version_request} found for {}",
                    krate.name()
                );
                return;
            }
        };

        tracing::debug!("selected version: {version_num}");

        // TODO: check cargo cache
        // TODO: download
        // TODO: verify checksum
        // TODO: maybe extract
    }
}

impl std::fmt::Display for App {
    #[fehler::throws(std::fmt::Error)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) {
        write!(f, "cargo dl")?;
        if self.extract {
            write!(f, " --extract")?;
        }
        if let Some(output) = &self.output {
            write!(f, " --output={:?}", output)?;
        }
        write!(f, " {}", self.krate)?;
        if let Some(version_request) = &self.version_request {
            write!(f, " {}", version_request)?;
        }
    }
}

#[fehler::throws]
fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("CARGO_DL_LOG"))
        .with_writer(std::io::stderr)
        .pretty()
        .init();
    let Command::Dl(app) = Command::parse();
    app.run()?;
}
