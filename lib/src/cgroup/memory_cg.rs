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

use crate::cgroup::utils;
use crate::common;
use crate::common::CGroupType;
use crate::ffi::{is_bpf_moudule_valid, wrapper_get_cgroup_mem_data, BPF_MODULE_CGROUP_MEM};
use crate::psi::PressureStallInfo;
use crate::system::get_secs_since_epoch;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use utoipa::ToSchema;

pub fn new_memory_cgroup(
    mount_point: &str,
    user_path: &Path,
    cgroup_type: CGroupType,
) -> MemoryCGroup {
    match cgroup_type {
        CGroupType::V1 => MemoryCGroup::V1(MemoryCGroupV1 {
            full_path: PathBuf::from(format!(
                "{}/memory/{}",
                mount_point,
                user_path.to_string_lossy()
            )),
            user_path: user_path.to_path_buf(),
            ..Default::default()
        }),
        CGroupType::V2 => MemoryCGroup::V2(MemoryCGroupV2 {
            full_path: PathBuf::from(format!("{}/{}", mount_point, user_path.to_string_lossy())),
            user_path: user_path.to_path_buf(),
            mem_numa_stats: HashMap::new(),
            ..Default::default()
        }),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum MemoryCGroup {
    /// cgroup v1 info from cgroupfs
    V1(MemoryCGroupV1),
    /// cgroup v1 info from cgroupfs
    V2(MemoryCGroupV2),
}

impl MemoryCGroup {
    pub fn full_path(&self) -> &Path {
        match self {
            MemoryCGroup::V1(v1) => v1.full_path(),
            MemoryCGroup::V2(v2) => v2.full_path(),
        }
    }

    pub fn update(&mut self) {
        match self {
            MemoryCGroup::V1(v1) => v1.update(),
            MemoryCGroup::V2(v2) => v2.update(),
        }
    }
    pub fn user_path(&mut self) -> &Path {
        match self {
            MemoryCGroup::V1(v1) => v1.user_path(),
            MemoryCGroup::V2(v2) => v2.user_path(),
        }
    }

    pub fn update_ebpf(&mut self) {
        if !is_bpf_moudule_valid(BPF_MODULE_CGROUP_MEM) {
            info!("memory bpf module is invalid");
            return;
        }

        match self {
            MemoryCGroup::V1(v1) => v1.update_ebpf(),
            MemoryCGroup::V2(v2) => v2.update_ebpf(),
        }
    }

    pub fn reset(&mut self) {
        match self {
            MemoryCGroup::V1(v1) => v1.reset(),
            MemoryCGroup::V2(v2) => v2.reset(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct MemoryCGroupV2 {
    full_path: PathBuf,
    user_path: PathBuf,
    mem_stats: MemStatsV2,
    mem_numa_stats: HashMap<String, MemNumaStatsV2>,
    mem_pressure: PressureStallInfo,
    mem_local_events: MemEventLocalV2,
    memory_usage_in_bytes: Option<u64>,
    max: Option<u64>,
    high: Option<u64>,
    low: Option<u64>,
    min: Option<u64>,
    swap_max: Option<u64>,
    watermark_scale_factor: Option<u64>,
    oom_cnt: Option<u64>,
    update_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct MemEventLocalV2 {
    low: u64,
    high: u64,
    max: u64,
    oom: u64,
    oom_kill: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct MemNumaStatsV2 {
    anon: u64,
    file: u64,
    kernel_stack: u64,
    shmem: u64,
    file_mapped: u64,
    file_dirty: u64,
    file_writeback: u64,
    anon_thp: u64,
    inactive_anon: u64,
    active_anon: u64,
    inactive_file: u64,
    active_file: u64,
    unevictable: u64,
    slab_reclaimable: u64,
    slab_unreclaimable: u64,
    workingset_refault: u64,
    workingset_activate: u64,
    workingset_nodereclaim: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct MemStatsV2 {
    anon: u64,
    file: u64,
    kernel_stack: u64,
    sock: u64,
    shmem: u64,
    file_mapped: u64,
    file_dirty: u64,
    file_writeback: u64,
    anon_thp: u64,
    inactive_anon: u64,
    active_anon: u64,
    inactive_file: u64,
    active_file: u64,
    unevictable: u64,
    slab_reclaimable: u64,
    slab_unreclaimable: u64,
    slab: u64,
    bgd_reclaim: u64,
    workingset_refault: u64,
    workingset_activate: u64,
    workingset_nodereclaim: u64,
    pgfault: u64,
    pgmajfault: u64,
    pgrefill: u64,
    pgscan: u64,
    pgsteal: u64,
    pgactivate: u64,
    pgdeactivate: u64,
    pglazyfree: u64,
    pglazyfreed: u64,
    thp_fault_alloc: u64,
    thp_collapse_alloc: u64,
}

impl MemoryCGroupV2 {
    fn full_path(&self) -> &Path {
        self.full_path.as_path()
    }
    fn user_path(&self) -> &Path {
        self.user_path.as_path()
    }

    fn update_oom_cnt(&mut self) -> common::Result<bool> {
        let path = PathBuf::from(&self.user_path());
        let mem_data = wrapper_get_cgroup_mem_data(path);
        let _ = self.oom_cnt.insert(mem_data.mem_oom_cnt);
        Ok(true)
    }

    fn update_memory_stat(&mut self) -> common::Result<bool> {
        let mut file = PathBuf::from(&self.full_path());
        file.push("memory.stat");
        let contents = fs::read_to_string(&file)?;
        let data = utils::parse_cgroup_file(contents.as_str());

        let stat = MemStatsV2::default();
        let r = serde_json::to_string(&stat).unwrap();
        let mut stat_map: Map<String, Value> = serde_json::from_str(&r).unwrap();
        for (key, value) in data {
            stat_map.insert(String::from(key), Value::Number(Number::from(value)));
        }

        let s = serde_json::to_string(&stat_map).unwrap();
        let mem_stats_v2: MemStatsV2 = serde_json::from_str(&s).unwrap();
        self.mem_stats = mem_stats_v2;
        Ok(true)
    }

    fn update_memory_numa_stat(&mut self) -> common::Result<bool> {
        let mut file = PathBuf::from(&self.full_path());
        file.push("memory.numa_stat");
        let contents = fs::read_to_string(&file)?;
        let mem_numa_stats = utils::parse_cgroup_reverse_nested_u64(&contents);
        for (numa_no, numa_stat) in mem_numa_stats {
            if numa_no.is_empty() {
                continue;
            }

            let stat = MemNumaStatsV2::default();
            let r = serde_json::to_string(&stat).unwrap();
            let mut stat_map: Map<String, Value> = serde_json::from_str(&r).unwrap();
            for (key, value) in numa_stat {
                stat_map.insert(key, Value::Number(Number::from(value)));
            }

            let s = serde_json::to_string(&stat_map).unwrap();
            let mem_numa_stats_v2: MemNumaStatsV2 = serde_json::from_str(&s).unwrap();
            self.mem_numa_stats.insert(numa_no, mem_numa_stats_v2);
        }

        Ok(true)
    }

    fn update_memory_pressure(&mut self) -> common::Result<bool> {
        let mut file = PathBuf::from(&self.full_path());
        file.push("memory.pressure");
        let contents = fs::read_to_string(&file)?;
        self.mem_pressure = PressureStallInfo::from(contents.as_str());
        Ok(true)
    }

    fn update_memory_distribution_setting(&mut self) -> common::Result<bool> {
        let v = vec![
            "memory.max",
            "memory.high",
            "memory.low",
            "memory.min",
            "memory.swap.max",
            "memory.watermark_scale_factor",
            "memory.current",
        ];
        let data: HashMap<&str, Option<u64>> = v
            .iter()
            .map(|s| {
                let mut path = PathBuf::from(&self.full_path);
                path.push(*s);
                (*s, utils::get_cgroup_value(path))
            })
            .collect();

        if let Some(value) = data.get(&"memory.max") {
            self.max = *value;
        }
        if let Some(value) = data.get(&"memory.high") {
            self.high = *value;
        }
        if let Some(value) = data.get(&"memory.low") {
            self.low = *value;
        }
        if let Some(value) = data.get(&"memory.min") {
            self.min = *value;
        }
        if let Some(value) = data.get(&"memory.swap.max") {
            self.swap_max = *value;
        }
        if let Some(value) = data.get(&"memory.watermark_scale_factor") {
            self.watermark_scale_factor = *value;
        }
        if let Some(value) = data.get(&"memory.current") {
            self.memory_usage_in_bytes = *value;
        }
        Ok(true)
    }

    fn update_memory_local_event(&mut self) -> common::Result<bool> {
        let mut file = PathBuf::from(&self.full_path());
        file.push("memory.events.local");
        let contents = fs::read_to_string(&file)?;
        let data = utils::parse_cgroup_file(contents.as_str());
        let s = serde_json::to_string(&data).unwrap();
        let mem_local_events_v2: MemEventLocalV2 = serde_json::from_str(&s).unwrap();
        self.mem_local_events = mem_local_events_v2;
        Ok(true)
    }

    fn update(&mut self) {
        if let Err(e) = self.update_memory_stat() {
            warn!(
                "[memcgv2] update memory stat error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }

        if let Err(e) = self.update_memory_numa_stat() {
            warn!(
                "[memcgv2] update memory numa stat error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }

        if let Err(e) = self.update_memory_distribution_setting() {
            warn!(
                "[memcgv2] update memory distribution error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }

        if let Err(e) = self.update_memory_local_event() {
            warn!(
                "[memcgv2] update memory local event error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }

        if let Err(e) = self.update_memory_pressure() {
            warn!(
                "[memcgv2] update memory pressure error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }
    }

    fn update_ebpf(&mut self) {
        if let Err(e) = self.update_oom_cnt() {
            warn!(
                "[memcgv2] update oom cnt error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }
        self.update_time = get_secs_since_epoch()
    }

    fn reset(&mut self) {
        *self = Self {
            full_path: self.full_path.clone(),
            user_path: self.user_path.clone(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MemoryCGroupNumaStat {
    numa_name: String,
    total: Option<u64>,
    file: Option<u64>,
    anon: Option<u64>,
    unevictable: Option<u64>,
    hierarchical_total: Option<u64>,
    hierarchical_file: Option<u64>,
    hierarchical_anon: Option<u64>,
    hierarchical_unevictable: Option<u64>,
}

impl From<(&str, HashMap<&str, u64>)> for MemoryCGroupNumaStat {
    fn from((name, value): (&str, HashMap<&str, u64>)) -> Self {
        MemoryCGroupNumaStat {
            numa_name: name.to_string(),
            total: value.get(&"total").cloned(),
            file: value.get(&"file").cloned(),
            anon: value.get(&"anon").cloned(),
            unevictable: value.get(&"unevictable").cloned(),
            hierarchical_total: value.get(&"hierarchical_total").cloned(),
            hierarchical_file: value.get(&"hierarchical_file").cloned(),
            hierarchical_anon: value.get(&"hierarchical_anon").cloned(),
            hierarchical_unevictable: value.get(&"hierarchical_unevictable").cloned(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct MemoryCGroupV1 {
    full_path: PathBuf,
    user_path: PathBuf,
    memory_limit_in_bytes: Option<u64>,
    memory_usage_in_bytes: Option<u64>,
    kern_memory_usage_in_bytes: Option<u64>,
    cache: Option<u64>,
    rss: Option<u64>,
    shmem: Option<u64>,
    dirty: Option<u64>,
    kswapd_steal: Option<u64>,
    writeback: Option<u64>,
    pgfault: Option<u64>,
    pgmajfault: Option<u64>,
    allocstall: Option<u64>,
    total_cache: Option<u64>,
    total_rss: Option<u64>,
    total_shmem: Option<u64>,
    total_dirty: Option<u64>,
    total_kswapd_steal: Option<u64>,
    total_writeback: Option<u64>,
    total_pgfault: Option<u64>,
    total_pgmajfault: Option<u64>,
    total_allocstall: Option<u64>,
    watermark_scale_factor: Option<usize>,
    oom_cnt: Option<u64>,
    numa_stat: Option<Vec<MemoryCGroupNumaStat>>,
    update_time: u64,
}

impl MemoryCGroupV1 {
    pub fn update_numa_stat(&mut self) -> common::Result<bool> {
        let mut file = PathBuf::from(&self.full_path);
        file.push("memory.numa_stat");
        let contents = fs::read_to_string(&file)?;
        let data: HashMap<&str, HashMap<&str, u64>> =
            utils::parse_cgroup_numa_stat_file(contents.as_str());
        let numa_stat_vec = data
            .into_iter()
            .map(|(key, value)| MemoryCGroupNumaStat::from((key, value)))
            .collect::<Vec<MemoryCGroupNumaStat>>();
        self.numa_stat = Some(numa_stat_vec);
        Ok(true)
    }

    fn update_memory_stat(&mut self) -> common::Result<bool> {
        let mut file = PathBuf::from(&self.full_path());
        file.push("memory.stat");
        let contents = fs::read_to_string(&file)?;
        let data: HashMap<&str, u64> = utils::parse_cgroup_file(contents.as_str());
        self.cache = data.get(&"cache").cloned();
        self.rss = data.get(&"rss").cloned();
        self.shmem = data.get(&"shmem").cloned();
        self.dirty = data.get(&"dirty").cloned();
        self.kswapd_steal = data.get(&"kswapd_steal").cloned();
        self.writeback = data.get(&"writeback").cloned();
        self.pgfault = data.get(&"pgfault").cloned();
        self.pgmajfault = data.get(&"pgmajfault").cloned();
        self.allocstall = data.get(&"allocstall").cloned();
        self.total_cache = data.get(&"total_cache").cloned();
        self.total_rss = data.get(&"total_rss").cloned();
        self.total_shmem = data.get(&"total_shmem").cloned();
        self.total_dirty = data.get(&"total_dirty").cloned();
        self.total_kswapd_steal = data.get(&"total_kswapd_steal").cloned();
        self.total_writeback = data.get(&"total_writeback").cloned();
        self.total_pgfault = data.get(&"total_pgfault").cloned();
        self.total_pgmajfault = data.get(&"total_pgmajfault").cloned();
        self.total_allocstall = data.get(&"total_allocstall").cloned();
        Ok(true)
    }

    fn update_kern_memory_usage(&mut self) -> common::Result<bool> {
        let mut path = PathBuf::from(&self.full_path);
        path.push("memory.kmem.usage_in_bytes");
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        if let Some(Ok(data)) = reader.lines().next() {
            self.kern_memory_usage_in_bytes = Some(data.parse::<u64>()?);
        }
        Ok(true)
    }

    pub fn user_memory_usage_in_bytes(&self) -> Option<u64> {
        if let Some(a) = self.memory_usage_in_bytes {
            if let Some(b) = self.kern_memory_usage_in_bytes {
                return Some(a - b);
            }
        }
        None
    }

    fn update_memory_limit(&mut self) -> common::Result<bool> {
        let mut path = PathBuf::from(&self.full_path);
        path.push("memory.limit_in_bytes");
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        if let Some(Ok(data)) = reader.lines().next() {
            self.memory_limit_in_bytes = Some(data.parse::<u64>()?);
        }
        Ok(true)
    }

    fn update_memory_usage_in_bytes(&mut self) -> common::Result<bool> {
        let mut path = PathBuf::from(&self.full_path);
        path.push("memory.usage_in_bytes");
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        if let Some(Ok(data)) = reader.lines().next() {
            self.memory_usage_in_bytes = Some(data.parse::<u64>()?);
        }
        Ok(true)
    }

    fn update_watermark_scale_factor(&mut self) -> common::Result<bool> {
        let mut path = PathBuf::from(&self.full_path);
        path.push("memory.watermark_scale_factor");
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        if let Some(Ok(data)) = reader.lines().next() {
            self.watermark_scale_factor = Some(data.parse::<usize>().unwrap());
        }
        Ok(true)
    }

    fn full_path(&self) -> &Path {
        self.full_path.as_path()
    }

    fn user_path(&self) -> &Path {
        self.user_path.as_path()
    }

    pub fn update_oom_cnt(&mut self) -> common::Result<bool> {
        let path = PathBuf::from(&self.user_path());
        let mem_data = wrapper_get_cgroup_mem_data(path);
        self.oom_cnt = Some(mem_data.mem_oom_cnt);
        Ok(true)
    }

    fn update(&mut self) {
        if let Err(e) = self.update_memory_limit() {
            warn!(
                "[memcg] update memory limit error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }
        if let Err(e) = self.update_memory_stat() {
            warn!(
                "[memcg] update memory stat error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }
        if let Err(e) = self.update_memory_limit() {
            warn!(
                "[memcg] update memory limit error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }
        if let Err(e) = self.update_kern_memory_usage() {
            warn!(
                "[memcg] update kern memory usage error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }
        if let Err(e) = self.update_memory_usage_in_bytes() {
            warn!(
                "[memcg] update memory usage error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }
        if let Err(e) = self.update_watermark_scale_factor() {
            warn!(
                "[memcg] update watermark scale factor error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }
        if let Err(e) = self.update_numa_stat() {
            warn!(
                "[memcg] update numa stat error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }
    }

    fn update_ebpf(&mut self) {
        if let Err(e) = self.update_oom_cnt() {
            warn!(
                "[memcg] update oom cnt error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }
        self.update_time = get_secs_since_epoch()
    }

    fn reset(&mut self) {
        *self = Self {
            full_path: self.full_path.clone(),
            user_path: self.user_path.clone(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests_memory_cg {
    use super::*;
    use crate::cgroup::{new_memory_cgroup, SubSystem};
    use std::env;
    use std::path::PathBuf;

    #[test]
    fn test_memory_cg() {
        let mount_point: String = String::from(format!(
            "{}/tests/sample",
            env::current_dir().unwrap().to_string_lossy()
        ));
        let user_path: PathBuf = PathBuf::from("pod_user_path");

        let mut cg = new_memory_cgroup(&mount_point, &user_path, CGroupType::V1 {});
        cg.update();

        let correct_full_path = PathBuf::from(format!("{}/memory/pod_user_path", mount_point));

        if let MemoryCGroup::V1(memory_cg) = cg {
            assert_eq!(memory_cg.full_path(), correct_full_path);
            assert_eq!(memory_cg.memory_limit_in_bytes, Some(34359738368 as u64));
            assert_eq!(memory_cg.kern_memory_usage_in_bytes, Some(850628608 as u64));
            assert_eq!(memory_cg.memory_usage_in_bytes, Some(243873845248 as u64));
            assert_eq!(
                memory_cg.user_memory_usage_in_bytes(),
                Some((243873845248 as u64 - 850628608 as u64) as u64)
            );
            assert_eq!(memory_cg.cache, Some(8611385344 as u64));
            assert_eq!(memory_cg.rss, Some(4919517184 as u64));
            assert_eq!(memory_cg.shmem, Some(1097728 as u64));
            assert_eq!(memory_cg.dirty, Some(1835008 as u64));
            assert_eq!(memory_cg.kswapd_steal, Some(25 as u64));
            assert_eq!(memory_cg.writeback, Some(13 as u64));
            assert_eq!(memory_cg.pgfault, Some(843170397 as u64));
            assert_eq!(memory_cg.pgmajfault, Some(85 as u64));
            assert_eq!(memory_cg.allocstall, Some(11 as u64));
            assert_eq!(memory_cg.total_cache, Some(8611385344 as u64));
            assert_eq!(memory_cg.total_rss, Some(4919517184 as u64));
            assert_eq!(memory_cg.total_shmem, Some(1097728 as u64));
            assert_eq!(memory_cg.total_dirty, Some(1835008 as u64));
            assert_eq!(memory_cg.total_kswapd_steal, Some(25 as u64));
            assert_eq!(memory_cg.total_writeback, Some(13 as u64));
            assert_eq!(memory_cg.total_pgfault, Some(843170397 as u64));
            assert_eq!(memory_cg.total_pgmajfault, Some(85 as u64));
            assert_eq!(memory_cg.total_allocstall, Some(11 as u64));
            assert_eq!(memory_cg.watermark_scale_factor, Some(100 as usize));

            // test for memory subsystem
            {
                let memory_subsystem = SubSystem::Memory(MemoryCGroup::V1(memory_cg));
                assert_eq!(
                    memory_subsystem.get_full_path(),
                    correct_full_path.as_path()
                );
                assert_eq!(memory_subsystem.sub_system_path_exists(), true);
                assert_eq!(memory_subsystem.is_net_cgroup(), false);
                assert_eq!(memory_subsystem.is_memory_cgroup(), true);
            }
        }
    }

    #[test]
    fn test_memory_cg_v2() {
        let mount_point: String = env::current_dir()
            .unwrap()
            .into_os_string()
            .into_string()
            .unwrap();
        let user_path: PathBuf = PathBuf::from("tests/sample/cgroupv2");

        let mut cg = new_memory_cgroup(&mount_point, &user_path, CGroupType::V2 {});
        cg.update();

        let correct_full_path = PathBuf::from(format!("{}/tests/sample/cgroupv2", mount_point));

        if let MemoryCGroup::V2(memory_cg) = cg {
            assert_eq!(memory_cg.full_path(), correct_full_path);
            assert_eq!(memory_cg.max, Some(5368709120 as u64));
            assert_eq!(memory_cg.high, Some(18446744073709551615 as u64));
            assert_eq!(memory_cg.low, Some(0 as u64));
            assert_eq!(memory_cg.min, Some(0 as u64));
            assert_eq!(memory_cg.swap_max, Some(18446744073709551615 as u64));
            assert_eq!(memory_cg.watermark_scale_factor, Some(0 as u64));

            // memory.stat
            assert_eq!(memory_cg.mem_stats.anon, 13787136);
            assert_eq!(memory_cg.mem_stats.kernel_stack, 811008);
            assert_eq!(memory_cg.mem_stats.pgfault, 535821);

            //memory.numa_stat
            assert_eq!(memory_cg.mem_numa_stats["N0"].anon, 12029952);
            assert_eq!(memory_cg.mem_numa_stats["N1"].file_mapped, 4730880);

            //memory.pressure
            assert_eq!(memory_cg.mem_pressure.get_some().unwrap().get_avg10(), 1.23);
            assert_eq!(
                memory_cg.mem_pressure.get_full().unwrap().get_total(),
                666.0
            );

            //memory.events.local
            assert_eq!(memory_cg.mem_local_events.low, 1);
            assert_eq!(memory_cg.mem_local_events.oom_kill, 666);

            // test for memory subsystem
            {
                let memory_subsystem = SubSystem::Memory(MemoryCGroup::V2(memory_cg));
                assert_eq!(
                    memory_subsystem.get_full_path(),
                    correct_full_path.as_path()
                );
                assert_eq!(memory_subsystem.sub_system_path_exists(), true);
                assert_eq!(memory_subsystem.is_net_cgroup(), false);
                assert_eq!(memory_subsystem.is_memory_cgroup(), true);
            }
        }
    }
}
