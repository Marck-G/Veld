use std::{fs, path::Path};

use rstest::*;

use crate::{
    constants,
    structs::{Manifest, PackageMetadata},
    tools::generate_config::config,
};

struct TestContext {
    pub conf_path: String,
}

impl TestContext {
    pub fn setup() -> Self {
        let path = Path::new("./");
        if path.join(constants::FILE_BUILD_NAME).exists() {
            fs::remove_file(path).ok();
        }
        Self {
            conf_path: "./".into(),
        }
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        let path = Path::new(self.conf_path.as_str());
        if path.join(constants::FILE_BUILD_NAME).exists() {
            fs::remove_file(path.join(constants::FILE_BUILD_NAME)).unwrap();
        }
    }
}

#[fixture]
fn ctx() -> TestContext {
    TestContext::setup()
}

#[rstest]
fn test_veld_file_creation(ctx: TestContext) {
    let metadata = PackageMetadata {
        name: "test-package".into(),
        version: semver::Version::new(1, 0, 0),
        authors: None,
        license: None,
        description: None,
    };
    dbg!(&metadata);
    let result = config::generate(ctx.conf_path.clone(), metadata.clone());
    dbg!(&result);
    assert!(result.is_ok());
    assert!(Path::new(ctx.conf_path.as_str())
        .join(constants::FILE_BUILD_NAME)
        .exists());
    let ctn =
        fs::read_to_string(Path::new(ctx.conf_path.as_str()).join(constants::FILE_BUILD_NAME))
            .unwrap();
    let manifest: Manifest = toml::from_str(&ctn).unwrap();
    assert_eq!(manifest.package.name, metadata.name);
    assert_eq!(manifest.package.version, metadata.version);
    drop(ctx);
}
