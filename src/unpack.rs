use anyhow::{anyhow, Context, Error};
use std::path::{Component, Path};

#[fehler::throws]
pub(crate) fn unpack(
    version: &crates_index::Version,
    mut archive: tar::Archive<impl std::io::Read>,
    output: impl AsRef<Path>,
) {
    let base = format!("{}-{}", version.name(), version.version());
    let output = output.as_ref();
    std::fs::create_dir_all(&output)?;
    let mut entries = archive.entries()?;
    while let Some(mut entry) = entries.next().transpose()? {
        let path = entry.path()?;
        if path.components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        }) {
            fehler::throw!(anyhow!(
                "a file in the archive ({}) contains a .. or root segment",
                path.display()
            ));
        }
        let dst = output.join(path.strip_prefix(&base)?);
        std::fs::create_dir_all(dst.parent().context("file missing parent")?)?;
        entry.unpack(dst)?;
    }
}
