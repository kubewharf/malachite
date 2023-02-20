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

#![allow(dead_code)]
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use std::mem::MaybeUninit;
use std::time::SystemTime;

// fd mutex, prevent too many files open at once
#[cfg(not(tarpaulin_include))]
#[allow(clippy::mutex_atomic)]
pub(crate) static mut REMAINING_FILES: once_cell::sync::Lazy<Arc<Mutex<u64>>> =
    once_cell::sync::Lazy::new(|| unsafe {
        let mut limits = libc::rlimit {
            rlim_cur: 0, // soft limit
            rlim_max: 0, // hard limit
        };

        // more info: https://man7.org/linux/man-pages/man2/getrlimit.2.html
        if libc::getrlimit(libc::RLIMIT_NOFILE, &mut limits) == 0 {
            // get fd limit failed, expect: 1. soft limit == hard limit ;  2. soft limit 100000
            // TODO: if soft limit != hard limit , set limit
            // TODO: logging
            return Arc::new(Mutex::new(limits.rlim_cur / 2));
        }

        // get fd limit failed, return 1024/2 as default
        Arc::new(Mutex::new(512))
    });

pub(crate) fn check_open_files_limit(f: File) -> Option<File> {
    if let Ok(ref mut x) = unsafe { REMAINING_FILES.lock() } {
        if **x > 0 {
            **x -= 1;
            return Some(f);
        }
    }
    // TODO: exception happended
    None
}

pub(crate) fn get_all_data_from_file(file: &mut File, size: usize) -> io::Result<String> {
    let mut buf = String::with_capacity(size);
    file.seek(SeekFrom::Start(0))?;
    file.read_to_string(&mut buf)?;
    Ok(buf)
}

pub(crate) fn get_all_data<P: AsRef<Path>>(file_path: P, size: usize) -> io::Result<String> {
    let mut file = File::open(file_path.as_ref())?;
    get_all_data_from_file(&mut file, size)
}

pub(crate) fn copy_from_file(entry: &Path) -> Vec<String> {
    match File::open(entry) {
        Ok(mut f) => {
            let mut data = vec![0; 16_384];

            if let Ok(size) = f.read(&mut data) {
                data.truncate(size);
                let mut out = Vec::with_capacity(20);
                let mut start = 0;
                for (pos, x) in data.iter().enumerate() {
                    if *x == 0 {
                        if pos - start >= 1 {
                            if let Ok(s) =
                                std::str::from_utf8(&data[start..pos]).map(|x| x.trim().to_owned())
                            {
                                out.push(s);
                            }
                        }
                        start = pos + 1; // to keeping prevent '\0'
                    }
                }
                out
            } else {
                Vec::new()
            }
        }
        Err(_) => Vec::new(),
    }
}

pub fn realpath(original: &Path) -> std::path::PathBuf {
    use libc::{c_char, lstat, stat, S_IFLNK, S_IFMT};

    fn and(x: u32, y: u32) -> u32 {
        x & y
    }

    let result = PathBuf::from(original);
    let mut result_s = result.to_str().unwrap_or("").as_bytes().to_vec();
    result_s.push(0);
    let mut buf = MaybeUninit::<stat>::uninit();
    let res = unsafe { lstat(result_s.as_ptr() as *const c_char, buf.as_mut_ptr()) };
    let buf = unsafe { buf.assume_init() };
    let link: u32 = S_IFLNK;
    if res < 0 || and(buf.st_mode, S_IFMT).ne(&link) {
        PathBuf::new()
    } else {
        match fs::read_link(&result) {
            Ok(f) => f,
            Err(_) => PathBuf::new(),
        }
    }
}

pub(crate) fn get_secs_since_epoch() -> u64 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs(),
        _ => panic!("SystemTime before UNIX EPOCH!"),
    }
}

pub(crate) fn get_naosecs_since_epoch() -> u128 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_nanos(),
        _ => panic!("SystemTime before UNIX EPOCH!"),
    }
}

pub(crate) fn get_microsecs_since_epoch() -> u128 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_micros(),
        _ => panic!("SystemTime before UNIX EPOCH!"),
    }
}

pub(crate) fn get_uptime() -> u64 {
    let content = get_all_data("/proc/uptime", 50).unwrap_or_default();
    content
        .split('.')
        .next()
        .and_then(|t| t.parse().ok())
        .unwrap_or_default()
}
