[![version-badge][]][version] [![license-badge][]][license] [![rust-version-badge][]][rust-version]

Cargo subcommand for downloading crate sources

```
Cargo subcommand for downloading crate sources

Usage: cargo dl [OPTIONS] <CRATE[@VERSION_REQ]>...

Arguments:
  <CRATE[@VERSION_REQ]>...
          The crate(s) to download.

          Optionally including which version of the crate to download after `@`,
          in the standard semver constraint format used in Cargo.toml. If
          unspecified the newest non-prerelease, non-yanked version will be
          fetched.

Options:
  -x, --extract
          Specify this flag to have the crate extracted automatically.

          Note that unless changed via the --output flag, this will extract the
          files to a new subdirectory bearing the name of the downloaded crate
          archive.

  -o, --output <OUTPUT>
          Normally, the compressed crate is written to a file (or directory if
          --extract is used) based on its name and version.  This flag allows to
          change that by providing an explicit file or directory path. (Only
          when downloading a single crate)

      --allow-yanked
          Allow yanked versions to be chosen

      --no-cache
          Disable checking cargo cache for the crate file

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```


[version-badge]: https://img.shields.io/crates/v/cargo-dl.svg?style=flat-square
[version]: https://crates.io/crates/cargo-dl
[license-badge]: https://img.shields.io/crates/l/cargo-dl.svg?style=flat-square
[license]: #license
[rust-version-badge]: https://img.shields.io/badge/rust-latest%20stable-blueviolet.svg?style=flat-square
[rust-version]: #rust-version-policy

# Rust Version Policy

This crate only supports the current stable version of Rust, patch releases may
use new features at any time.

# License

Licensed under either of

 * Apache License, Version 2.0 (`LICENSE-APACHE` or <http://www.apache.org/licenses/LICENSE-2.0>)
 * MIT license (`LICENSE-MIT` or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be dual licensed as above, without any
additional terms or conditions.
