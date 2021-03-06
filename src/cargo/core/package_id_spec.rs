use std::fmt;
use semver::Version;
use url::{mod, Url, UrlParser};

use core::PackageId;
use util::{CargoResult, ToUrl, Require, human, ToSemver};

#[deriving(Clone, PartialEq, Eq)]
pub struct PackageIdSpec {
    name: String,
    version: Option<Version>,
    url: Option<Url>,
}

impl PackageIdSpec {
    pub fn parse(spec: &str) -> CargoResult<PackageIdSpec> {
        if spec.contains("/") {
            match spec.to_url() {
                Ok(url) => return PackageIdSpec::from_url(url),
                Err(..) => {}
            }
            if !spec.contains("://") {
                match url(format!("cargo://{}", spec).as_slice()) {
                    Ok(url) => return PackageIdSpec::from_url(url),
                    Err(..) => {}
                }
            }
        }
        let mut parts = spec.as_slice().splitn(1, ':');
        let name = parts.next().unwrap();
        let version = match parts.next() {
            Some(version) => Some(try!(Version::parse(version).map_err(human))),
            None => None,
        };
        for ch in name.chars() {
            if !ch.is_alphanumeric() && ch != '_' && ch != '-' {
                return Err(human(format!("invalid character in pkgid `{}`: `{}`",
                                         spec, ch)))
            }
        }
        Ok(PackageIdSpec {
            name: name.to_string(),
            version: version,
            url: None,
        })
    }

    pub fn from_package_id(package_id: &PackageId) -> PackageIdSpec {
        PackageIdSpec {
            name: package_id.get_name().to_string(),
            version: Some(package_id.get_version().clone()),
            url: Some(package_id.get_source_id().url.clone()),
        }
    }

    fn from_url(mut url: Url) -> CargoResult<PackageIdSpec> {
        if url.query.is_some() {
            return Err(human(format!("cannot have a query string in a pkgid: {}",
                             url)));
        }
        let frag = url.fragment.take();
        let (name, version) = {
            let path = try!(url.path().require(|| {
                human(format!("pkgid urls must have a path: {}", url))
            }));
            let path_name = try!(path.last().require(|| {
                human(format!("pkgid urls must have at least one path \
                               component: {}", url))
            }));
            match frag {
                Some(fragment) => {
                    let mut parts = fragment.as_slice().splitn(1, ':');
                    let name_or_version = parts.next().unwrap();
                    match parts.next() {
                        Some(part) => {
                            let version = try!(part.to_semver().map_err(human));
                            (name_or_version.to_string(), Some(version))
                        }
                        None => {
                            if name_or_version.char_at(0).is_alphabetic() {
                                (name_or_version.to_string(), None)
                            } else {
                                let version = try!(name_or_version.to_semver()
                                                                  .map_err(human));
                                (path_name.to_string(), Some(version))
                            }
                        }
                    }
                }
                None => (path_name.to_string(), None),
            }
        };
        Ok(PackageIdSpec {
            name: name,
            version: version,
            url: Some(url),
        })
    }

    pub fn get_name(&self) -> &str { self.name.as_slice() }
    pub fn get_version(&self) -> Option<&Version> { self.version.as_ref() }
    pub fn get_url(&self) -> Option<&Url> { self.url.as_ref() }

    pub fn matches(&self, package_id: &PackageId) -> bool {
        if self.get_name() != package_id.get_name() { return false }

        match self.version {
            Some(ref v) => if v != package_id.get_version() { return false },
            None => {}
        }

        match self.url {
            Some(ref u) => *u == package_id.get_source_id().url,
            None => true
        }
    }
}

fn url(s: &str) -> url::ParseResult<Url> {
    return UrlParser::new().scheme_type_mapper(mapper).parse(s);

    fn mapper(scheme: &str) -> url::SchemeType {
        if scheme == "cargo" {
            url::RelativeScheme(1)
        } else {
            url::whatwg_scheme_type_mapper(scheme)
        }
    }

}

impl fmt::Show for PackageIdSpec {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut printed_name = false;
        match self.url {
            Some(ref url) => {
                if url.scheme.as_slice() == "cargo" {
                    try!(write!(f, "{}/{}", url.host().unwrap(),
                                url.path().unwrap().connect("/")));
                } else {
                    try!(write!(f, "{}", url));
                }
                if url.path().unwrap().last().unwrap() != &self.name {
                    printed_name = true;
                    try!(write!(f, "#{}", self.name));
                }
            }
            None => { printed_name = true; try!(write!(f, "{}", self.name)) }
        }
        match self.version {
            Some(ref v) => {
                try!(write!(f, "{}{}", if printed_name {":"} else {"#"}, v));
            }
            None => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use core::{PackageId, SourceId};
    use super::{PackageIdSpec, url};
    use semver::Version;

    #[test]
    fn good_parsing() {
        fn ok(spec: &str, expected: PackageIdSpec) {
            let parsed = PackageIdSpec::parse(spec).unwrap();
            assert_eq!(parsed, expected);
            assert_eq!(parsed.to_string().as_slice(), spec);
        }

        ok("http://crates.io/foo#1.2.3", PackageIdSpec {
            name: "foo".to_string(),
            version: Some(Version::parse("1.2.3").unwrap()),
            url: Some(url("http://crates.io/foo").unwrap()),
        });
        ok("http://crates.io/foo#bar:1.2.3", PackageIdSpec {
            name: "bar".to_string(),
            version: Some(Version::parse("1.2.3").unwrap()),
            url: Some(url("http://crates.io/foo").unwrap()),
        });
        ok("crates.io/foo", PackageIdSpec {
            name: "foo".to_string(),
            version: None,
            url: Some(url("cargo://crates.io/foo").unwrap()),
        });
        ok("crates.io/foo#1.2.3", PackageIdSpec {
            name: "foo".to_string(),
            version: Some(Version::parse("1.2.3").unwrap()),
            url: Some(url("cargo://crates.io/foo").unwrap()),
        });
        ok("crates.io/foo#bar", PackageIdSpec {
            name: "bar".to_string(),
            version: None,
            url: Some(url("cargo://crates.io/foo").unwrap()),
        });
        ok("crates.io/foo#bar:1.2.3", PackageIdSpec {
            name: "bar".to_string(),
            version: Some(Version::parse("1.2.3").unwrap()),
            url: Some(url("cargo://crates.io/foo").unwrap()),
        });
        ok("foo", PackageIdSpec {
            name: "foo".to_string(),
            version: None,
            url: None,
        });
        ok("foo:1.2.3", PackageIdSpec {
            name: "foo".to_string(),
            version: Some(Version::parse("1.2.3").unwrap()),
            url: None,
        });
    }

    #[test]
    fn bad_parsing() {
        assert!(PackageIdSpec::parse("baz:").is_err());
        assert!(PackageIdSpec::parse("baz:1.0").is_err());
        assert!(PackageIdSpec::parse("http://baz:1.0").is_err());
        assert!(PackageIdSpec::parse("http://#baz:1.0").is_err());
    }

    #[test]
    fn matching() {
        let sid = SourceId::for_central().unwrap();
        let foo = PackageId::new("foo", "1.2.3", &sid).unwrap();
        let bar = PackageId::new("bar", "1.2.3", &sid).unwrap();

        assert!( PackageIdSpec::parse("foo").unwrap().matches(&foo));
        assert!(!PackageIdSpec::parse("foo").unwrap().matches(&bar));
        assert!( PackageIdSpec::parse("foo:1.2.3").unwrap().matches(&foo));
        assert!(!PackageIdSpec::parse("foo:1.2.2").unwrap().matches(&foo));
    }
}
