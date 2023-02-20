/*
Copyright 2023 The Malachite Authors.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

use std::env;

#[allow(dead_code)]
fn macos_build_env() {
    println!("cargo:rustc-link-search=native=/usr/local/lib");
    println!("cargo:rustc-link-lib=static=elf");
    println!("cargo:rustc-link-search=native=/usr/local/opt/zlib/lib");
    println!("cargo:rustc-link-lib=static=z");
}

#[allow(dead_code)]
fn linux_build_env() {
    #[cfg(target_arch = "x86_64")]
    println!("cargo:rustc-link-search=native=/usr/lib/x86_64-linux-gnu");
    #[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
    println!("cargo:rustc-link-search=native=/usr/lib/aarch64-linux-gnu");
    println!("cargo:rustc-link-lib=static=glib-2.0");
    println!("cargo:rustc-link-lib=static=elf");
    println!("cargo:rustc-link-lib=static=z");
    println!("cargo:rustc-link-lib=static=pcre");
    println!("cargo:rustc-link-lib=dylib=m");
    println!("cargo:rustc-link-lib=dylib=pthread");
}

fn main() {
    // lib/ffi/bpf 所需的 native lib
    let pwd = env::current_dir().unwrap();

    // ffi/ebpf
    #[cfg(target_arch = "x86_64")]
    println!(
        "cargo:rustc-link-search=native={}/src/ffi/bpf/c_bpf/x86/",
        pwd.to_str().unwrap()
    );
    #[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
    println!(
        "{}",
        format!(
            "cargo:rustc-link-search=native={}/src/ffi/bpf/c_bpf/arm64",
            pwd.to_str().unwrap()
        )
    );
    println!("cargo:rustc-link-lib=static=cgroup");
    println!("cargo:rustc-link-lib=static=bpf");

    // ffi/byteperf
    #[cfg(target_arch = "x86_64")]
    println!(
        "cargo:rustc-link-search=native={}/src/ffi/pmu/byteperf/lib_x86_64/",
        pwd.to_str().unwrap()
    );
    #[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
    println!(
        "{}",
        format!(
            "cargo:rustc-link-search=native={}/src/ffi/pmu/byteperf/lib_aarch64/",
            pwd.to_str().unwrap()
        )
    );
    println!("cargo:rustc-link-lib=static=byteperf");
    println!("cargo:rustc-link-lib=static=pfm");

    // bpf 相关库需要 lib elf/z
    #[cfg(target_os = "linux")]
    linux_build_env();
    #[cfg(target_os = "macos")]
    macos_build_env();

    println!("cargo:rerun-if-changed=src/ffi/bpf/c_bpf/malachite.c");
    println!("cargo:rerun-if-changed=src/ffi/pmu/byteperf/include/malachite.c");
}
