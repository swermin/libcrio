use log::debug;
use serde::Serialize;
use serde_json::Value;
use std::io::prelude::*;
use std::process::Command;
use std::process::Stdio;
use std::str::FromStr;

/// A CLI wrapper object
#[derive(Debug, Serialize, PartialEq, Clone)]
pub struct Cli {
    /// The bin_path to find the crio_cli required as the host process may not have this preconfigured.
    /// Usually set to "/bin:/sbin:/usr/bin:/usr/sbin:/usr/local/bin:/home/kubernetes/bin"
    /// If you are deploying crictl on the host you may want to append that location as well.
    pub bin_path: String,
    /// The location of the crictl.yaml
    pub config_path: Option<String>,
    /// The command for listing images. If not supplied it will default to 'img'
    pub image_command: ImageCommand,
}

/// A switch to indicate which image command to run
#[derive(Debug, Serialize, PartialEq, Clone)]
pub enum ImageCommand {
    Img,
    Images,
}

impl fmt::Display for ImageCommand {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(LowercaseFormatter(formatter), "{:?}", self)
    }
}

impl FromStr for ImageCommand {
    type Err = ();

    fn from_str(input: &str) -> Result<ImageCommand, Self::Err> {
        match input.to_lowercase().as_str() {
            "img" => Ok(ImageCommand::Img),
            "images" => Ok(ImageCommand::Images),
            _ => Err(()),
        }
    }
}

use std::fmt::{self, Write};

struct LowercaseFormatter<'a, 'b>(pub &'a mut fmt::Formatter<'b>);

impl<'a, 'b> fmt::Write for LowercaseFormatter<'a, 'b> {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        for ch in s.chars() {
            self.0.write_fmt(format_args!("{}", ch.to_lowercase()))?;
        }

        Ok(())
    }
}

/// Returns a defauilt instance of `Cli` with
/// bin_path`: "/bin:/sbin:/usr/bin:/usr/sbin:/usr/local/bin:/home/kubernetes/bin"
/// config_path`: None,
/// image_command` `ImageCommand::Img`
impl Default for Cli {
    fn default() -> Cli {
        Cli {
            bin_path: "/bin:/sbin:/usr/bin:/usr/sbin:/usr/local/bin:/home/kubernetes/bin"
                .to_string(),
            config_path: None,
            image_command: ImageCommand::Img,
        }
    }
}

impl Cli {
    /// Returns a JSON value containing the pod information
    ///
    /// # Arguments
    ///
    /// * `hostname` - The hostname of the pod
    ///
    /// # Examples
    ///
    /// ```
    /// use libcrio::Cli;
    /// let bin_path = format!("{}/mock/iks", env!("CARGO_MANIFEST_DIR"));
    /// let cli = Cli {
    ///     bin_path,
    ///     ..Default::default()
    /// };
    /// let val = cli.pod("tests").unwrap();
    /// ```
    pub fn pod(&self, hostname: &str) -> Result<Value, String> {
        let pod_output_args = match &self.config_path {
            Some(s) => {
                vec!["-c", s.as_str(), "pods", "--name", hostname, "-o", "json"]
            }
            None => {
                vec!["pods", "--name", hostname, "-o", "json"]
            }
        };

        let pod_list = run_command(pod_output_args, &self.bin_path)?;
        let pod = match pod_list["items"].get(0) {
            Some(s) => s,
            None => {
                return Err("failed to create pod at index 0".to_string());
            }
        };
        Ok(pod.clone())
    }

    /// Returns a JSON value containing the pod inpection output
    ///
    /// # Arguments
    ///
    /// * `pod_id` - The id of the pod
    ///
    /// # Examples
    ///
    /// ```
    /// use libcrio::Cli;
    /// let bin_path = format!("{}/mock/iks", env!("CARGO_MANIFEST_DIR"));
    /// let cli = Cli {
    ///     bin_path,
    ///     ..Default::default()
    /// };
    /// let val = cli.inspect_pod("51cd8bdaa13a65518e790d307359d33f9288fc82664879c609029b1a83862db6").unwrap();
    /// ```
    pub fn inspect_pod(&self, pod_id: &str) -> Result<Value, String> {
        let inspect_output_args = match &self.config_path {
            Some(s) => vec!["-c", s.as_str(), "inspectp", pod_id],
            None => vec!["inspectp", pod_id],
        };
        run_command(inspect_output_args, &self.bin_path)
    }

