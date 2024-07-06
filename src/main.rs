use std::{
    collections::HashMap,
    fs::{copy, create_dir_all},
};

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use reqwest::header;
use std::os::unix::fs::chroot;
use structs::{AuthResponse, ImageManifest};
use tempfile::tempdir;

mod structs;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let image_arg_parts = &args[2].split(":").collect::<Vec<&str>>();
    let image = image_arg_parts.get(0).unwrap();
    let version = image_arg_parts.get(1).unwrap_or(&"latest");
    let command = &args[3];
    let command_args = &args[4..];

    let client = reqwest::Client::new();

    let scope = format!("repository:library/{image}:pull");
    let auth_params: HashMap<&str, &str> =
        HashMap::from([("service", "registry.docker.io"), ("scope", scope.as_str())]);
    let auth_url =
        reqwest::Url::parse_with_params(&format!("https://auth.docker.io/token"), auth_params)?;
    let token = client
        .get(auth_url)
        .send()
        .await?
        .json::<AuthResponse>()
        .await?
        .token;

    let manifest_url =
        format!("https://registry.hub.docker.com/v2/library/{image}/manifests/{version}");
    let digest = client
        .get(manifest_url)
        .bearer_auth(token.clone())
        .header(
            header::ACCEPT,
            "application/vnd.docker.distribution.manifest.v2+json",
        )
        .send()
        .await?
        .json::<ImageManifest>()
        .await?
        .layers[0]
        .digest
        .clone();

    let blob_url = format!("https://registry.hub.docker.com/v2/library/{image}/blobs/{digest}");
    let blob = client
        .get(blob_url)
        .bearer_auth(token.clone())
        .send()
        .await?
        .bytes()
        .await?;

    let tar = GzDecoder::new(&blob[..]);

    let temp_directory = tempdir()?;
    tar::Archive::new(tar).unpack(temp_directory.path())?;

    create_dir_all(temp_directory.path().join("dev/null"))?;

    // copying the command binary is only needed for previous stages
    let (command_path, _) = command.split_at(command.rfind('/').unwrap());
    create_dir_all(
        temp_directory
            .path()
            .join(command_path.strip_prefix("/").unwrap_or(command_path)),
    )?;
    copy(
        command,
        &temp_directory
            .path()
            .join(command.strip_prefix("/").unwrap_or(command)),
    )?;

    chroot(temp_directory.path())?;
    unsafe {
        libc::unshare(libc::CLONE_NEWPID);
    }

    let output = std::process::Command::new(command)
        .args(command_args)
        .output()
        .with_context(|| {
            format!(
                "Tried to run '{}' with arguments {:?}",
                command, command_args
            )
        })?;

    let std_out = std::str::from_utf8(&output.stdout)?;
    print!("{}", std_out);
    let std_err = std::str::from_utf8(&output.stderr)?;
    eprint!("{}", std_err);
    if output.status.code().is_some() {
        std::process::exit(output.status.code().unwrap());
    }

    Ok(())
}
