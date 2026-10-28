use std::{env, error::Error, ffi::OsString, fmt::Display, fs, io, path::{Path, PathBuf}};

use serde::Deserialize;

pub const CONFIG_PATH: &str = "config.toml";
const DEFAULT_EXE_PATH: &str = "stormworks64.exe";
const DEFAULT_DLL_PATH: &str = "sw_head_tracking.dll";

pub fn print_help() {
    println!(
"sw-head-tracking: head tracking for Stormworks

Launches Stormworks and loads the head tracking mod.

USAGE:
    sw-loader [OPTIONS]

OPTIONS:
        --exe <PATH>    Game executable to launch [default: {DEFAULT_EXE_PATH}]
        --dll <PATH>    Mod dll to load [default: {DEFAULT_DLL_PATH}]
        --enable-roll   Also apply head roll to the camera [default: off]
        --config <PATH> Config file to read [default: {CONFIG_PATH}]
    -h, --help          Print this help and exit

Options can also be set in {CONFIG_PATH} using the keys
enable_roll, exe_path and dll_path."
    );
}

pub struct Config {
    pub enable_roll: bool,
    pub exe_path: PathBuf,
    pub dll_path: PathBuf
}

impl Config {
    pub fn load() -> Result<Command, ConfigError> {
        let args = Args::parse()?;
        
        if args.print_help { return Ok(Command::Help) };

        let Args { config_path, overrides, .. } = args;
        let mut config = Self::default();

        match config_path {
            Some(path) => {
                let config_file = PartialConfig::from_file(&path)?.ok_or_else(|| ConfigError::MissingConfigFile(path))?;
                config.apply(config_file);
            },
            None => {
                if let Some(config_file) = PartialConfig::from_file(CONFIG_PATH)? {
                    config.apply(config_file);
                }
            }
        }

        config.apply(overrides);

        Ok(Command::Run(config))
    }

    fn apply(&mut self, partial: PartialConfig) {
        if let Some(enable_roll) = partial.enable_roll {
            self.enable_roll = enable_roll;
        }

        if let Some(exe_path) = partial.exe_path {
            self.exe_path = exe_path;
        }

        if let Some(dll_path) = partial.dll_path {
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

pub enum Command {
    Run(Config),
    Help
}

#[derive(Debug)]
pub enum ConfigError {
    MissingValue(&'static str),
    UnknownArgument(OsString),
    MissingConfigFile(PathBuf),
    Io { path: PathBuf, source: io::Error },
    Parse { path: PathBuf, source: toml::de::Error }
}

impl Error for ConfigError { }

impl Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::MissingValue(option) => write!(f, "option `{option}` requires a path"),
            ConfigError::UnknownArgument(arg) => write!(f, "unknown argument `{}`", arg.to_string_lossy()),
            ConfigError::MissingConfigFile(path) => write!(f, "config file `{}` not found", path.display()),
            ConfigError::Io { path, source } => write!(f, "cannot read config file `{}`: {source}", path.display()),
            ConfigError::Parse { path, source } => write!(f, "invalid config file `{}`:\n{source}", path.display())
        }
    }
}

struct Args {
    config_path: Option<PathBuf>,
    print_help: bool,
    overrides: PartialConfig
}

impl Args {
    fn parse() -> Result<Self, ConfigError> {
        let mut out = Self {
            config_path: None,
            print_help: false,
            overrides: PartialConfig::default()
        };

        let mut args = env::args_os().skip(1);
        while let Some(os_arg) = args.next() {
            match os_arg.to_str() {
                Some("--help") | Some("-h") => 
                {
                    out.print_help = true;
                    return Ok(out);
                }
                Some("--config") => {
                    let path = args.next().ok_or(ConfigError::MissingValue("--config"))?;
                    out.config_path = Some(path.into());
                }
                Some("--enable-roll") => out.overrides.enable_roll = Some(true),
                Some("--exe") => {
                    let path = args.next().ok_or(ConfigError::MissingValue("exe"))?;
                    out.overrides.exe_path = Some(path.into());
                },
                Some("--dll") => {
                    let path = args.next().ok_or(ConfigError::MissingValue("--dll"))?;
                    out.overrides.dll_path = Some(path.into());
                }
                _ => return Err(ConfigError::UnknownArgument(os_arg))
            }
        }

        Ok(out)
    }
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct PartialConfig {
    enable_roll: Option<bool>,
    exe_path: Option<PathBuf>,
    dll_path: Option<PathBuf>
}

impl PartialConfig {
    fn from_file(path: impl AsRef<Path>) -> Result<Option<Self>, ConfigError> {
        let path = path.as_ref();
        let text = match fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(ConfigError::Io { path: path.into(), source })
        };
        toml::from_str(&text).map(Some).map_err(|source| ConfigError::Parse { path: path.into(), source })
    }
}