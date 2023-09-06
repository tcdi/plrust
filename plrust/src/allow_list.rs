use crate::gucs::PLRUST_ALLOWED_DEPENDENCIES;
use semver::{BuildMetadata, Comparator, Op, Version, VersionReq};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::iter::once;
use std::ops::Deref;
use std::path::PathBuf;
use std::str::FromStr;
use toml::Value;

#[derive(Debug, PartialEq)]
pub struct Dependency {
    name: String,
    versions: BTreeMap<OrderedVersionReq, toml::value::Table>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[repr(transparent)]
pub struct OrderedVersionReq(VersionReq);

pub type AllowList = BTreeMap<String, Dependency>;

impl Display for OrderedVersionReq {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for OrderedVersionReq {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        OrderedVersionReq::try_from(value.as_str())
    }
}

impl TryFrom<&str> for OrderedVersionReq {
    type Error = Error;

    /// only allow "*", exact, and bounded VersionReq values.  
    ///
    /// Versions with "prerelease" parts are not supported
    fn try_from(version: &str) -> Result<Self, Self::Error> {
        let vreq = VersionReq::parse(&version)
            .map_err(|e| Error::MalformedVersion(version.to_string(), e.to_string()))?;

        if validate_versionreq(&vreq, true) {
            Ok(OrderedVersionReq(vreq))
        } else {
            Err(Error::UnsupportedVersionReq(version.to_string()))
        }
    }
}

fn validate_versionreq(vreq: &VersionReq, require_exact: bool) -> bool {
    let has_prelrease = vreq.comparators.iter().any(|cmp| !cmp.pre.is_empty());
    if has_prelrease {
        // -shitshow versions not allowed (https://docs.rs/semver/latest/semver/struct.Prerelease.html#examples)
        false
    } else if vreq.comparators.len() == 0 {
        // it's a "*" version
        true
    } else if vreq.comparators.len() == 1 {
        if require_exact {
            // is it an exact version: "=x.y.z"
            vreq.comparators[0].op == Op::Exact
        } else {
            // if `require_exact` is false, then as long as it only has 1 comparator, it's valid
            true
        }
    } else if vreq.comparators.len() == 2
        && matches!(vreq.comparators[0].op, Op::Greater | Op::GreaterEq)
        && matches!(vreq.comparators[1].op, Op::Less | Op::LessEq)
    {
        // it's a bounded version: ">=a.b.c, <=x.y.z"
        true
    } else {
        false
    }
}

impl Deref for OrderedVersionReq {
    type Target = VersionReq;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Ord for OrderedVersionReq {
    /// Orders [`VersionReq`] values from smallest to largest, with "*" considered the smallest
    fn cmp(&self, other: &Self) -> Ordering {
        let self_vers = self.as_versions();
        let other_vers = other.as_versions();

        if self_vers.is_empty() {
            // '*' version is the smallest
            return Ordering::Less;
        } else if other_vers.is_empty() {
            // '*' version is the smallest
            return Ordering::Greater;
        } else {
            match self_vers[0].cmp(&other_vers[0]) {
                // how do the upper bounds of each side compare?
                Ordering::Equal if self_vers.len() > 1 && other_vers.len() > 1 => {
                    self_vers[1].cmp(&other_vers[1])
                }

                // how does our value compare to the upper bound of the other?
                Ordering::Equal if other_vers.len() > 1 => self_vers[0].cmp(&other_vers[1]),

                // lower bounds compared unequal, so that's our final answer
                ne => ne,
            }
        }
    }
}

impl PartialOrd for OrderedVersionReq {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn fake_version(cmp: &Comparator) -> Version {
    let mut version = Version {
        major: cmp.major,
        minor: cmp.minor.unwrap_or(0),
        patch: cmp.patch.unwrap_or(0),
        pre: cmp.pre.clone(),
        build: BuildMetadata::EMPTY,
    };

    if cmp.op == Op::Greater {
        version.patch += 1;
    } else if cmp.op == Op::Less {
        if version.patch > 0 {
            version.patch -= 1;
        } else if version.minor > 0 {
            version.minor -= 1;
        } else if version.major > 0 {
            version.major -= 1;
        }
    }

