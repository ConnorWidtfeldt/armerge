use crate::arbuilder::ArBuilder;
use crate::MergeError;
use crate::MergeError::ExternalToolLaunchError;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use tracing::{debug, info};

#[derive(Debug)]
pub struct MacArBuilder {
    output_path: PathBuf,
    obj_paths: Vec<PathBuf>,
    closed: bool,
}

impl ArBuilder for MacArBuilder {
    fn append_obj(&mut self, path: &Path) -> Result<(), MergeError> {
        self.obj_paths.push(path.to_owned());
        Ok(())
    }

    fn close(mut self: Box<Self>) -> Result<(), MergeError> {
        self.write_obj()
    }
}

impl MacArBuilder {
    pub fn new(path: &Path) -> Self {
        Self {
            output_path: path.to_owned(),
            obj_paths: vec![],
            closed: false,
        }
    }

    fn write_obj(&mut self) -> Result<(), MergeError> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;

        let mut args = [
            OsString::from("-static"),
            OsString::from("-o"),
            self.output_path.as_os_str().to_owned(),
        ]
        .to_vec();
        let mut count = 0;
        args.extend(
            self.obj_paths
                .iter()
                .inspect(|_| count += 1)
                .map(|p| p.as_os_str().into()),
        );

        info!(
            "Merging {} objects: libtool {}",
            count,
            args.iter()
                .map(|s| s.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" ")
        );

        let libtool_path = if let Some(libtool_var) = std::env::var_os("LIBTOOL") {
            debug!("Using libtool from LIBTOOL environment variable ({})", libtool_var.to_string_lossy());
            libtool_var
        } else {
            OsString::from_str("libtool").unwrap()
        };

        let libtool_str = libtool_path.to_string_lossy().to_string();

        let output =
            Command::new(libtool_path)
                .args(&args)
                .output()
                .map_err(|e| ExternalToolLaunchError {
                    tool: libtool_str.clone(),
                    inner: e,
                })?;
        if output.status.success() {
            Ok(())
        } else {
            Err(MergeError::ExternalToolError {
                reason: "Failed to merge object files with `libtool`".to_string(),
                tool: libtool_str,
                args,
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            })
        }
    }
}
