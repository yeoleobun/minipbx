use std::fmt;

use super::text::split_unquoted;

/// `;name[=value]` parameters, in order. Names compare case-insensitively.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Params(Vec<(String, Option<String>)>);

impl Params {
    /// Parses `a=1;b;c=2` (without a leading `;`).
    pub fn parse(s: &str) -> Self {
        let params = split_unquoted(s, b';')
            .into_iter()
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(|p| match p.split_once('=') {
                Some((k, v)) => (k.trim().to_owned(), Some(v.trim().to_owned())),
                None => (p.to_owned(), None),
            })
            .collect();
        Self(params)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.0.iter().any(|(k, _)| k.eq_ignore_ascii_case(name))
    }

    /// The value of `name`; `None` if absent or a flag without a value.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .and_then(|(_, v)| v.as_deref())
    }

    pub fn set(&mut self, name: &str, value: Option<&str>) {
        let value = value.map(str::to_owned);
        match self
            .0
            .iter_mut()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
        {
            Some(entry) => entry.1 = value,
            None => self.0.push((name.to_owned(), value)),
        }
    }

    pub fn remove(&mut self, name: &str) {
        self.0.retain(|(k, _)| !k.eq_ignore_ascii_case(name));
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, Option<&str>)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v.as_deref()))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Writes each parameter with a leading `;`.
impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (k, v) in &self.0 {
            match v {
                Some(v) => write!(f, ";{k}={v}")?,
                None => write!(f, ";{k}")?,
            }
        }
        Ok(())
    }
}