    version
}

impl OrderedVersionReq {
    /// Return `true` if our inner [`VersionReq`] matches the specified `other` [`VersionReq`].
    ///
    /// Our definition of "match" is that `other` needs to first pass [`validate_versionreq()`], and
    /// then we ensure that the lower (and possibly upper) bounds of each version match each other
    /// by pretending they're actually [`Version`]s.
    #[rustfmt::skip]
    fn matches_versionreq(&self, other: &VersionReq) -> bool {
        if !validate_versionreq(other, false) {
            return false;
        }

        let other_lower = other.comparators.get(0);
        let other_upper = other.comparators.get(1);

        match (other_lower, other_upper) {
            // user gave us a single version, so lets see if it matches us
            (Some(other_lower), None) => self.0.matches(&fake_version(other_lower)),

            // user gave us a bounded version, so lets make sure its lower and upper match ours
            (Some(other_lower), Some(other_upper)) => {
                let my_lower = self.0.comparators.get(0);
                let my_upper = self.0.comparators.get(1);

                match (my_lower, my_upper) {
                    (Some(my_lower), Some(my_upper)) => my_lower.matches(&fake_version(other_lower)) && my_upper.matches(&fake_version(other_upper)),
                    (Some(my_lower), None) => my_lower.matches(&fake_version(other_lower)) && my_lower.matches(&fake_version(other_upper)),
                    (None, _) =>  true,
                }
            }

            // user gave us a wildcard
            (None, _) =>  true
        }
    }

    /// Convert each part of the inner [`VersionReq`] into an exact [`Version`] the best we can.
    /// For unknown fields, we assume zero.
    fn as_versions(&self) -> Vec<Version> {
        self.0.comparators.iter().map(fake_version).collect()
    }
}

#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum Error {
    #[error("Unsupported value type: {0:?}")]
    UnsupportedValueType(Value),
    #[error("Dependency entry is missing the `version` attribute")]
    VersionMissing,
    #[error("Cannot read allow-list dependency file")]
    CannotReadAllowList,
    #[error("Not a TOML file")]
    NotATomlFile,
    #[error("`plrust.allowed_dependencies` is not set in `postgresql.conf`")]
    NotConfigured,
    #[error("The value of `plrust.allowed_dependencies` is not a valid path")]
    InvalidPath,
    #[error("`{0}` is malformed: {1}")]
    MalformedVersion(String, String),
    #[error("`{0}` is not permitted by the allow-list")]
    VersionNotPermitted(String),
    #[error("`{0}` is not a supported version requirement.  Use wildcard (`*`), exact (`=x.y.z`), or bounded ranges (`>=a.b.c, <=x.y.z`)")]
    UnsupportedVersionReq(String),
}

/// This struct describes name of the dependency, version of the dependency, features of the dependency that are enabled, and whether default features are enabled
struct AllowedDependency {
    name: String,
    ver: String,
    features: Vec<String>,
    default_features: bool,
}

/// A container struct to wrap around a vector of AllowedDependency used by plrust.allowed_dependencies function
pub struct AllowedDependencies {
    allowed_dependencies: Vec<AllowedDependency>,
}

/// A type alias to wrap around the tuple signature of plrust.allowed_dependencies function.
/// An AllowedDependency struct will be converted into this type.
pub type AllowedDependencyTuple = (String, String, Vec<String>, bool);

impl From<AllowedDependencies> for Vec<AllowedDependencyTuple> {
    fn from(a: AllowedDependencies) -> Vec<AllowedDependencyTuple> {
        a.allowed_dependencies
            .iter()
            .map(|item| {
                (
                    item.name.to_owned(),
                    item.ver.to_owned(),
                    item.features.to_owned(),
                    item.default_features,
                )
            })
            .collect()
    }
}

/// Get all the allowed dependencies entries as a AllowedDependencies struct
pub fn get_allowed_dependencies() -> AllowedDependencies {
    let allowlist: BTreeMap<String, Dependency> =
        load_allowlist().expect("Error loading dependency allow-list");
    let mut allowed_dependencies: Vec<AllowedDependency> = vec![];
    for dependency in allowlist.values() {
        let entries = get_entries_for_single_allowed_dependency(dependency);
        allowed_dependencies.extend(entries)
    }
    AllowedDependencies {
        allowed_dependencies,
    }
}

/// Multiple versions of a dependency can be allowed. Create an AllowedDependency struct for each version of the dependency
fn get_entries_for_single_allowed_dependency(dep: &Dependency) -> Vec<AllowedDependency> {
    let mut entries: Vec<AllowedDependency> = vec![];
    for version in dep.versions.values() {
        let name = dep.name.clone();
        let ver = version
            .get("version")
            .unwrap()
            .to_string()
            .replace("\"", "");
        let features: Vec<String> = match version.get("features") {
            Some(features) => features
                .as_array()
                .unwrap()
                .iter()
                .map(|f| f.as_str().unwrap().to_owned())
                .collect(),
            None => vec![],
        };
        let default_features = version
            .get("default-features")
            .map(|b| b.as_bool().unwrap())
            .unwrap_or(true);

        entries.push(AllowedDependency {
            name,
            ver,
            features,
            default_features,
        });
    }
    entries
}

impl TryFrom<(&str, Value)> for Dependency {
    type Error = Error;

