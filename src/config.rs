use std::{env, fs, path::{Path, PathBuf}};

use serde::Deserialize;

pub const CONFIG_PATH: &str = "config.toml";
const DEFAULT_EXE_PATH: &str = "stormworks64.exe";
const DEFAULT_DLL_PATH: &str = concat!(env!("CARGO_CRATE_NAME"), ".dll");

pub struct Config {
    pub enable_roll: bool,
    pub exe_path: PathBuf,
    pub dll_path: PathBuf
}

impl Config {
    pub fn from_args(&mut self) {
        let mut args = env::args_os().skip(1);
        while let Some(os_arg) = args.next() {
            let arg = os_arg.to_str();
            match arg {
                Some("--enable-roll") => self.enable_roll = true,
                Some("--exe") => {
                    let path = args.next().unwrap();
                    self.exe_path = path.into();
                },
                Some("--dll") => {
                    let path = args.next().unwrap();
                    self.dll_path = path.into();
                }
                _ => panic!()
            }
        }
    }

    pub fn from_file(&mut self, path: impl AsRef<Path>) {
        let Some(config) = ConfigFile::from_file(path) else { return };

        if let Some(enable_roll) = config.enable_roll {
            self.enable_roll = enable_roll;
        }

        if let Some(exe_path) = config.exe_path {
            self.exe_path = exe_path;
        }

        if let Some(dll_path) = config.dll_path {
            self.dll_path = dll_path;
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enable_roll: false,
            exe_path: DEFAULT_EXE_PATH.into(),
            dll_path: DEFAULT_DLL_PATH.into()
        }
    }
}

#[derive(Deserialize)]
pub struct ConfigFile {
    pub enable_roll: Option<bool>,
    pub exe_path: Option<PathBuf>,
    pub dll_path: Option<PathBuf>
}

impl ConfigFile {
    pub fn from_file(path: impl AsRef<Path>) -> Option<Self> {
        let text = fs::read_to_string(path).ok()?;
        toml::from_str(&text).ok()
    }
}