use build_rs::output::rerun_if_changed;

fn main() {
    const SHARED: bool = !cfg!(feature = "static");

    rerun_if_changed("minhook");
    
    #[cfg(feature = "build-from-source")]
    {
        use build_rs::input::cargo_cfg_target_arch;
        use build_rs::output::{rustc_link_lib, rustc_link_search_kind, error};
        
        let mut config = cmake::Config::new("minhook");
        let dir = config
        .define("BUILD_SHARED_LIBS", if SHARED { "ON" } else { "OFF" })
        .build();
    
        let search_path = dir.join("lib");
        let target_arch = cargo_cfg_target_arch();
        let cmake_profile = config.get_profile();
        let link_arg = match get_link_arg(&search_path, &target_arch, cmake_profile, SHARED) {
            Ok(a) => a,
            Err(e) => {
                error(e);
                return;
            }
        };
    
        rustc_link_search_kind("native", &search_path);        
        rustc_link_lib(&link_arg);
    }

    #[cfg(not(feature = "build-from-source"))]
    {
        use build_rs::output::rustc_link_lib_kind;
        
        rustc_link_lib_kind(if SHARED { "dylib" } else { "static" }, "minhook");
    }
}

#[cfg(feature = "build-from-source")]
use std::{fs, path::Path};

#[cfg(feature = "build-from-source")]
fn get_link_arg(search_path: impl AsRef<Path>, target_arch: &str, cmake_profile: &str, shared: bool) -> Result<String, &'static str> {
    let search_path = search_path.as_ref();

    let arch_postfix = match target_arch {
        "x86" => "x32",
        "x86_64" => "x64",
        _ => return Err("unsupported architecture")
    };
    let debug_suffix = if cmake_profile == "Debug" { "d" } else { "" };

    let search_name = format!("minhook.{arch_postfix}{debug_suffix}");

    if shared {
        return Ok(format!("dylib={search_name}"));
    }

    for entry in fs::read_dir(search_path).or(Err("error reading directory"))? {
        let Ok(entry) = entry else { return Err("error reading directory entry") };

        let file_name = entry.file_name();
        let file_name = file_name.to_str().ok_or("invalid file name encoding")?;
        if file_name.starts_with(&search_name) {
            return Ok(format!("static:+verbatim={}", file_name));
        }
    }

    Err("library not found")
}