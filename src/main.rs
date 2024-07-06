use std::fs::{copy, create_dir_all};

use anyhow::{Context, Result};
use std::os::unix::fs::chroot;
use tempfile::tempdir;

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let command = &args[3];
    let command_args = &args[4..];

    let temp_directory = tempdir()?;
    create_dir_all(temp_directory.path().join("dev/null"))?;
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
