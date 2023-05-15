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

use crate::common;
use crate::common::CGroupType;
use crate::ffi::{
    is_bpf_moudule_valid, wrapper_get_cgroup_net_data, WrapperNetData, BPF_MODULE_CGROUP_NET,
};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct NetCGroup {
    full_path: PathBuf,
    user_path: PathBuf,
    pub(crate) bpf_net_data: WrapperNetData,
    pub(crate) old_bpf_net_data: WrapperNetData,
    update_time: u64,
}

pub fn new_net_cgroup(mount_point: &str, user_path: &Path, cgroup_type: CGroupType) -> NetCGroup {
    let full_path: PathBuf = match cgroup_type {
        CGroupType::V1 => PathBuf::from(format!(
            "{}/net_cls/{}",
            mount_point,
            user_path.to_string_lossy()
        )),
        CGroupType::V2 => PathBuf::from(format!("{}/{}", mount_point, user_path.to_string_lossy())),
    };

    NetCGroup {
        full_path,
        user_path: user_path.to_path_buf(),
        bpf_net_data: Default::default(),
        old_bpf_net_data: Default::default(),
        ..Default::default()
    }
}

impl NetCGroup {
    pub fn full_path(&self) -> &Path {
        self.full_path.as_path()
    }

    pub fn user_path(&self) -> &Path {
        self.user_path.as_path()
    }

    #[cfg(not(tarpaulin_include))]
    pub fn update_bpf_data(&mut self) -> common::Result<bool> {
        let user_path = PathBuf::from(&self.user_path);
        self.old_bpf_net_data = self.bpf_net_data;
        self.bpf_net_data = wrapper_get_cgroup_net_data(user_path);
        Ok(true)
    }

    #[cfg(not(tarpaulin_include))]
    pub fn update(&mut self) {}

    pub fn update_ebpf(&mut self) {
        if !is_bpf_moudule_valid(BPF_MODULE_CGROUP_NET) {
            info!("bpf net module is invalid");
            return;
        }

        if let Err(e) = self.update_bpf_data() {
            warn!(
                "[net_cg] update bpf data error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }
    }

    pub fn reset(&mut self) {
        *self = Self {
            full_path: self.full_path.clone(),
            user_path: self.user_path.clone(),
            ..Default::default()
        }
    }
}
