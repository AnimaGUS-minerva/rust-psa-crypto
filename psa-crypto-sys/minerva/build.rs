// Copyright 2020 Contributors to the Parsec project.
// SPDX-License-Identifier: Apache-2.0

#![deny(
    nonstandard_style,
// @@    dead_code,
    improper_ctypes,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    private_in_public,
    unconditional_recursion,
// @@    unused,
    unused_allocation,
    unused_comparisons,
    unused_parens,
    while_true,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    //unused_results,
    missing_copy_implementations
)]
// This one is hard to avoid.
#![allow(clippy::multiple_crate_versions)]

mod config;
mod features;
mod headers;
mod mbedtls;
#[path = "bindgen.rs"]
mod mod_bindgen;

#[path = "../build.rs"]
mod mod_build;

#[macro_use]
extern crate lazy_static;

use mbedtls::BuildConfig;

fn main() -> std::io::Result<()> {
    #[cfg(feature = "operations")]
    return operations::script_operations();

    #[cfg(all(feature = "interface", not(feature = "operations")))]
    return interface::script_interface();

    #[cfg(not(any(feature = "interface", feature = "operations")))]
    Ok(())
}

#[cfg(all(feature = "interface", not(feature = "operations")))]
mod interface {
    pub fn script_interface() -> Result<()> {
        crate::mod_build::interface::script_interface()
    }
}

#[cfg(feature = "operations")]
mod operations {
    use crate::mod_build::common;
    use cmake::Config;
    use std::env;
    use std::io::{Error, ErrorKind, Result};
    use std::path::PathBuf;
    use walkdir::WalkDir;

    fn link_to_lib(lib_path: String, link_statically: bool) {
        let link_type = if link_statically { "static" } else { "dylib" };

        // Request rustc to link the Mbed Crypto library
        println!("cargo:rustc-link-search=native={}", lib_path,);
        println!("cargo:rustc-link-lib={}=mbedtls", link_type);
        println!("cargo:rustc-link-lib={}=mbedx509", link_type);
        println!("cargo:rustc-link-lib={}=mbedcrypto", link_type);
    }

    // Build script when the operations feature is on
    pub fn script_operations() -> Result<()> {
        let lib;
        let include;
        let statically;

        if env::var("MBEDTLS_LIB_DIR").is_err() ^ env::var("MBEDTLS_INCLUDE_DIR").is_err() {
            return Err(Error::new(
                ErrorKind::Other,
                "both environment variables MBEDTLS_LIB_DIR and MBEDTLS_INCLUDE_DIR need to be set for operations feature",
            ));
        }

        crate::mod_build::operations::configure_mbed_crypto()?;

        if let (Ok(lib_dir), Ok(include_dir)) =
            (env::var("MBEDTLS_LIB_DIR"), env::var("MBEDTLS_INCLUDE_DIR"))
        {
            lib = lib_dir;
            include = include_dir;
            statically = cfg!(feature = "static") || env::var("MBEDCRYPTO_STATIC").is_ok();
        } else {
            println!("Did not find environment variables, building MbedTLS!");

            let mut mbed_lib_dir = crate::mod_build::operations::compile_mbed_crypto()?; // @@

            let mut mbed_include_dir = mbed_lib_dir.clone();
            mbed_lib_dir.push("lib");
            if !mbed_lib_dir.as_path().exists() {
                _ = mbed_lib_dir.pop();
                mbed_lib_dir.push("lib64");
            }
            mbed_include_dir.push("include");

            lib = mbed_lib_dir.to_str().unwrap().to_owned();
            include = mbed_include_dir.to_str().unwrap().to_owned();
            statically = true;
            //external_mbedtls = false;

            let cfg = super::BuildConfig::new();
            cfg.create_config_h();
            cfg.print_rerun_files();
            cfg.bindgen();
        }

        // Linking to PSA Crypto library is only needed for the operations.
        link_to_lib(lib, statically);

        common::generate_mbed_crypto_bindings(include.clone(), false/*external_mbedtls*/)?;
        common::compile_shim_library(include, false/*metadata*/, false/*external_mbedtls*/).and(Ok(()))
    }
}