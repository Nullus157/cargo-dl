mod crate_name;
mod package_id_spec;
mod cache;
mod unpack;

use std::io::Read;
use anyhow::{anyhow, Context, Error};
use clap::Parser;
use tracing_subscriber::EnvFilter;
use crate::{package_id_spec::PackageIdSpec, crate_name::CrateName};

const USER_AGENT: &str = concat!("cargo-dl/", env!("CARGO_PKG_VERSION"));
const CRATE_SIZE_LIMIT: u64 = 40 * 1024 * 1024;

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

    /// Slow down operations for manually testing UI
    #[clap(long)]
    slooooow: bool,
}

impl App {
    fn slow(&self) {
        if self.slooooow {
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    }

    #[fehler::throws]
    #[tracing::instrument(fields(%self))]
    fn run(self) {
        let mut index = crates_index::Index::new_cargo_default()?;
        let bar = indicatif::ProgressBar::new_spinner().with_style(indicatif::ProgressStyle::default_spinner().template("{prefix:>12.bright.cyan} {spinner} {msg:.cyan}"))
            .with_prefix("Updating")
        .with_message("index");
        bar.enable_steady_tick(100);
        index.update()?;
        self.slow();
        bar.println("Updated index");

        bar.set_prefix("Finding");
        bar.set_message(self.spec.name.0.clone());
        self.slow();
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

        let version_request = self.spec.version_req.clone().unwrap_or(semver::VersionReq::STAR);
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
            Vec::from_iter(versions.iter().map(|(num, _)| num.to_string()))
        );

        let (_, version) = match versions.first() {
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

        bar.println(format!("Found {} {}", version.name(), version.version()));
        bar.set_prefix("Checking");
        bar.set_message(format!("cache for {} {}", version.name(), version.version()));
        self.slow();

        let output = self.output.clone().unwrap_or_else(|| if self.extract {
            format!("{}-{}", version.name(), version.version())
        } else {
            format!("{}-{}.crate", version.name(), version.version())
        });
        match cache::lookup(&index, version) {
            Ok(path) => {
                tracing::debug!("found cached crate at {}", path.display());
                bar.println("found cached crate");
                if self.extract {
                    bar.set_prefix("Extracting");
                    bar.set_message(format!("{} {} to {}", version.name(), version.version(), output));
                    self.slow();
                    let archive = tar::Archive::new(flate2::bufread::GzDecoder::new(std::io::BufReader::new(std::fs::File::open(path)?)));
                    unpack::unpack(version, archive, &output)?;
                    bar.finish_with_message(format!("{} {} extracted to {}", version.name(), version.version(), output));
                } else {
                    bar.set_prefix("Writing");
                    bar.set_message(format!("{} {} to {}", version.name(), version.version(), output));
                    self.slow();
                    std::fs::copy(path, &output)?;
                    bar.finish_with_message(format!("{} {} written to {}", version.name(), version.version(), output));
                }
            }
            Err(err) => {
                bar.println("no cached crate");
                use sha2::Digest;
                tracing::debug!("{err:?}");
                let url = version.download_url(&index.index_config()?).context("missing download url")?;
                bar.set_prefix("Downloading");
                bar.set_message(format!("{} {}", version.name(), version.version()));
                let resp = ureq::get(&url).set("User-Agent", USER_AGENT).call()?;
                let mut data;
                if let Some(len) = resp.header("Content-Length").and_then(|s| s.parse::<usize>().ok()) {
                    data = Vec::with_capacity(len);
                    bar.set_style(indicatif::ProgressStyle::default_bar().template("{prefix:>12.bright.cyan} [{bar:27}] {bytes:>9}/{total_bytes:9}  {bytes_per_sec} {elapsed:>4}/{eta:4} - {msg:.cyan}"));
                    bar.set_length(u64::try_from(len)?);
                } else {
                    data = Vec::with_capacity(usize::try_from(CRATE_SIZE_LIMIT)?);
                }
                bar.wrap_read(resp.into_reader()).take(CRATE_SIZE_LIMIT).read_to_end(&mut data)?;
                self.slow();
                tracing::debug!("downloaded crate ({} bytes)", data.len());
                let calculated_checksum = sha2::Sha256::digest(&data);
                if calculated_checksum.as_slice() != version.checksum() {
                    fehler::throw!(anyhow!("invalid checksum, expected {} but got {}", hex::encode(version.checksum()), hex::encode(calculated_checksum)));
                }
                tracing::debug!("verified checksum ({})", hex::encode(version.checksum()));

                if self.extract {
                    let archive = tar::Archive::new(flate2::bufread::GzDecoder::new(std::io::Cursor::new(data)));
                    unpack::unpack(version, archive, &output)?;
                    bar.println(format!("{} {} extracted to {}", version.name(), version.version(), output));
                } else {
                    std::fs::write(&output, data)?;
                    bar.println(format!("{} {} written to {}", version.name(), version.version(), output));
                }
            }
        }
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
    let filter = EnvFilter::new("INFO");
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