    /// Returns a JSON value containing the containers related to a pod
    ///
    /// # Arguments
    ///
    /// * `pod_id` - The id of the pod
    ///
    /// # Examples
    ///
    /// ```
    /// use libcrio::Cli;
    /// let bin_path = format!("{}/mock/iks", env!("CARGO_MANIFEST_DIR"));
    /// let cli = Cli {
    ///     bin_path,
    ///     ..Default::default()
    /// };
    /// let val = cli.pod_containers("51cd8bdaa13a65518e790d307359d33f9288fc82664879c609029b1a83862db6").unwrap();
    /// ```
    pub fn pod_containers(&self, pod_id: &str) -> Result<Value, String> {
        let ps_output_args = match &self.config_path {
            Some(s) => vec!["-c", s.as_str(), "ps", "-o", "json", "-p", pod_id],
            None => vec!["ps", "-o", "json", "-p", pod_id],
        };
        run_command(ps_output_args, &self.bin_path)
    }

    /// Returns a JSON value containing the container inpection output
    ///
    /// # Arguments
    ///
    /// * `container_id` - The id of the container
    ///
    /// # Examples
    ///
    /// ```
    /// use libcrio::Cli;
    /// let bin_path = format!("{}/mock/iks", env!("CARGO_MANIFEST_DIR"));
    /// let cli = Cli {
    ///     bin_path,
    ///     ..Default::default()
    /// };
    /// let val = cli.inspect_container("765312810c818bca4836c3598e21471bfd96be8ca84ca952290a9900b7c055a7").unwrap();
    /// ```
    pub fn inspect_container(&self, container_id: &str) -> Result<Value, String> {
        let inspect_output_args = match &self.config_path {
            Some(s) => vec!["-c", s.as_str(), "inspect", container_id],
            None => vec!["inspect", container_id],
        };
        run_command(inspect_output_args, &self.bin_path)
    }

    /// Returns a JSON value containing the images related to a container
    ///
    /// # Arguments
    ///
    /// * `image_ref` - The image reference related to one of the containers obtained from `pod_containers`
    ///
    /// # Examples
    ///
    /// ```
    /// use libcrio::Cli;
    /// let bin_path = format!("{}/mock/iks", env!("CARGO_MANIFEST_DIR"));
    /// let cli = Cli {
    ///     bin_path,
    ///     ..Default::default()
    /// };
    /// let val = cli.image("sha256:3b8adc6c30f4e7e4afb57daef9d1c8af783a4a647a4670780e9df085c0525efa").unwrap();
    /// ```
    pub fn image(&self, image_ref: &str) -> Result<Value, String> {
        let img_cmd_string = format!("{}", &self.image_command);
        let img_cmd = img_cmd_string.as_str();

        let image_output_args = match &self.config_path {
            Some(s) => vec!["-c", s.as_str(), img_cmd, "-o", "json"],
            None => vec![img_cmd, "-o", "json"],
        };
        let log_args = image_output_args.clone();
        let image_list = run_command(image_output_args, &self.bin_path)?;
        match image_list["images"].as_array() {
            Some(img_lines) => {
                debug!("Found {} images", img_lines.len());
                for line in img_lines {
                    let line_obj: Value = serde_json::to_value(line).unwrap();
                    let line_obj_id = line_obj["id"].as_str().unwrap_or_default();

                    debug!("Matching {} using {}", line_obj_id, image_ref);
                    if line_obj_id == image_ref {
                        debug!("MATCHED {} using {}", line_obj_id, image_ref);
                        return Ok(line_obj.clone());
                    } else if let Some(arr) = line_obj["repoDigests"].as_array() {
                        debug!("Matching inspecting repoDigests \n{:?}", arr);
                        for digest in arr {
                            let digest_str = digest.as_str().unwrap_or_default();
                            debug!("Matching repoDigests {} to {}", digest_str, image_ref);
                            if digest_str == image_ref {
                                debug!("MATCHED {} to {}", line_obj_id, image_ref);
                                return Ok(line_obj.clone());
                            }
                        }
                    }
                }
                Err(format!("no images matched in crictl img {:?}", log_args))
            }
            None => Err(format!("no images found in crictl img {:?}", log_args)),
        }
    }