    fn try_from(value: (&str, Value)) -> Result<Self, Self::Error> {
        let name = value.0.to_string();
        let value = value.1;

        match value {
            Value::String(version) => {
                let version = OrderedVersionReq::try_from(version)?;
                let mut table = toml::value::Table::new();
                table.insert("version".to_string(), Value::String(version.to_string()));

                Ok(Dependency {
                    name,
                    versions: BTreeMap::from_iter(once((version, table))),
                })
            }
            Value::Array(versions) => {
                let versions = versions
                    .into_iter()
                    .map(|value| match value {
                        Value::String(version) => {
                            let version = OrderedVersionReq::try_from(version)?;
                            let mut table = toml::value::Table::new();
                            table.insert("version".to_string(), Value::String(version.to_string()));

                            Ok((version, table))
                        }
                        Value::Table(table) => {
                            // value of the features field can only be in form of an array of string
                            if let Some(features) = table.get("features") {
                                features
                                    .as_array()
                                    .ok_or(Error::UnsupportedValueType(features.clone()))?
                                    .iter()
                                    .map(|val| {
                                        val.as_str()
                                            .ok_or(Error::UnsupportedValueType(val.clone()))?;
                                        Ok(())
                                    })
                                    .collect::<Result<Vec<()>, Error>>()?;
                            };
                            // value of the default-features field can only be in form of a boolean
                            if let Some(default_features) = table.get("default-features") {
                                default_features
                                    .as_bool()
                                    .ok_or(Error::UnsupportedValueType(default_features.clone()))?;
                            };
                            match table.get("version") {
                                Some(version) => {
                                    let version = version
                                        .as_str()
                                        .ok_or(Error::UnsupportedValueType(version.clone()))?;
                                    let version = OrderedVersionReq::try_from(version)?;
                                    Ok((version, table))
                                }
                                None => Err(Error::VersionMissing),
                            }
                        }
                        unsupported => Err(Error::UnsupportedValueType(unsupported)),
                    })
                    .collect::<Result<_, Error>>()?;

                Ok(Dependency { name, versions })
            }

            Value::Table(table) => {
                // value of the features field is required to be  an array of string
                if let Some(features) = table.get("features") {
                    features
                        .as_array()
                        .ok_or(Error::UnsupportedValueType(features.clone()))?
                        .iter()
                        .map(|val| {
                            val.as_str()
                                .ok_or(Error::UnsupportedValueType(val.clone()))?;
                            Ok(())
                        })
                        .collect::<Result<Vec<()>, Error>>()?;
                };
                // value of the default-features field is required to be a boolean
                if let Some(default_features) = table.get("default-features") {
                    default_features
                        .as_bool()
                        .ok_or(Error::UnsupportedValueType(default_features.clone()))?;
                };
                let version = table.get("version").ok_or(Error::VersionMissing)?;
                let version = version
                    .as_str()
                    .ok_or(Error::UnsupportedValueType(version.clone()))?;
                let version = OrderedVersionReq::try_from(version)?;
                Ok(Dependency {
                    name,
                    versions: BTreeMap::from_iter(once((version, table))),
                })
            }

            unknown => Err(Error::UnsupportedValueType(unknown)),
        }
    }
}

impl Dependency {
    /// Given some kind of version string, which could be a literal version such as `1.2.3`, or
    /// any [`semver::VersionReq`]-compatible version pattern, find the **largest** declared
    /// version entry that matches the specified `wanted_version`.
    ///
    /// This function will return the most constrained or exact version number it can, whether that
    /// is the caller's `wanted_version` or the VersionReq from the allow-list.  
    ///
    /// If the user asks for a literal version, such as "1.2.3" and there's a matching entry
    /// (regardless of the VersionReq pattern), the version returned is "=1.2.3".  
    ///
    /// If the user asks for some kind of imprecise or pattern version number then the first matching
    /// specification from the allow-list is returned.
    ///
    /// If the user asks for some kind of imprecise or pattern version and the matching allow-list
    /// VersionReq is a wildcard pattern, then the user's `wanted_version` is returned.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MalformedVersion`] if the `wanted_version` argument cannot be parsed into
    /// either a [`semver::Version`] or a [`semver::VersionReq`].
    ///
    /// Returns [`Error::VersionNotPermitted`] if the `wanted_version` argument does not match any of the
    /// declared versions in this [`Dependency`]
    pub fn get_dependency_entry(&self, wanted_version: &str) -> Result<Value, Error> {
        let wanted_version = wanted_version.trim().trim_start_matches('=');

        let ranked_versions = self.versions.iter().rev(); // we iterate our versions in `.rev()` order to find the largest matching version

        let mut table_entry = None;
        match Version::parse(wanted_version) {
            // it's a literal version number, such as 1.2.345
            Ok(wanted_version) => {
                for (version, table) in ranked_versions {
                    if version.matches(&wanted_version) {
                        // the version the user wants matches one of our allow-list versions
                        // so we use the user's version, prefixed with an "=" so it's an exact version
                        let mut table = table.clone();
                        table.insert(
                            "version".to_string(),
                            Value::String(format!("={wanted_version}")),
                        );

                        table_entry = Some(table);
                        break;
                    }
                }
            }

            // it's *probably* a VersionReq, so we'll handle this a little differently
            Err(_) => {
                let wanted_version = VersionReq::parse(wanted_version).map_err(|e| {
                    Error::MalformedVersion(wanted_version.to_string(), e.to_string())
                })?;

                for (version, table) in ranked_versions {
                    if version.matches_versionreq(&wanted_version)
                        || (!version.comparators.is_empty()
                            && wanted_version.matches(&fake_version(&version.comparators[0])))
                    {
                        let mut table = table.clone();

                        if version.to_string().contains('*') {
                            // the matching allow-list VersionReq is a wildcard pattern, so since the
                            // `wanted_version` matches it, use the (probably) more precise `wanted_version`
                            table.insert(
                                "version".to_string(),
                                Value::String(wanted_version.to_string()),
                            );
                        }

                        table_entry = Some(table);
                        break;
                    }
                }
            }
        }

        table_entry
            .map(|entry| Value::Table(entry))
            .ok_or_else(|| Error::VersionNotPermitted(wanted_version.to_string()))
    }
}

/// Reads the "dependency allow-list" from disk, at the path specified by the
/// `plrust.allowed_dependencies` GUC
pub fn load_allowlist() -> eyre::Result<AllowList> {
    let path = PathBuf::from_str(
        &PLRUST_ALLOWED_DEPENDENCIES
            .get()
            .map(|cstr| {
                cstr.to_str()
                    .expect("plrust.allowed_dependencies is not valid UTF8")
            })
            .ok_or(Error::NotConfigured)?,
    )
    .map_err(|_| Error::InvalidPath)?;

    let contents = std::fs::read_to_string(path).map_err(|_| Error::CannotReadAllowList)?;
    Ok(parse_allowlist(&contents)?)
}

pub(crate) fn parse_allowlist(contents: &str) -> Result<AllowList, Error> {
    let toml = toml::from_str::<toml::value::Table>(&contents).map_err(|_| Error::NotATomlFile)?;
    let mut allowed = AllowList::new();
    for (depname, value) in toml {
        let dependency = Dependency::try_from((depname.as_str(), value))?;
        allowed.insert(depname, dependency);
    }
    Ok(allowed)
}

#[cfg(test)]
mod tests {
    use crate::allow_list::{parse_allowlist, Error, OrderedVersionReq};
    use semver::VersionReq;

