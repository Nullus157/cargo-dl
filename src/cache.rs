use anyhow::{anyhow, Context, Error};
use crates_index::Version;
use std::path::PathBuf;

#[fehler::throws]
#[fn_error_context::context("hashing {}", path.as_ref().display())]
fn sha256_file(path: impl AsRef<std::path::Path>) -> [u8; 32] {
    use sha2::Digest;

    let mut file = std::fs::File::open(path.as_ref())?;
    let mut hasher = sha2::Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;

    *hasher.finalize().as_ref()
}

#[fehler::throws]
#[fn_error_context::context("finding cache dir for registry {}", url)]
pub(crate) fn find_cache_dir(url: &str) -> std::path::PathBuf {
    let (path, _) = crates_index::local_path_and_canonical_url(url, None)?;
    let mut components = path.components();

    let dirname = components
        .next_back()
        .context("missing index dirname")?
        .as_os_str();
    let parent = components
        .next_back()
        .context("missing index parent")?
        .as_os_str();
    let grandparent = components
        .next_back()
        .context("missing index grandparent")?
        .as_os_str();

    if parent != "index" || grandparent != "registry" {
        fehler::throw!(anyhow!("unexpected registry cache structure"));
    }

    let cache_path = components
        .as_path()
        .join("registry")
        .join("cache")
        .join(dirname);

    if !cache_path.exists() {
        fehler::throw!(anyhow!("cache dir {} does not exist", cache_path.display()));
    }

    cache_path
}

#[fehler::throws]
#[fn_error_context::context(
    "failed finding cached file for {}@{} in registry {}",
    version.name(),
    version.version(),
    url,
)]
pub(crate) fn lookup(url: &str, version: &Version) -> PathBuf {
    let cache_dir = find_cache_dir(url)?;

    let cache_file = cache_dir.join(format!("{}-{}.crate", version.name(), version.version()));
    if !cache_file.exists() {
        fehler::throw!(anyhow!(
            "cache file {} does not exist",
            cache_file.display()
        ));
    }

    let calculated_checksum = sha256_file(&cache_file)?;
    if &calculated_checksum != version.checksum() {
        fehler::throw!(anyhow!(
            "invalid checksum, expected {} but got {}",
            hex::encode(version.checksum()),
            hex::encode(calculated_checksum)
        ));
    }

    cache_file
}

#[fehler::throws]
pub(crate) fn lookup_all(urls: &[&str], version: &Version) -> PathBuf {
    for url in urls {
        match lookup(url, version) {
            Ok(path) => return path,
            Err(err) => tracing::debug!("{err:?}"),
        }
    }
    fehler::throw!(anyhow!(
        "failed finding cached file for {}@{} in registries {:?}",
        version.name(),
        version.version(),
        urls
    ));
}
