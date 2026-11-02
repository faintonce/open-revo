use std::path::{Path, PathBuf};

fn find_ec_registry() -> Option<PathBuf> {
    let candidate1 = Path::new("config/ec_registry.json");
    if candidate1.exists() {
        return Some(candidate1.to_path_buf());
    }
    let candidate2 = Path::new("../../config/ec_registry.json");
    if candidate2.exists() {
        return Some(candidate2.to_path_buf());
    }
    None
}

#[path = "src/hal/vault_codegen.rs"]
mod vault_codegen;

fn generate_hardware_vault() {
    println!("cargo:rerun-if-changed=config/ec_registry.json");
    println!("cargo:rerun-if-changed=../../config/ec_registry.json");
    let registry =
        find_ec_registry().unwrap_or_else(|| PathBuf::from("../../config/ec_registry.json"));
    vault_codegen::generate_hardware_vault(&registry);
}

fn main() {
    generate_hardware_vault();
}