    const TOML: &str = r#"
a = [ "=1.2.3", "=3.0", ">=6.0.0, <=10", { version = "=2.4.5", features = [ "x", "y", "z" ] }, "*", ">=1.0.0, <5.0.0",">=1.0.0, <2.0.0", ">=2, <=4", "=2.99.99" ]
b = "*"
c = "=1.2.3"
d = { version = "=3.4.5", features = [ "x", "y", "z" ], default-features = false }
    "#;

    #[test]
    fn test_allowlist_parse_good() {
        assert!(parse_allowlist(TOML).is_ok());
    }

    #[rustfmt::skip]
    #[test]
    fn test_allowlist_parse_invalid_version_values() {
        assert_eq!(parse_allowlist("a = '=1.2.3.4.5'"), Err(Error::MalformedVersion("=1.2.3.4.5".to_string(), VersionReq::parse("=1.2.3.4.5").err().unwrap().to_string())));
        assert_eq!(parse_allowlist("a = '1.2.3'"), Err(Error::UnsupportedVersionReq("1.2.3".to_string())));
        assert_eq!(parse_allowlist("a = 42"), Err(Error::UnsupportedValueType(toml::Value::Integer(42))));
        assert_eq!(parse_allowlist("a = { features = ['a', 'b', 'c'] }"), Err(Error::VersionMissing));
        assert_eq!(parse_allowlist("a = { features = 42 }"), Err(Error::UnsupportedValueType(toml::Value::Integer(42))));
        assert_eq!(parse_allowlist("a = { features = [ 42 ] }"), Err(Error::UnsupportedValueType(toml::Value::Integer(42))));
        assert_eq!(parse_allowlist("a = { default-features = 'false' }"), Err(Error::UnsupportedValueType(toml::Value::String("false".to_string()))));
    }

