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

use nix::sys::statfs::*;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::error;
use std::fs::File;

pub static MOUNT_POINT: &str = "/sys/fs/cgroup";
pub(crate) type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

#[derive(Clone, Debug, Copy, Eq, Deserialize, Serialize, PartialEq, Hash)]
pub enum CGroupType {
    V1,
    V2,
}

fn get_cgroup_type_config() -> Option<CGroupType> {
    None
}

#[cfg(target_os = "linux")]
fn _get_cgroup_type() -> (CGroupType, bool) {
    let file = File::open(MOUNT_POINT).expect("[lib] stat cgroup mount point failed");
    let fs = fstatfs(&file).expect("[lib] fstatfs cg file error");
    if fs.filesystem_type() == CGROUP2_SUPER_MAGIC {
        return (CGroupType::V2, true);
    }
    (CGroupType::V1, true)
}

fn get_cgroup_type(_args: Option<CGroupType>) -> (CGroupType, bool) {
    #[cfg(target_os = "linux")]
    return _get_cgroup_type();
    #[cfg(target_os = "macos")]
    return (CGroupType::V1, true);
}

#[derive(Clone, Debug, Copy)]
pub struct ConfigDetails<T> {
    config_value: Option<T>,
    actual_value: T,
    enable_status: bool,
}

impl<T> ConfigDetails<T>
where
    T: Clone,
{
    fn new(
        config_func: impl Fn() -> Option<T>,
        init_func: impl Fn(Option<T>) -> (T, bool),
    ) -> ConfigDetails<T> {
        let config_value = config_func();
        let (actual_value, enable_status) = init_func(config_value.clone());
        ConfigDetails {
            config_value,
            actual_value,
            enable_status,
        }
    }

    pub fn config_status(self) -> Option<T> {
        self.config_value
    }

    pub fn actual_status(self) -> T {
        self.actual_value
    }

    pub fn enable_status(&self) -> bool {
        self.enable_status
    }
}

#[derive(Clone, Debug, Copy)]
pub struct ModuleList {
    pub cgroup_type: ConfigDetails<CGroupType>,
}

impl ModuleList {
    fn new() -> ModuleList {
        ModuleList {
            cgroup_type: ConfigDetails::new(get_cgroup_type_config, get_cgroup_type),
        }
    }
}

pub static MODULE_LIST: Lazy<ModuleList> = Lazy::new(ModuleList::new);
