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

use crate::cgroup::{
    new_blkio_cgroup, new_cpu_cgroup, new_cpuset_cgroup, new_memory_cgroup, new_net_cgroup,
    new_perf_event_cgroup, BlkIOCGroup, CpuCGroup, CpuSetCGroup, MemoryCGroup, NetCGroup,
    PerfEventCGroup,
};
use crate::common;
use crate::common::{CGroupType, MODULE_LIST};
use crate::settings;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

const SUB_SYSTEM_MEM_STR: &str = "memory";
const SUB_SYSTEM_CPU_SET_STR: &str = "cpuset";
const SUB_SYSTEM_CPU_STR: &str = "cpuacct";
const SUB_SYSTEM_BLK_IOSTR: &str = "blkio";
const SUB_SYSTEM_NET_STR: &str = "net_cls";
const SUB_SYSTEM_PERF_EVENT_STR: &str = "perf_event";

#[derive(Eq, PartialEq, Hash, Serialize, Deserialize, Clone, Debug)]
pub enum SubSystemType {
    Memory,
    Cpuset,
    Cpuacct,
    Blkio,
    NetCls,
    PerfEvent,
}

impl SubSystemType {
    pub fn to_settings(&self) -> settings::SubSystemType {
        match self {
            SubSystemType::Memory => settings::SubSystemType::Mem,
            SubSystemType::Cpuset => settings::SubSystemType::Cpu,
            SubSystemType::Cpuacct => settings::SubSystemType::Cpu,
            SubSystemType::Blkio => settings::SubSystemType::Storage,
            SubSystemType::NetCls => settings::SubSystemType::Net,
            SubSystemType::PerfEvent => settings::SubSystemType::Unknow,
        }
    }
}

impl fmt::Display for SubSystemType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SubSystemType::Memory => write!(f, "{}", SUB_SYSTEM_MEM_STR),
            SubSystemType::Cpuset => write!(f, "{}", SUB_SYSTEM_CPU_SET_STR),
            SubSystemType::Cpuacct => write!(f, "{}", SUB_SYSTEM_CPU_STR),
            SubSystemType::Blkio => write!(f, "{}", SUB_SYSTEM_BLK_IOSTR),
            SubSystemType::NetCls => write!(f, "{}", SUB_SYSTEM_NET_STR),
            SubSystemType::PerfEvent => write!(f, "{}", SUB_SYSTEM_PERF_EVENT_STR),
        }
    }
}

pub type CGroupUserPath = PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CGroup {
    mount_point: PathBuf,
    user_path: CGroupUserPath,
    pub(crate) sub_system_groups: HashMap<SubSystemType, SubSystem>,
    cgroup_type: CGroupType,
}

impl CGroup {
    pub fn new(mount_point: &str, user_path: PathBuf) -> CGroup {
        let cgroup_type: CGroupType = MODULE_LIST.cgroup_type.actual_status();
        let sub_system_groups: HashMap<SubSystemType, SubSystem> = vec![
            SubSystem::Memory(new_memory_cgroup(mount_point, &user_path, cgroup_type)),
            SubSystem::Cpu(new_cpu_cgroup(mount_point, &user_path, cgroup_type)),
            SubSystem::CpuSet(new_cpuset_cgroup(mount_point, &user_path, cgroup_type)),
            SubSystem::BlkIO(new_blkio_cgroup(mount_point, &user_path, cgroup_type)),
            SubSystem::Net(new_net_cgroup(mount_point, &user_path, cgroup_type)),
            SubSystem::PerfEvent(new_perf_event_cgroup(mount_point, &user_path, cgroup_type)),
        ]
        .into_iter()
        .map(|sub_system| (sub_system.to_type(), sub_system))
        .collect();

        CGroup {
            mount_point: PathBuf::from(mount_point.to_string()),
            user_path,
            sub_system_groups,
            cgroup_type,
        }
    }

    pub fn user_path(&self) -> &Path {
        self.user_path.as_path()
    }

    pub fn mount_point(&self) -> &Path {
        self.mount_point.as_path()
    }
    pub fn is_valid(&self) -> bool {
        !self.sub_system_groups.is_empty()
    }

    pub fn clear_invalid_subsystem_item(&mut self) {
        self.sub_system_groups
            .retain(|_, subsystem| subsystem.sub_system_path_exists());
    }

    pub fn update(&mut self, ds_settings: &dyn settings::DataSource) -> common::Result<bool> {
        // delete invalid subsystem info
        self.clear_invalid_subsystem_item();
        if self.is_valid() && ds_settings.is_enable() {
            // update valid subsystem item
            for (_sub_system_type, sub_system) in self.sub_system_groups.iter_mut() {
                if let Some(sub_sys) = ds_settings.get_sub_system(_sub_system_type.to_settings()) {
                    if !sub_sys.is_enable() {
                        info!(
                            "cgroup sub system {:?} is disabled",
                            _sub_system_type.to_settings()
                        );
                        continue;
                    }
                }
                sub_system.update();
            }
        } else {
            warn!("cgroup is invalid");
        }
        Ok(true)
    }

