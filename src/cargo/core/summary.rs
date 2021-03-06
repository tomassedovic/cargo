use std::collections::HashMap;

use semver::Version;
use core::{Dependency, PackageId, SourceId};

use util::{CargoResult, human};

/// Summaries are cloned, and should not be mutated after creation
#[deriving(Show,Clone,PartialEq)]
pub struct Summary {
    package_id: PackageId,
    dependencies: Vec<Dependency>,
    features: HashMap<String, Vec<String>>,
}

impl Summary {
    pub fn new(pkg_id: PackageId,
               dependencies: Vec<Dependency>,
               features: HashMap<String, Vec<String>>) -> CargoResult<Summary> {
        for dep in dependencies.iter() {
            if features.find_equiv(&dep.get_name()).is_some() {
                return Err(human(format!("Features and dependencies cannot have \
                                          the same name: `{}`", dep.get_name())))
            }
            if dep.is_optional() && !dep.is_transitive() {
                return Err(human(format!("Dev-dependencies are not allowed \
                                          to be optional: `{}`",
                                          dep.get_name())))
            }
        }
        for (feature, list) in features.iter() {
            for dep in list.iter() {
                if features.find_equiv(&dep.as_slice()).is_some() { continue }
                let d = dependencies.iter().find(|d| {
                    d.get_name() == dep.as_slice()
                });
                match d {
                    Some(d) => {
                        if d.is_optional() { continue }
                        return Err(human(format!("Feature `{}` depends on `{}` \
                                                  which is not an optional \
                                                  dependency.\nConsider adding \
                                                  `optional = true` to the \
                                                  dependency", feature, dep)))
                    }
                    None => {
                        return Err(human(format!("Feature `{}` includes `{}` \
                                                  which is neither a dependency \
                                                  nor another feature",
                                                  feature, dep)))
                    }
                }
            }
        }
        Ok(Summary {
            package_id: pkg_id,
            dependencies: dependencies,
            features: features,
        })
    }

    pub fn get_package_id(&self) -> &PackageId {
        &self.package_id
    }

    pub fn get_name(&self) -> &str {
        self.get_package_id().get_name()
    }

    pub fn get_version(&self) -> &Version {
        self.get_package_id().get_version()
    }

    pub fn get_source_id(&self) -> &SourceId {
        self.package_id.get_source_id()
    }

    pub fn get_dependencies(&self) -> &[Dependency] {
        self.dependencies.as_slice()
    }

    pub fn get_features(&self) -> &HashMap<String, Vec<String>> {
        &self.features
    }
}

pub trait SummaryVec {
    fn names(&self) -> Vec<String>;
}

impl SummaryVec for Vec<Summary> {
    // TODO: Move to Registry
    fn names(&self) -> Vec<String> {
        self.iter().map(|summary| summary.get_name().to_string()).collect()
    }

}
