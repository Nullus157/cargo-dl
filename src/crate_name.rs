
#[derive(Clone, Debug)]
pub(crate) struct CrateName(pub(crate) String);

#[derive(thiserror::Error, Clone, Debug, displaydoc::Display)]
pub(crate) enum ParseError {
    /// invalid character {0} at index {1}, crate names must be alphanumeric or `-_`
    InvalidCharacter(char, usize),
}

impl std::str::FromStr for CrateName {
    type Err = ParseError;

    #[fehler::throws(ParseError)]
    fn from_str(s: &str) -> Self {
        if let Some((index, char)) = s.chars().enumerate().find(|(_, c)| !c.is_alphanumeric() && !['-', '_'].contains(c)) {
            fehler::throw!(ParseError::InvalidCharacter(char, index));
        }
        CrateName(s.to_owned())
    }
}

impl std::fmt::Display for CrateName {
    #[fehler::throws(std::fmt::Error)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) {
        write!(f, "{}", self.0)?;
    }
}
