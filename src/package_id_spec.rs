use crate::{crate_name, CrateName};

#[derive(Clone, Debug)]
pub(crate) struct PackageIdSpec {
    pub(crate) name: CrateName,
    pub(crate) version_req: Option<semver::VersionReq>,
}

#[derive(thiserror::Error, Debug, displaydoc::Display)]
pub(crate) enum ParseError {
    /// invalid crate name
    CrateName(#[from] #[source] crate_name::ParseError),
    /// invalid version request
    VersionReq(#[from] #[source] semver::Error),
}

impl std::str::FromStr for PackageIdSpec {
    type Err = ParseError;

    #[fehler::throws(ParseError)]
    fn from_str(s: &str) -> Self {
        if let Some(i) = s.find(':') {
            Self {
                name: s[..i].parse()?,
                version_req: Some(s[(i + 1)..].parse()?),
            }
        } else {
            Self {
                name: s.parse()?,
                version_req: None,
            }
        }
    }
}

impl std::fmt::Display for PackageIdSpec {
    #[fehler::throws(std::fmt::Error)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) {
        write!(f, "{}", self.name)?;
        if let Some(version_req) = &self.version_req {
            write!(f, ":{}", version_req)?;
        }
    }
}
