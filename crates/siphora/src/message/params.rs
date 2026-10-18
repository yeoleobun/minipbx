use std::fmt;

/// `;name[=value]` parameters, in order. Names compare case-insensitively.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Params(Vec<(String, Option<String>)>);

impl Params {
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

impl FromIterator<(String, Option<String>)> for Params {
    fn from_iter<I: IntoIterator<Item = (String, Option<String>)>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
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
