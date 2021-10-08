use std::path::Path;
use std::fs::File;
extern crate serde;
use serde::Deserialize;

#[derive(thiserror::Error, Debug)]
enum DockerConfigError {
    #[error("could not retrieve the runc status")]
    Status(#[from] std::io::Error),

    #[error("could not format")]
    Format(#[from] std::fmt::Error),

    #[error("could not convert bytes to utf-8 string")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("could not parse JSON")]
    Json(#[from] serde_json::Error),

    #[error("could not find sandbox container bundle directory")]
    BundleDirError,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Mount {
    destination: String,
    r#type: String,
    source: String,
    options: Vec<String>
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Mounts {
    mounts: Vec<Mount>
}

//noinspection RsMainFunctionNotFound
pub fn config<P: AsRef<std::path::Path>>(
    container_bundle: P,
) -> Result<Option<std::string::String>> {
    let bundle_path = container_bundle.as_ref();
    let config_path = bundle_path.join("config.json");
    let f = std::fs::File::open(config_path)?;
    let r = std::io::BufReader::new(f);

    let m: Mounts = serde_json::from_reader(r).expect("JSON was not well-formatted");

    for test in m.mounts {
        let source: Vec<&str> = test.source.split('/').collect();
        if source.len() > 1 {
            if source[ source.len() - 1 ] == "hostname" {
                let config_v2= str::replace(&test.source, "hostname", "config.v2.json");
                return Ok(Some(config_v2));
            }
        }
    }

    Ok(None)
}

use serde_json::{Result, Value};
use serde_json::map::Values;

pub fn label(docker_bundle: &str) -> Result<lockc::bpfstructs::container_policy_level> {
    let config_path = docker_bundle.as_ref();
    let f = std::fs::File::open(config_path)?;
    let r = std::io::BufReader::new(f);

    let l: Value = serde_json::from_reader(r).expect("JSON was not well-formatted");

    let x = l["Config"]["Labels"]["org.lockc.policy"].as_str();

    match x {
        Some(x) => match x.as_str() {
            "restricted" => {
                Ok(lockc::bpfstructs::container_policy_level_POLICY_LEVEL_RESTRICTED)
            }
            "baseline" => Ok(lockc::bpfstructs::container_policy_level_POLICY_LEVEL_BASELINE),
            "privileged" => {
                Ok(lockc::bpfstructs::container_policy_level_POLICY_LEVEL_PRIVILEGED)
            }
            _ => Ok(lockc::bpfstructs::container_policy_level_POLICY_LEVEL_BASELINE)
        }
        None => Ok(lockc::bpfstructs::container_policy_level_POLICY_LEVEL_BASELINE),
    }

    Ok(())
}

fn main() {}