    /// Returns a text value containing the logs related to a container
    ///
    /// # Arguments
    ///
    /// * `container_id` - The container_id related to one of the containers obtained from `pod_containers`
    ///
    /// # Examples
    ///
    /// ```
    /// use libcrio::Cli;
    /// let bin_path = format!("{}/mock/iks", env!("CARGO_MANIFEST_DIR"));
    /// let cli = Cli {
    ///     bin_path,
    ///     ..Default::default()
    /// };
    /// #[allow(deprecated)]
    /// let val = cli.logs("sha256:3b8adc6c30f4e7e4afb57daef9d1c8af783a4a647a4670780e9df085c0525efa").unwrap();
    /// ```
    #[deprecated]
    pub fn logs(&self, container_id: &str) -> Result<String, String> {
        let log_output_args = match &self.config_path {
            Some(s) => vec!["-c", s.as_str(), "logs", container_id],
            None => vec!["logs", container_id],
        };
        run_command_text(log_output_args, &self.bin_path)
    }

    /// Returns a text value containing the logs related to a container
    ///
    /// # Arguments
    ///
    /// * `container_id` - The container_id related to one of the containers obtained from `pod_containers`
    ///
    /// * `line_count` - The number of lines to take from the end of the log.
    ///
    /// # Examples
    ///
    /// ```
    /// use libcrio::Cli;
    /// let bin_path = format!("{}/mock/iks", env!("CARGO_MANIFEST_DIR"));
    /// let cli = Cli {
    ///     bin_path,
    ///     ..Default::default()
    /// };
    /// let val = cli.tail_logs("sha256:3b8adc6c30f4e7e4afb57daef9d1c8af783a4a647a4670780e9df085c0525efa", 500).unwrap();
    /// ```
    pub fn tail_logs(&self, container_id: &str, line_count: u32) -> Result<String, String> {
        let tailoption = format!("--tail={}", line_count);
        let log_output_args = match &self.config_path {
            Some(s) => vec!["-c", s.as_str(), "logs", tailoption.as_str(), container_id],
            None => vec!["logs", tailoption.as_str(), container_id],
        };
        run_command_text(log_output_args, &self.bin_path)
    }

    /// # Arguments
    ///
    /// * `path` - The additional path to append to bin_path,
    ///
    /// # Examples
    ///
    /// ```
    /// use libcrio::Cli;
    /// let bin_path = format!("{}/mock/iks", env!("CARGO_MANIFEST_DIR"));
    /// let mut cli = Cli {
    ///     bin_path,
    ///     ..Default::default()
    /// };
    /// cli.append_bin_path("/my/new/location".to_string());
    /// ```
    pub fn append_bin_path(&mut self, path: String) {
        let internal = if !path.starts_with(':') {
            format!(":{}", path)
        } else {
            path
        };
        self.bin_path.push_str(internal.as_str());
    }
}

fn slice_to_value(slice: &[u8], args: Vec<&str>) -> Result<Value, String> {
    match serde_json::from_slice(slice) {
        Ok(v) => Ok(v),
        Err(e) => Err(format!(
            "failed to create output from slice for {:?} {}",
            args, e
        )),
    }
}

