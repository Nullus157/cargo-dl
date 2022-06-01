mod crate_name;
mod package_id_spec;
mod cache;

use anyhow::{Context, Error};
use clap::Parser;
use tracing_subscriber::EnvFilter;
use crate::{package_id_spec::PackageIdSpec, crate_name::CrateName};

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

    // TODO: Easy way to download latest pre-release
    // TODO: Way to download yanked versions
    /// The crate to download.
    ///
    /// Which version of the crate to download, in the standard semver constraint format used in
    /// Cargo.toml. If unspecified the newest non-prerelease, non-yanked version will be fetched.
    #[clap(name = "CRATE[:VERSION_REQ]")]
    spec: PackageIdSpec,

    /// Allow yanked versions to be chosen.
    #[clap(long)]
    allow_yanked: bool,
}

impl App {
    #[fehler::throws]
    #[tracing::instrument(fields(%self))]
    fn run(self) {
        let index = crates_index::Index::new_cargo_default()?;

        // TODO: fuzzy name matching https://github.com/frewsxcv/rust-crates-index/issues/75
        let krate = match index.crate_(&self.spec.name.0) {
            Some(krate) => krate,
            None => {
                tracing::error!("could not find crate `{}` in the index", self.spec.name);
                return;
            }
        };

        tracing::debug!(
            "all available versions: {:?}",
            Vec::from_iter(krate.versions().iter().map(|v| v.version()))
        );

        let version_request = self.spec.version_req.unwrap_or(semver::VersionReq::STAR);
        let versions = {
            let mut versions: Vec<_> = krate
                .versions()
                .iter()
                .filter(|version| self.allow_yanked || !version.is_yanked())
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

        let (version_num, version) = match versions.first() {
            Some(val) => val,
            None => {
                tracing::error!(
                    "no version matching {version_request} found for {}",
                    krate.name()
                );
                let yanked_versions = {
                    let mut versions: Vec<_> = krate
                        .versions()
                        .iter()
                        .filter(|version| version.is_yanked())
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
                if let Some((num, _)) = yanked_versions.first() {
                    tracing::warn!("The yanked version {num} matched, use `--allow-yanked` to download it");
                }
                return;
            }
        };

        tracing::warn!("selected version `{version_num}`");

        cache::lookup(&index, version)?;
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
        if self.allow_yanked {
            write!(f, " --allow-yanked")?;
        }
        if self.extract {
            write!(f, " --extract")?;
        }
        if let Some(output) = &self.output {
            write!(f, " --output={:?}", output)?;
        }
        write!(f, " {}", self.spec)?;
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

fn env_filter() -> (EnvFilter, Option<anyhow::Error>) {
    let filter = EnvFilter::new("WARN");
    match get_env_directive("CARGO_DL_LOG") {
        Ok(Some(directive)) => {
            (filter.add_directive(directive), None)
        }
        Ok(None) => {
            (filter, None)
        }
        Err(err) => {
            (filter, Some(err.context("failed to apply log directive")))
        }
    }
}

#[fehler::throws]
fn main() {
    let (env_filter, err) = env_filter();
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .pretty()
        .init();
    if let Some(err) = err {
        tracing::warn!("{err:?}");
    }
    match Command::try_parse() {
        Ok(Command::Dl(app)) => app.run()?,
        Err(e @ clap::Error { kind: clap::ErrorKind::ValueValidation, .. }) => {
            use std::error::Error;
            println!("Error: invalid value for {}", e.info[0]);
            println!();
            if let Some(source) = e.source() {
                println!("Caused by:");
                let chain = anyhow::Chain::new(source);
                for (i, error) in chain.into_iter().enumerate() {
                    println!("    {i}: {error}");
                }
            }
            std::process::exit(1);
        }
        Err(e) => e.exit(),
    }
}
