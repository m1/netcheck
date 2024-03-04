use core::fmt;
use std::ffi::OsString;
use std::str::FromStr;

use reqwest::Url;

use crate::runner::url::vec_to_string;

/// Target is a struct that holds the name and urls of a target.
#[derive(Clone, PartialEq)]
pub struct Target {
    pub name: String,
    pub urls: Vec<Url>,
}

impl Target {
    /// Create a new Target.
    ///
    /// # Arguments
    /// * `name` - A string that holds the name of the target.
    /// * `urls` - A vector of Urls that holds the urls of the target.
    ///
    /// # Example
    /// ```
    /// # use netcheck::runner::Target;
    /// # use reqwest::Url;
    ///
    /// let target = Target::new("external".to_string(), vec![Url::parse("https://example.com").unwrap()]);
    ///
    /// ```
    pub fn new(name: String, urls: Vec<Url>) -> Self {
        Self { name, urls }
    }
}

impl FromStr for Target {
    type Err = ();

    /// Create a Target from a string.
    ///
    /// # Arguments
    ///
    /// * `str`: A string that holds the name and urls of the target.
    ///
    /// returns: Result<Target, <Target as FromStr>::Err>
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::str::FromStr;
    /// # use netcheck::runner::Target;
    /// let target = Target::from_str("external=https://example.com,https://example2.com").unwrap();
    /// ```
    fn from_str(str: &str) -> Result<Self, Self::Err> {
        match str.split_once("=") {
            Some((name, urls)) => {
                let urls = urls.split(",").map(|url| {
                    let u = if url.trim().starts_with("http") {
                        url.trim().to_string()
                    } else {
                        format!("https://{}", url.trim())
                    };

                    Url::parse(u.as_str()).unwrap()
                }).collect();
                Ok(Target::new(name.to_string(), urls))
            }
            None => Err(()),
        }
    }
}

impl From<OsString> for Target {
    fn from(str: OsString) -> Self { Target::from_str(str.to_str().unwrap()).unwrap() }
}

impl fmt::Debug for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Target {{ name: {}, urls: [{:?}] }}", self.name, vec_to_string(self.urls.clone()))
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_target_new() {
        let target = Target::new("external".to_string(), vec![Url::parse("https://example.com").expect("failed to parse url")]);
        assert_eq!(target.name, "external");
        assert_eq!(target.urls[0], Url::parse("https://example.com").expect("failed to parse url"));
    }

    #[test]
    fn test_target_from_str() {
        let target = Target::from_str("external=https://example.com").expect("failed to parse");
        assert_eq!(target.name, "external");
        assert_eq!(target.urls[0], Url::parse("https://example.com").expect("failed to parse url"));
    }

    #[test]
    fn test_target_from_str_append_https() {
        let target = Target::from_str("external=example.com").expect("failed to parse");
        assert_eq!(target.name, "external");
        assert_eq!(target.urls[0], Url::parse("https://example.com").expect("failed to parse url"));
    }

    #[test]
    fn test_target_from_str_fail() {
        let target = Target::from_str("external");
        assert_eq!(target.err(), Some(()));
    }

    #[test]
    fn test_target_from_os_string() {
        let target = Target::from(OsString::from("external=https://example.com"));
        assert_eq!(target.name, "external");
        assert_eq!(target.urls[0], Url::parse("https://example.com").expect("failed to parse url"));
    }

    #[test]
    fn test_target_debug() {
        let target = Target::new("external".to_string(), vec![Url::parse("https://example.com").expect("failed to parse url")]);
        assert_eq!(format!("{:?}", target), "Target { name: external, urls: [\"https://example.com/\"] }");
    }
}
