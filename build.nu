#!/usr/bin/env nu

# A build script to compile the plugin for multiple, specific Nushell versions.

def success [] {
    ansi gradient --fgstart '0x00FF00' --fgend '0xAAAAAA' | print
}

def info [] {
    ansi gradient --fgstart '0xBBBB00' --fgend '0xAAAAAA' | print
}

def main [] {
    # --- Configuration ---
    let nu_versions = [
        "0.108.0",
        "0.109.1" # A placeholder for the latest version
    ]

    let cargo_toml = open Cargo.toml
    let pkg_name = $cargo_toml | get package.name
    let pkg_version = $cargo_toml | get package.version
    let plugin_binary_name = $pkg_name

    print "--- Backing up Cargo.toml ---"
    let original_cargo_toml = $cargo_toml

    mkdir dist

    "--- Starting multi-version build ---" | info
    for version in $nu_versions {
        print $"\n--- Building for Nushell v($version) ---"

        print "  - Modifying Cargo.toml..."
        $original_cargo_toml
            | update dependencies.nu-plugin $"($version)"
            | update dependencies.nu-protocol $"($version)"
            | save --force Cargo.toml

        "  - Running `cargo build --release`..." | info
        let build_result = ( cargo build --release ) | complete
        if $build_result.exit_code != 0 {
            print -e $"\n❌ BUILD FAILED for v($version):"
            print $build_result.stderr
            $original_cargo_toml | save --force Cargo.toml
            exit 1
        }

        let source_path = $"target/release/($plugin_binary_name)"
        let dest_name = $"($plugin_binary_name)-v($pkg_version)-nu($version | str replace '.' '_')"
        let dest_path = $"dist/($dest_name)"
        cp $source_path $dest_path

        $"  - ✅ Success! Binary created at `($dest_path)`" | success
    }

    print "\n--- Build process complete. ---"
    print "--- Restoring original Cargo.toml... ---"
    $original_cargo_toml | save --force Cargo.toml
}
