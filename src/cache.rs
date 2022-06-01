use std::path::PathBuf;
use anyhow::{anyhow, Context, Error};
use crates_index::{Index, Version};

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
#[fn_error_context::context("finding cache dir for registry {}", index.path().display())]
pub(crate) fn find_cache_dir(index: &Index) -> std::path::PathBuf {
    let mut components = index.path().components();
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
    index.path().display(),
)]
pub(crate) fn lookup(index: &Index, version: &Version) -> PathBuf {
    let cache_dir = find_cache_dir(index)?;

    let cache_file = cache_dir.join(format!("{}-{}.crate", version.name(), version.version()));
    if !cache_file.exists() {
        fehler::throw!(anyhow!("cache file {} does not exist", cache_file.display()));
    }

    if &sha256_file(&cache_file)? != version.checksum() {
        fehler::throw!(anyhow!("invalid checksum"));
    }

    cache_file
}
