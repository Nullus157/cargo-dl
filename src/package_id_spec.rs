use crate::{crate_name, CrateName};

#[derive(Clone, Debug)]
pub(crate) struct PackageIdSpec {
    pub(crate) name: CrateName,
    pub(crate) version_req: Option<semver::VersionReq>,
}

#[derive(thiserror::Error, Debug, displaydoc::Display)]
pub(crate) enum ParseError {
    /// invalid crate name '{1}'
    CrateName(#[source] crate_name::ParseError, String),
    /// invalid version request '{1}'
    VersionReq(#[source] semver::Error, String),
}

impl std::str::FromStr for PackageIdSpec {
    type Err = ParseError;

    #[culpa::throws(ParseError)]
    fn from_str(s: &str) -> Self {
        let parse_crate_name = |s: &str| {
            s.parse::<CrateName>()
                .map_err(|e| ParseError::CrateName(e, s.to_owned()))
        };
        if let Some(i) = s.find('@') {
            let v = &s[(i + 1)..];
            Self {
                name: parse_crate_name(&s[..i])?,
                version_req: Some(
                    v.parse()
                        .map_err(|e| ParseError::VersionReq(e, v.to_owned()))?,
                ),
            }
        } else {
            Self {
                name: parse_crate_name(s)?,
                version_req: None,
            }
        }
    }
}

impl std::fmt::Display for PackageIdSpec {
    #[culpa::throws(std::fmt::Error)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) {
        if let PackageIdSpec {
            name,
            version_req: Some(version_req),
        } = self
        {
            f.pad(&format!("{name}@{version_req}"))?;
        } else {
            write!(f, "{}", self.name)?;
        }
    }
}