    pub fn update_ebpf(&mut self, ds_settings: &dyn settings::DataSource) -> common::Result<bool> {
        // delete invalid subsystem info
        self.clear_invalid_subsystem_item();
        if self.is_valid() && ds_settings.is_enable() {
            // update valid subsystem item
            for (_sub_system_type, sub_system) in self.sub_system_groups.iter_mut() {
                if let Some(sub_sys) = ds_settings.get_sub_system(_sub_system_type.to_settings()) {
                    if !sub_sys.is_enable() {
                        info!(
                            "cgroup sub system {:?} is disabled",
                            _sub_system_type.to_settings()
                        );
                        continue;
                    }
                }
                sub_system.update_ebpf();
            }
        }
        Ok(true)
    }

    pub fn reset(&mut self, ds_settings: &dyn settings::DataSource) -> common::Result<bool> {
        // delete invalid subsystem info
        self.clear_invalid_subsystem_item();
        if self.is_valid() {
            // update valid subsystem item
            for (_sub_system_type, sub_system) in self.sub_system_groups.iter_mut() {
                if ds_settings.is_enable() {
                    if let Some(sub_settings) =
                        ds_settings.get_sub_system(_sub_system_type.to_settings())
                    {
                        if sub_settings.is_enable() {
                            info!(
                                "cgroup sub system {} is enable {}",
                                _sub_system_type.to_string(),
                                sub_system.to_type().to_string()
                            );
                            continue;
                        }
                    }
                }

                sub_system.reset();
            }
        }
        Ok(true)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubSystem {
    Memory(MemoryCGroup),
    CpuSet(CpuSetCGroup),
    Cpu(CpuCGroup),
    BlkIO(BlkIOCGroup),
    Net(NetCGroup),
    PerfEvent(PerfEventCGroup),
}

impl SubSystem {
    pub fn to_type(&self) -> SubSystemType {
        match *self {
            SubSystem::Memory(_) => SubSystemType::Memory,
            SubSystem::CpuSet(_) => SubSystemType::Cpuset,
            SubSystem::Cpu(_) => SubSystemType::Cpuacct,
            SubSystem::BlkIO(_) => SubSystemType::Blkio,
            SubSystem::Net(_) => SubSystemType::NetCls,
            SubSystem::PerfEvent(_) => SubSystemType::PerfEvent,
        }
    }
    pub const fn is_memory_cgroup(&self) -> bool {
        matches!(self, SubSystem::Memory(_))
    }

    pub const fn is_cpuset_cgroup(&self) -> bool {
        matches!(self, SubSystem::CpuSet(_))
    }

    pub const fn is_cpu_cgroup(&self) -> bool {
        matches!(self, SubSystem::Cpu(_))
    }

    pub const fn is_blkio_cgroup(&self) -> bool {
        matches!(self, SubSystem::BlkIO(_))
    }
    pub const fn is_net_cgroup(&self) -> bool {
        matches!(self, SubSystem::Net(_))
    }
    pub const fn is_perf_event_cgroup(&self) -> bool {
        matches!(self, SubSystem::PerfEvent(_))
    }
    pub fn get_full_path(&self) -> &Path {
        match self {
            SubSystem::Memory(x) => x.full_path(),
            SubSystem::Cpu(x) => x.full_path(),
            SubSystem::CpuSet(x) => x.full_path(),
            SubSystem::BlkIO(x) => x.full_path(),
            SubSystem::Net(x) => x.full_path(),
            SubSystem::PerfEvent(x) => x.full_path(),
        }
    }

    #[cfg(not(tarpaulin_include))]
    pub fn update(&mut self) {
        match self {
            SubSystem::Memory(ref mut x) => x.update(),
            SubSystem::Cpu(ref mut x) => x.update(),
            SubSystem::CpuSet(ref mut x) => x.update(),
            SubSystem::BlkIO(ref mut x) => x.update(),
            SubSystem::Net(ref mut x) => x.update(),
            SubSystem::PerfEvent(ref mut x) => x.update(),
        }
    }

    pub fn update_ebpf(&mut self) {
        match self {
            SubSystem::Memory(ref mut x) => x.update_ebpf(),
            SubSystem::Cpu(ref mut x) => x.update_ebpf(),
            SubSystem::CpuSet(ref mut x) => x.update_ebpf(),
            SubSystem::BlkIO(ref mut x) => x.update_ebpf(),
            SubSystem::Net(ref mut x) => x.update_ebpf(),
            SubSystem::PerfEvent(ref mut x) => x.update_ebpf(),
        }
    }

    pub fn reset(&mut self) {
        info!("cgroup reset sub system {}", self.to_type().to_string());
        match self {
            SubSystem::Memory(ref mut x) => x.reset(),
            SubSystem::Cpu(ref mut x) => x.reset(),
            SubSystem::CpuSet(ref mut x) => x.reset(),
            SubSystem::BlkIO(ref mut x) => x.reset(),
            SubSystem::Net(ref mut x) => x.reset(),
            SubSystem::PerfEvent(ref mut x) => x.reset(),
        }
    }

    pub fn sub_system_path_exists(&self) -> bool {
        self.get_full_path().exists()
    }

    #[cfg(not(tarpaulin_include))]
    pub fn update_perf_event(&mut self, new_data: &PerfEventCGroup) -> common::Result<bool> {
        match self {
            SubSystem::PerfEvent(ref mut x) => x.update_with_value(
                new_data.cpi,
                new_data.instructions,
                new_data.cycles,
                new_data.l3_cache_miss,
                new_data.utilization,
            ),
            _ => Ok(true),
        }
    }
}
