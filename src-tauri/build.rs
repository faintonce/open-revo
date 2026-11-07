use std::env;
use std::path::Path;

#[path = "crates/openrevo-core/src/hal/vault_codegen.rs"]
mod vault_codegen;

fn generate_hardware_vault() {
    vault_codegen::generate_hardware_vault(Path::new("config/ec_registry.json"));
}

fn git_short_hash() -> String {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output();

    match output {
        Ok(result) if result.status.success() => {
            String::from_utf8_lossy(&result.stdout).trim().to_string()
        }
        _ => env::var("VITE_GIT_COMMIT").unwrap_or_default(),
    }
}

fn main() {
    // Library tests also link TaskDialogIndirect and need Common Controls v6.
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'");
        println!("cargo:rustc-link-arg-bins=/MANIFEST:NO");
    }
    generate_hardware_vault();

    let git_hash = git_short_hash();
    if !git_hash.is_empty() {
        println!("cargo:rustc-env=OPENREVO_GIT_COMMIT={git_hash}");
        println!(
            "cargo:rustc-env=OPENREVO_APP_VERSION={}{}",
            env!("CARGO_PKG_VERSION"),
            format!("-{}", git_hash)
        );
    } else {
        println!(
            "cargo:rustc-env=OPENREVO_APP_VERSION={}",
            env!("CARGO_PKG_VERSION")
        );
    }

    let mut windows = tauri_build::WindowsAttributes::new();
    windows = windows.app_manifest(
        r#"
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="requireAdministrator" uiAccess="false" />
      </requestedPrivileges>
    </security>
  </trustInfo>
  <dependency>
    <dependentAssembly>
      <assemblyIdentity
        type="win32"
        name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0"
        processorArchitecture="*"
        publicKeyToken="6595b64144ccf1df"
        language="*"
      />
    </dependentAssembly>
  </dependency>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <!-- Windows 10 and Windows 11 -->
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}" />
    </application>
  </compatibility>
</assembly>
"#,
    );
    println!("cargo:rerun-if-changed=icons/icon.ico");
    println!("cargo:rerun-if-changed=tauri.conf.json");
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
        .expect("failed to run tauri-build");
}