fn run_command_text(args: Vec<&str>, bin_path: &str) -> Result<String, String> {
    debug!("running {:?} {:?}", args, bin_path);
    let cmd = match Command::new("crictl")
        .env("PATH", bin_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .args(&args)
        .spawn()
    {
        Ok(v) => v,
        Err(e) => {
            return Err(format!("failed to execute crictl {:?} {}", args, e));
        }
    };
    let waiter = match cmd.wait_with_output() {
        Ok(v) => v,
        Err(e) => {
            return Err(format!("failed to execute crictl {:?} {}", args, e));
        }
    };

    let mut err_str = String::new();
    match waiter.stderr.as_slice().read_to_string(&mut err_str) {
        Err(e) => {
            return Err(format!(
                "stderr read error - failed to execute crictl {:?} {}",
                args, e
            ));
        }
        Ok(_) => {
            if !err_str.is_empty() {
                return Err(format!(
                    "stderr not empty - failed to execute crictl {:?} {}",
                    args, err_str
                ));
            }
        }
    }

    // if !waiter.success() {
    //     return Err(format!(
    //         "crictl status is unsuccessful {:?}, {}",
    //         args, waiter
    //     ));
    // }
    let mut ok_str = String::new();
    match waiter.stdout.as_slice().read_to_string(&mut ok_str) {
        Err(e) => Err(format!(
            "stdout error - failed to execute crictl {:?} {}",
            args, e
        )),
        Ok(_) => Ok(ok_str),
    }
}

fn run_command(args: Vec<&str>, bin_path: &str) -> Result<Value, String> {
    let l_args = args.clone();
    let str_ok = run_command_text(args, bin_path)?;
    slice_to_value(str_ok.as_bytes(), l_args)
}

#[cfg(test)]
mod tests {
    use crate::{Cli, ImageCommand};
    use std::str::FromStr;

    pub fn get_clis() -> Vec<Cli> {
        let mut test_cases: Vec<Cli> = vec![];
        let bin_path = format!("{}/mock/iks", env!("CARGO_MANIFEST_DIR"));
        test_cases.push(Cli {
            bin_path,
            config_path: None,
            image_command: ImageCommand::Img,
        });
        test_cases
    }

    pub fn get_big_data_cli() -> Cli {
        let bin_path = format!("{}/mock/big_data", env!("CARGO_MANIFEST_DIR"));
        Cli {
            bin_path,
            config_path: None,
            image_command: ImageCommand::Img,
        }
    }

    pub fn get_only_errors_cli() -> Cli {
        let bin_path = format!("{}/mock/only_errors", env!("CARGO_MANIFEST_DIR"));
        Cli {
            bin_path,
            config_path: None,
            image_command: ImageCommand::Img,
        }
    }

    pub fn get_long_logs_cli() -> Cli {
        let bin_path = format!("{}/mock/long_logs:/usr/bin", env!("CARGO_MANIFEST_DIR"));
        Cli {
            bin_path,
            config_path: None,
            image_command: ImageCommand::Img,
        }
    }

    pub fn get_mixed_errors_cli() -> Cli {
        let bin_path = format!("{}/mock/mixed_errors", env!("CARGO_MANIFEST_DIR"));
        Cli {
            bin_path,
            config_path: None,
            image_command: ImageCommand::Img,
        }
    }
    pub fn get_bad_json_cli() -> Cli {
        let bin_path = format!("{}/mock/bad_json", env!("CARGO_MANIFEST_DIR"));
        Cli {
            bin_path,
            config_path: None,
            image_command: ImageCommand::Img,
        }
    }
    pub fn get_openshift_cli() -> Cli {
        let bin_path = format!("{}/mock/openshift", env!("CARGO_MANIFEST_DIR"));
        Cli {
            bin_path,
            config_path: None,
            image_command: ImageCommand::Img,
        }
    }

    #[test]
    fn test_append_bin_path() {
        let mut cli = Cli::default();
        let path = "/my/path".to_string();
        cli.append_bin_path(path);
        assert_eq!(
            cli.bin_path,
            "/bin:/sbin:/usr/bin:/usr/sbin:/usr/local/bin:/home/kubernetes/bin:/my/path"
                .to_string(),
        );

        let path2 = ":/my/path2".to_string();
        cli.append_bin_path(path2);
        assert_eq!(
            cli.bin_path,
            "/bin:/sbin:/usr/bin:/usr/sbin:/usr/local/bin:/home/kubernetes/bin:/my/path:/my/path2"
                .to_string(),
        );
    }

    /*************************************************************************
     * pod Tests
     **************************************************************************/
    #[test]
    fn test_pod_returns_a_pod_openshift() {
        let cli = get_openshift_cli();
        let val = cli.pod("tests").unwrap();
        assert_eq!(
            val["id"].as_str().unwrap(),
            "134b58ab2e0cfd7432a9db818b1b4ec52fdc747333f0ba2c9342860dc2ea7c50"
        );
    }

    #[test]
    fn test_pod_returns_a_pod() {
        for cli in get_clis() {
            let val = cli.pod("tests").unwrap();
            assert_eq!(
                val["id"].as_str().unwrap(),
                "51cd8bdaa13a65518e790d307359d33f9288fc82664879c609029b1a83862db6"
            );
        }
    }
    #[test]
    fn test_pod_returns_a_pod_only_errors_cli() {
        let cli = get_only_errors_cli();
        let val = cli.pod("tests");
        let expected = Err(String::from(
            "failed to create output from slice for [\"pods\", \"--name\", \"tests\", \"-o\", \"json\"] EOF while parsing a value at line 2 column 0",
        ));
        assert_eq!(expected, val);
    }

    #[test]
    fn test_pod_returns_a_pod_mixed_errors_cli() {
        let cli = get_mixed_errors_cli();
        let val = cli.pod("tests");
        let expected = Err(String::from("stderr not empty - failed to execute crictl [\"pods\", \"--name\", \"tests\", \"-o\", \"json\"] An error message\n"));
        assert_eq!(expected, val);
    }

    #[test]
    fn test_pod_returns_a_pod_bad_json_cli() {
        let cli = get_bad_json_cli();
        let val = cli.pod("tests");
        let expected = Err(String::from("failed to create output from slice for [\"pods\", \"--name\", \"tests\", \"-o\", \"json\"] EOF while parsing a value at line 2 column 0"));
        assert_eq!(expected, val);
    }

    #[test]
    fn test_get_big_data() {
        let cli = get_big_data_cli();
        let val = cli.tail_logs("", 0).unwrap();
        let mut expected = String::from("");
        for _f in 0..65536 {
            expected.push('a');
        }
        expected.push('\n');
        assert_eq!(expected, val);
    }
    /*************************************************************************
     * inspect tests
     **************************************************************************/
    #[test]
    fn test_inspect_pod() {
        for cli in get_clis() {
            let val = cli
                .inspect_pod("51cd8bdaa13a65518e790d307359d33f9288fc82664879c609029b1a83862db6")
                .unwrap();
            assert_eq!(val["info"]["pid"].as_i64().unwrap(), 14017)
        }
    }
    #[test]
    fn test_inspect_pod_openshift() {
        let cli = get_openshift_cli();
        let val = cli
            .inspect_pod("134b58ab2e0cfd7432a9db818b1b4ec52fdc747333f0ba2c9342860dc2ea7c50")
            .unwrap();
        assert_eq!(val["info"]["pid"].as_i64().unwrap(), 38091)
    }
    #[test]
    fn test_inspect_returns_a_pod_mixed_errors_cli() {
        let cli = get_mixed_errors_cli();
        let val = cli.inspect_pod("tests");
        let expected = Err(String::from(
            "stderr not empty - failed to execute crictl [\"inspectp\", \"tests\"] An error message\n",
        ));
        assert_eq!(expected, val);
    }

    #[test]
    fn test_inspect_pod_only_errors_cli() {
        let cli = get_only_errors_cli();
        let val =
            cli.inspect_pod("51cd8bdaa13a65518e790d307359d33f9288fc82664879c609029b1a83862db6");
        let expected = Err(String::from("failed to create output from slice for [\"inspectp\", \"51cd8bdaa13a65518e790d307359d33f9288fc82664879c609029b1a83862db6\"] EOF while parsing a value at line 2 column 0"));
        assert_eq!(expected, val);
    }

    #[test]
    fn test_inspect_pod_bad_json_cli() {
        let cli = get_bad_json_cli();
        let val =
            cli.inspect_pod("51cd8bdaa13a65518e790d307359d33f9288fc82664879c609029b1a83862db6");
        let expected = Err(String::from("failed to create output from slice for [\"inspectp\", \"51cd8bdaa13a65518e790d307359d33f9288fc82664879c609029b1a83862db6\"] EOF while parsing a value at line 2 column 0"));
        assert_eq!(expected, val);
    }

    #[test]
    fn test_inspect_container() {
        for cli in get_clis() {
            let val = cli
                .inspect_container(
                    "765312810c818bca4836c3598e21471bfd96be8ca84ca952290a9900b7c055a7",
                )
                .unwrap();
            assert_eq!(val["info"]["pid"].as_i64().unwrap(), 254405)
        }
    }
    #[test]
    fn test_inspect_returns_a_container_mixed_errors_cli() {
        let cli = get_mixed_errors_cli();
        let val = cli.inspect_container("tests");
        let expected = Err(String::from(
            "stderr not empty - failed to execute crictl [\"inspect\", \"tests\"] An error message\n",
        ));
        assert_eq!(expected, val);
    }

    #[test]
    fn test_inspect_container_only_errors_cli() {
        let cli = get_only_errors_cli();
        let val = cli
            .inspect_container("765312810c818bca4836c3598e21471bfd96be8ca84ca952290a9900b7c055a7");
        let expected = Err(String::from("failed to create output from slice for [\"inspect\", \"765312810c818bca4836c3598e21471bfd96be8ca84ca952290a9900b7c055a7\"] EOF while parsing a value at line 2 column 0"));
        assert_eq!(expected, val);
    }

    #[test]
    fn test_inspect_container_bad_json_cli() {
        let cli = get_bad_json_cli();
        let val = cli
            .inspect_container("765312810c818bca4836c3598e21471bfd96be8ca84ca952290a9900b7c055a7");
        let expected = Err(String::from("failed to create output from slice for [\"inspect\", \"765312810c818bca4836c3598e21471bfd96be8ca84ca952290a9900b7c055a7\"] EOF while parsing a value at line 2 column 0"));
        assert_eq!(expected, val);
    }

    /*************************************************************************
     * pod containers tests
     **************************************************************************/
    #[test]
    fn test_pod_containers() {
        for cli in get_clis() {
            let val = cli
                .pod_containers("51cd8bdaa13a65518e790d307359d33f9288fc82664879c609029b1a83862db6")
                .unwrap();
            assert_eq!(
                val["containers"][0]["id"].as_str().unwrap(),
                "4bd48d7c6a03cd94a0e95e97011ed5d2ca72045723a5ed55da06fd54eff32b0a"
            )
        }
    }
    #[test]
    fn test_pod_containers_openshift() {
        let cli = get_openshift_cli();
        let val = cli
            .pod_containers("134b58ab2e0cfd7432a9db818b1b4ec52fdc747333f0ba2c9342860dc2ea7c50")
            .unwrap();
        assert_eq!(
            val["containers"][0]["id"].as_str().unwrap(),
            "0e04af54d9273f5bb37eddbe8ace750275d7939612dd4864c792168cce2cff82"
        )
    }
    #[test]
    fn test_pod_containers_only_errors_cli() {
        let cli = get_only_errors_cli();
        let val =
            cli.pod_containers("51cd8bdaa13a65518e790d307359d33f9288fc82664879c609029b1a83862db6");
        let expected = Err(String::from("failed to create output from slice for [\"ps\", \"-o\", \"json\", \"-p\", \"51cd8bdaa13a65518e790d307359d33f9288fc82664879c609029b1a83862db6\"] EOF while parsing a value at line 2 column 0"));
        assert_eq!(expected, val);
    }

    #[test]
    fn test_pod_containers_bad_json_cli() {
        let cli = get_bad_json_cli();
        let val =
            cli.pod_containers("51cd8bdaa13a65518e790d307359d33f9288fc82664879c609029b1a83862db6");
        let expected = Err(String::from("failed to create output from slice for [\"ps\", \"-o\", \"json\", \"-p\", \"51cd8bdaa13a65518e790d307359d33f9288fc82664879c609029b1a83862db6\"] EOF while parsing a value at line 2 column 0"));
        assert_eq!(expected, val);
    }

    #[test]
    fn test_pod_containers_mixed_errors_cli() {
        let cli = get_mixed_errors_cli();
        let val =
            cli.pod_containers("51cd8bdaa13a65518e790d307359d33f9288fc82664879c609029b1a83862db6");
        let expected = Err(String::from(
            "stderr not empty - failed to execute crictl [\"ps\", \"-o\", \"json\", \"-p\", \"51cd8bdaa13a65518e790d307359d33f9288fc82664879c609029b1a83862db6\"] An error message\n",
        ));
        assert_eq!(expected, val);
    }

    /*************************************************************************
     * image tests
     **************************************************************************/
    #[test]
    fn test_image() {
        for cli in get_clis() {
            let val = cli
                .image("sha256:3b8adc6c30f4e7e4afb57daef9d1c8af783a4a647a4670780e9df085c0525efa")
                .unwrap();
            assert_eq!(val["size"].as_str().unwrap(), "338054458")
        }
    }
    #[test]
    fn test_image_openshift() {
        let cli = get_openshift_cli();
        let val = cli
            .image("quay.io/icdh/segfaulter@sha256:0630afbcfebb45059794b9a9f160f57f50062d28351c49bb568a3f7e206855bd")
            .unwrap();
        assert_eq!(val["size"].as_str().unwrap(), "10229047")
    }
    #[test]
    fn test_images_only_errors_cli() {
        let cli = get_only_errors_cli();
        let val =
            cli.image("sha256:3b8adc6c30f4e7e4afb57daef9d1c8af783a4a647a4670780e9df085c0525efa");
        let expected = Err(String::from(
            "failed to create output from slice for [\"img\", \"-o\", \"json\"] EOF while parsing a value at line 2 column 0",
        ));
        assert_eq!(expected, val);
    }

    #[test]
    fn test_json_errors_cli() {
        let cli = get_bad_json_cli();
        let val =
            cli.image("sha256:3b8adc6c30f4e7e4afb57daef9d1c8af783a4a647a4670780e9df085c0525efa");
        let expected = Err(String::from("failed to create output from slice for [\"img\", \"-o\", \"json\"] EOF while parsing a value at line 2 column 0"));
        assert_eq!(expected, val);
    }

    #[test]
    fn test_image_mixed_errors_cli() {
        let cli = get_mixed_errors_cli();
        let val =
            cli.image("sha256:3b8adc6c30f4e7e4afb57daef9d1c8af783a4a647a4670780e9df085c0525efa");
        let expected = Err(String::from(
            "stderr not empty - failed to execute crictl [\"img\", \"-o\", \"json\"] An error message\n",
        ));
        assert_eq!(expected, val);
    }
    /*************************************************************************
     * log tests
     **************************************************************************/
    #[allow(deprecated)]
    #[test]
    fn test_logs() {
        for cli in get_clis() {
            let val = cli
                .logs("51cd8bdaa13a65518e790d307359d33f9288fc82664879c609029b1a83862db6")
                .unwrap();
            assert_eq!(val, "A LOG\n".to_string())
        }
    }
    #[allow(deprecated)]
    #[test]
    fn test_logs_mixed_errors_cli() {
        let cli = get_mixed_errors_cli();
        let val = cli.logs("51cd8bdaa13a65518e790d307359d33f9288fc82664879c609029b1a83862db6");
        let expected = Err(String::from(
             "stderr not empty - failed to execute crictl [\"logs\", \"51cd8bdaa13a65518e790d307359d33f9288fc82664879c609029b1a83862db6\"] An error message\n",
         ));
        assert_eq!(expected, val);
    }
    #[test]
    fn test_tail_logs() {
        let cli = get_long_logs_cli();
        let val = cli
            .tail_logs(
                "51cd8bdaa13a65518e790d307359d33f9288fc82664879c609029b1a83862db6",
                500,
            )
            .unwrap();
        assert_eq!(val.lines().count(), 500);
        assert!(val.ends_with("logging 500\n"));
        assert!(!val.contains("logging 501"));
    }

    #[test]
    fn test_image_cmd_from_str() {
        assert_eq!(
            ImageCommand::Images,
            ImageCommand::from_str("IMAGES").unwrap()
        );
        assert_eq!(ImageCommand::Img, ImageCommand::from_str("imG").unwrap());

        let actual_error_kind = ImageCommand::from_str("ADSF").unwrap_err();
        assert_eq!((), actual_error_kind);

        let cl = ImageCommand::Img;
        assert_eq!(cl.clone(), ImageCommand::Img);
    }
}
