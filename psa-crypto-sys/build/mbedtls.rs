#![allow(unused)]

/* Copyright (c) Fortanix, Inc.
 *
 * Licensed under the GNU General Public License, version 2 <LICENSE-GPL or
 * https://www.gnu.org/licenses/gpl-2.0.html> or the Apache License, Version
 * 2.0 <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>, at your
 * option. This file may not be copied, modified, or distributed except
 * according to those terms. */

use crate::config;
use crate::features::FEATURES;
use crate::headers;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct BuildConfig {
    pub out_dir: PathBuf,
    pub mbedtls_src: PathBuf,
    pub mbedtls_include: PathBuf,
    pub config_h: PathBuf,
    pub cflags: Vec<String>,
}

impl BuildConfig {
    pub fn create_config_h(&self) {
        let mut defines = config::default_defines();
        for &(feat, def) in config::FEATURE_DEFINES {
            if FEATURES.have_feature(feat) {
                defines.insert(def.0, def.1);
            }
        }
        for &(feat, comp, def) in config::PLATFORM_DEFINES {
            if FEATURES.have_platform_component(feat, comp) {
                defines.insert(def.0, def.1);
            }
        }

        File::create(&self.config_h)
            .and_then(|mut f| {
                f.write_all(config::PREFIX.as_bytes())?;
                for (name, def) in defines {
                    f.write_all(def.define(name).as_bytes())?;
                }
                if FEATURES.have_feature("custom_printf") {
                    writeln!(f, "int mbedtls_printf(const char *format, ...);")?;
                }
                if FEATURES.have_platform_component("threading", "custom") {
                    writeln!(f, "typedef void* mbedtls_threading_mutex_t;")?;
                }
                if FEATURES.have_platform_component("time", "custom") {
                    writeln!(f, "long long mbedtls_time(long long*);")?;
                }
                f.write_all(config::SUFFIX.as_bytes())
            })
            .expect("config.h I/O error");
    }

    pub fn print_rerun_files(&self) {
        println!("cargo:rerun-if-env-changed=RUST_MBEDTLS_SYS_SOURCE");
        println!(
            "cargo:rerun-if-changed={}",
            self.mbedtls_src.join("CMakeLists.txt").display()
        );
        let include = self.mbedtls_src.join(Path::new("include").join("mbedtls"));
        for h in headers::enabled_ordered() {
            println!("cargo:rerun-if-changed={}", include.join(h).display());
        }
        for f in self
            .mbedtls_src
            .join("library")
            .read_dir()
            .expect("read_dir failed")
        {
            println!(
                "cargo:rerun-if-changed={}",
                f.expect("DirEntry failed").path().display()
            );
        }
    }

    pub fn new() -> Self {
        let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR environment not set?"));
        let config_h = out_dir.join("config.h");
        let mbedtls_src = PathBuf::from(
            env::var("RUST_MBEDTLS_SYS_SOURCE").unwrap_or_else(|_| "vendor".to_owned()),
        );
        let mbedtls_include = mbedtls_src.join("include");

        let mut cflags = vec![];
        if FEATURES.have_platform_component("c_compiler", "freestanding") {
            cflags.push("-fno-builtin".into());
            cflags.push("-U_FORTIFY_SOURCE".into());
            cflags.push("-D_FORTIFY_SOURCE=0".into());
            cflags.push("-fno-stack-protector".into());
        }

        BuildConfig {
            config_h,
            out_dir,
            mbedtls_src,
            mbedtls_include,
            cflags,
        }
    }
}
