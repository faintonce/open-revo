use std::path::{Path, PathBuf};

/// Resolves the registry the app embeds.
///
/// `src-tauri/build.rs` always reads `src-tauri/config/ec_registry.json`; here that file is
/// reached through the `../../config` fallback. A crate-local copy would silently embed a
/// *different* vault in the same binary, so it is called out as a build warning.
fn find_ec_registry() -> PathBuf {
    let local = Path::new("config/ec_registry.json");
    if local.is_file() {
        println!(
            "cargo:warning=openrevo-core is using the crate-local registry {}; src-tauri/build.rs reads src-tauri/config/ec_registry.json, so both copies must stay identical",
            local.display()
        );
        return local.to_path_buf();
    }
    PathBuf::from("../../config/ec_registry.json")
}

#[path = "src/hal/vault_codegen.rs"]
mod vault_codegen;

fn generate_hardware_vault() {
    // `generate_hardware_vault` decides between "absent" (mock vault) and "present but
    // broken" (stop the build), and registers the containing directory for rebuilds.
    vault_codegen::generate_hardware_vault(&find_ec_registry());
}

fn main() {
    generate_hardware_vault();
}