    #[test]
    fn test_allowlist_star() -> eyre::Result<()> {
        let allowed = parse_allowlist(TOML)?;
        let dep = allowed.get("b").expect("no dependency for `b`");
        let versions = dep.versions.keys().cloned().collect::<Vec<_>>();
        assert_eq!(versions, vec![OrderedVersionReq::try_from("*")?]);
        dep.get_dependency_entry("*")?;
        Ok(())
    }

    #[test]
    fn test_allowlist_versionreq_sort() -> eyre::Result<()> {
        let allowed = parse_allowlist(TOML)?;
        let dep = allowed.get("a").expect("no dependency for `a`");
        let versions = dep.versions.keys().cloned().collect::<Vec<_>>();

        assert_eq!(
            versions,
            vec![
                OrderedVersionReq::try_from("*")?,
                OrderedVersionReq::try_from(">=1.0.0, <2.0.0")?,
                OrderedVersionReq::try_from(">=1.0.0, <5.0.0")?,
                OrderedVersionReq::try_from("=1.2.3")?,
                OrderedVersionReq::try_from(">=2, <=4")?,
                OrderedVersionReq::try_from("=2.4.5")?,
                OrderedVersionReq::try_from("=2.99.99")?,
                OrderedVersionReq::try_from("=3.0")?,
                OrderedVersionReq::try_from(">=6.0.0, <=10")?,
            ]
        );
        Ok(())
    }

    #[rustfmt::skip]
    #[test]
    fn test_allowlist_version_formats() -> eyre::Result<()> {
        assert_eq!(OrderedVersionReq::try_from("1.2.3"), Err(Error::UnsupportedVersionReq("1.2.3".to_string())));
        assert_eq!(OrderedVersionReq::try_from("^1.2.3"), Err(Error::UnsupportedVersionReq("^1.2.3".to_string())));
        assert_eq!(OrderedVersionReq::try_from("~1.2.3"), Err(Error::UnsupportedVersionReq("~1.2.3".to_string())));
        assert_eq!(OrderedVersionReq::try_from(">1.2.3"), Err(Error::UnsupportedVersionReq(">1.2.3".to_string())));
        assert_eq!(OrderedVersionReq::try_from(">=1.2.3"), Err(Error::UnsupportedVersionReq(">=1.2.3".to_string())));
        assert_eq!(OrderedVersionReq::try_from("<1.2.3"), Err(Error::UnsupportedVersionReq("<1.2.3".to_string())));
        assert_eq!(OrderedVersionReq::try_from("<=1.2.3"), Err(Error::UnsupportedVersionReq("<=1.2.3".to_string())));
        assert_eq!(OrderedVersionReq::try_from("<4.5.6, >1.2.3"), Err(Error::UnsupportedVersionReq("<4.5.6, >1.2.3".to_string()))); // reverse range
        
        assert_eq!(OrderedVersionReq::try_from("=1.2.3"), Ok(OrderedVersionReq::try_from("=1.2.3")?));
        assert_eq!(OrderedVersionReq::try_from(">1.2.3, <4.5.6"), Ok(OrderedVersionReq::try_from(">1.2.3, <4.5.6")?));
        assert_eq!(OrderedVersionReq::try_from(">=1.2.3, <=4.5.6"), Ok(OrderedVersionReq::try_from(">=1.2.3, <=4.5.6")?));
        assert_eq!(OrderedVersionReq::try_from("*"), Ok(OrderedVersionReq::try_from("*")?));
        Ok(())
    }
}
