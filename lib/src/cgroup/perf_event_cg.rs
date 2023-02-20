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
use crate::common::{CGroupType, MOUNT_POINT};
use crate::ffi::pmu::ffi::byteperf_cgroup_buffer_malachite;
use crate::system::get_secs_since_epoch;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::ffi::CStr;
use std::path::{Path, PathBuf};

pub fn new_perf_event_cgroup(
    mount_point: &str,
    user_path: &Path,
    cgroup_type: CGroupType,
) -> PerfEventCGroup {
    let full_path: PathBuf = match cgroup_type {
        CGroupType::V1 => PathBuf::from(format!(
            "{}/perf_event/{}",
            mount_point,
            user_path.to_string_lossy()
        )),
        CGroupType::V2 => PathBuf::from(format!("{}/{}", mount_point, user_path.to_string_lossy())),
    };

    PerfEventCGroup {
        full_path,
        user_path: user_path.to_path_buf(),
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerfEventCGroup {
    user_path: PathBuf,
    full_path: PathBuf,
    pub(crate) cpi: f64,
    pub(crate) instructions: f64,
    pub(crate) cycles: f64,
    pub(crate) l3_cache_miss: f64,
    pub(crate) utilization: f64,
    pub(crate) icache_miss: f64,   // 未实现
    pub(crate) l2_cache_miss: f64, // 未实现
    update_time: u64,
}

impl PerfEventCGroup {
    pub fn user_path(&self) -> &Path {
        self.user_path.as_path()
    }
    pub fn full_path(&self) -> &Path {
        self.full_path.as_path()
    }
    pub fn new(full_path: PathBuf, user_path: PathBuf) -> PerfEventCGroup {
        PerfEventCGroup {
            user_path,
            full_path,
            ..Default::default()
        }
    }

    pub fn icache_miss(&self) -> f64 {
        self.icache_miss
    }

    pub fn l2_cache_miss(&self) -> f64 {
        self.l2_cache_miss
    }
    pub fn l3_cache_miss(&self) -> f64 {
        self.l3_cache_miss
    }

    pub fn cpi(&self) -> f64 {
        self.cpi
    }
    pub fn update_with_value(
        &mut self,
        cpi: f64,
        instructions: f64,
        cycles: f64,
        l3_cache_miss: f64,
        utilization: f64,
    ) -> common::Result<bool> {
        self.cpi = cpi;
        self.instructions = instructions;
        self.cycles = cycles;
        self.l3_cache_miss = l3_cache_miss;
        self.utilization = utilization;
        self.update_time = get_secs_since_epoch();
        Ok(true)
    }

    #[cfg(not(tarpaulin_include))]
    pub fn update(&mut self) {}

    pub fn update_ebpf(&mut self) {}

    pub fn reset(&mut self) {
        *self = Self::new(self.full_path.clone(), self.user_path.clone());
    }
}

#[cfg(not(tarpaulin_include))]
impl From<&byteperf_cgroup_buffer_malachite> for PerfEventCGroup {
    fn from(pmu_data: &byteperf_cgroup_buffer_malachite) -> Self {
        debug!("[byteperf debug] From function: {:?}", pmu_data);
        // TODO: check why there is 8 0 item in name
        let mut name_vec = pmu_data.name.to_vec();
        name_vec.retain(|&x| x != 0 as ::std::os::raw::c_char);
        let cgroup_path = unsafe { CStr::from_ptr(name_vec.as_ptr()).to_owned() };
        debug!("[byteperf] From function: cgroup_path = {:?}", cgroup_path);
        let mut cg_str = String::from(cgroup_path.to_str().unwrap());
        let full_path = String::from(cgroup_path.to_str().unwrap());
        if !cg_str.contains(MOUNT_POINT) {
            info!("[PMU] pmu data cgroup name error: {:?}", cgroup_path);
        }
        if !cg_str.contains("perf_event") {
            info!("[PMU] pmu data cgroup name error: {:?}", cgroup_path);
            // todo: error
        }
        let beta_offset = cg_str.find("kubepods").unwrap_or(cg_str.len());
        cg_str.drain(..beta_offset - 1);
        debug!("[byteperf] From function: user_path_str = {:?}", cg_str);
        let mut perf_event_cg =
            PerfEventCGroup::new(PathBuf::from(full_path), PathBuf::from(cg_str));
        perf_event_cg
            .update_with_value(
                pmu_data.cpi,
                pmu_data.instructions,
                pmu_data.cycles,
                pmu_data.l3_misses,
                pmu_data.utilization,
            )
            .unwrap();
        perf_event_cg
    }
}

#[cfg(test)]
mod tests_perf_event_cg {
    use super::*;
    use crate::cgroup::SubSystem;
    use std::env;
    use std::path::PathBuf;

    #[test]
    fn tests_perf_event_cg() {
        let mount_point: PathBuf = env::current_dir().unwrap();
        let user_path: String = String::from("pod_user_path");
        let full_path: PathBuf = PathBuf::from(format!(
            "{}/tests/sample/perf_event/{}",
            mount_point.to_string_lossy(),
            user_path
        ));
        let cpi: f64 = 1.0;
        let icache_miss: f64 = 2.0;
        let l2_cache_miss: f64 = 3.0;
        let l3_cache_miss: f64 = 4.0;
        let instructions: f64 = 5.0;
        let cycles: f64 = 6.0;
        let utilization: f64 = 7.0;
        let mut perf_event_cgroup = PerfEventCGroup::new(full_path, PathBuf::from(&user_path));

        perf_event_cgroup
            .update_with_value(cpi, instructions, cycles, l3_cache_miss, utilization)
            .unwrap();

        let correct_full_path = PathBuf::from(format!(
            "{}/tests/sample/perf_event/pod_user_path",
            mount_point.to_string_lossy()
        ));

        assert_eq!(perf_event_cgroup.full_path(), correct_full_path.as_path());
        assert_eq!(perf_event_cgroup.cpi, cpi);
        assert_eq!(perf_event_cgroup.instructions, instructions);
        assert_eq!(perf_event_cgroup.cycles, cycles);
        assert_eq!(perf_event_cgroup.utilization, utilization);
        assert_eq!(perf_event_cgroup.l3_cache_miss, l3_cache_miss);
        assert_eq!(
            perf_event_cgroup.user_path(),
            PathBuf::from(user_path.clone()).as_path()
        );

        // test for perf_event subsystem
        {
            let perf_event_subsystem = SubSystem::PerfEvent(perf_event_cgroup);
            assert_eq!(
                perf_event_subsystem.get_full_path(),
                correct_full_path.as_path()
            );
            assert_eq!(perf_event_subsystem.sub_system_path_exists(), true);
            assert_eq!(perf_event_subsystem.is_net_cgroup(), false);
            assert_eq!(perf_event_subsystem.is_perf_event_cgroup(), true);
        }
    }
}
