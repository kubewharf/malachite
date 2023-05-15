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
use crate::cpu::NodeVec;
use crate::ffi::{
    is_bpf_moudule_valid, wrapper_get_cgroup_nr_tasks, wrapper_get_cgroup_pmu_data,
    BPF_MODULE_CGROUP_PMU, BPF_MODULE_CGROUP_SCHED,
};
use crate::psi::PressureStallInfo;
use crate::system::{get_secs_since_epoch, LoadAvg};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::ops::Sub;
use std::path::{Path, PathBuf};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum CpuSetCGroup {
    V1(CpuSetCGroupV1),
    V2(CpuSetCGroupV2),
}

impl CpuSetCGroup {
    pub fn full_path(&self) -> &Path {
        match self {
            CpuSetCGroup::V1(v1) => v1.full_path(),
            CpuSetCGroup::V2(v2) => v2.full_path(),
        }
    }

    pub fn update(&mut self) {
        match self {
            CpuSetCGroup::V1(v1) => v1.update(),
            CpuSetCGroup::V2(v2) => v2.update(),
        }
    }

    pub fn update_ebpf(&mut self) {}

    pub fn reset(&mut self) {
        match self {
            CpuSetCGroup::V1(v1) => v1.reset(),
            CpuSetCGroup::V2(v2) => v2.reset(),
        }
    }
}

pub fn new_cpuset_cgroup(
    mount_point: &str,
    user_path: &Path,
    cgroup_type: CGroupType,
) -> CpuSetCGroup {
    match cgroup_type {
        CGroupType::V1 => CpuSetCGroup::V1(CpuSetCGroupV1 {
            full_path: PathBuf::from(format!(
                "{}/cpuset/{}",
                mount_point,
                user_path.to_string_lossy()
            )),
            mems: NodeVec::new(),
            cpus: NodeVec::new(),
            update_time: 0,
        }),
        CGroupType::V2 => CpuSetCGroup::V2(CpuSetCGroupV2 {
            full_path: PathBuf::from(format!("{}/{}", mount_point, user_path.to_string_lossy())),
            mems: NodeVec::new(),
            cpus: NodeVec::new(),
            update_time: 0,
        }),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct CpuSetCGroupV2 {
    full_path: PathBuf,
    pub(crate) mems: NodeVec,
    pub(crate) cpus: NodeVec,
    update_time: u64,
}

impl CpuSetCGroupV2 {
    pub fn full_path(&self) -> &Path {
        self.full_path.as_path()
    }

    pub fn mems(&self) -> &NodeVec {
        &self.mems
    }

    pub fn cpus(&self) -> &NodeVec {
        &self.cpus
    }

    pub fn update_cpus(&mut self) -> common::Result<String> {
        let mut path = PathBuf::from(&self.full_path);
        path.push("cpuset.cpus.effective");
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        if let Some(Ok(data)) = reader.lines().next() {
            self.cpus = NodeVec::from(data);
            Ok("".into())
        } else {
            Err("read cpuset.mems error".into())
        }
    }

    pub fn update_mems(&mut self) -> common::Result<String> {
        let mut path = PathBuf::from(&self.full_path);
        path.push("cpuset.mems.effective");
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        if let Some(Ok(data)) = reader.lines().next() {
            self.mems = NodeVec::from(data);
            Ok("".into())
        } else {
            Err("read cpuset.mems error".into())
        }
    }

    pub fn update(&mut self) {
        if let Err(e) = self.update_cpus() {
            warn!("[cpusetcg] update cpus error: {}", e);
        }
        if let Err(e) = self.update_mems() {
            warn!("[cpusetcg] update mems error: {}", e);
        }
        self.update_time = get_secs_since_epoch()
    }

    pub fn reset(&mut self) {
        *self = Self {
            full_path: self.full_path.clone(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct CpuSetCGroupV1 {
    full_path: PathBuf,
    pub(crate) mems: NodeVec,
    pub(crate) cpus: NodeVec,
    update_time: u64,
}

impl CpuSetCGroupV1 {
    pub fn full_path(&self) -> &Path {
        self.full_path.as_path()
    }
    pub fn mems(&self) -> &NodeVec {
        &self.mems
    }
    pub fn cpus(&self) -> &NodeVec {
        &self.cpus
    }

    pub fn update_cpus(&mut self) -> common::Result<String> {
        let mut path = PathBuf::from(&self.full_path);
        path.push("cpuset.cpus");
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        if let Some(Ok(data)) = reader.lines().next() {
            self.cpus = NodeVec::from(data);
            Ok("".into())
        } else {
            Err("read cpuset.mems error".into())
        }
    }

    pub fn update_mems(&mut self) -> common::Result<String> {
        let mut path = PathBuf::from(&self.full_path);
        path.push("cpuset.mems");
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        if let Some(Ok(data)) = reader.lines().next() {
            self.mems = NodeVec::from(data);
            Ok("".into())
        } else {
            Err("read cpuset.mems error".into())
        }
    }

    pub fn update(&mut self) {
        if let Err(e) = self.update_cpus() {
            warn!(
                "[cpusetcg] update cpus error: {}, path= {:?}",
                e, self.full_path
            );
        }
        if let Err(e) = self.update_mems() {
            warn!(
                "[cpusetcg] update mems error: {}, path= {:?}",
                e, self.full_path
            );
        }
        self.update_time = get_secs_since_epoch()
    }
    pub fn reset(&mut self) {
        *self = Self {
            full_path: self.full_path.clone(),
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct CpuCGroupBasicInfo {
    cpu_usage: u64,     // unit: ns
    cpu_user_time: u64, // unit: ns
    cpu_sys_time: u64,  // unit: ns
    update_time: u64,   // unit: s
}

impl Default for CpuCGroupBasicInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuCGroupBasicInfo {
    fn new() -> Self {
        Self {
            cpu_usage: 0,
            cpu_user_time: 0,
            cpu_sys_time: 0,
            update_time: 0,
        }
    }

    fn update(&mut self, cpu_usage: u64, cpu_user_time: u64, cpu_sys_time: u64) {
        self.cpu_usage = cpu_usage;
        self.cpu_user_time = cpu_user_time;
        self.cpu_sys_time = cpu_sys_time;
        self.update_time = get_secs_since_epoch()
    }
}

impl Sub for CpuCGroupBasicInfo {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        Self {
            cpu_usage: self.cpu_usage - other.cpu_usage,
            cpu_user_time: self.cpu_user_time - other.cpu_user_time,
            cpu_sys_time: self.cpu_sys_time - other.cpu_sys_time,
            update_time: self.update_time - other.update_time,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum CpuCGroup {
    V1(CpuCGroupV1),
    V2(CpuCGroupV2),
}

impl CpuCGroup {
    pub fn full_path(&self) -> &Path {
        match self {
            CpuCGroup::V1(v1) => v1.full_path(),
            CpuCGroup::V2(v2) => v2.full_path(),
        }
    }

    pub fn update(&mut self) {
        match self {
            CpuCGroup::V1(v1) => v1.update(),
            CpuCGroup::V2(v2) => v2.update(),
        }
    }

    pub fn update_ebpf(&mut self) {
        match self {
            CpuCGroup::V1(v1) => v1.update_ebpf(),
            CpuCGroup::V2(v2) => v2.update_ebpf(),
        }
    }

    pub fn reset(&mut self) {
        match self {
            CpuCGroup::V1(v1) => v1.reset(),
            CpuCGroup::V2(v2) => v2.reset(),
        }
    }
}

pub fn new_cpu_cgroup(mount_point: &str, user_path: &Path, cgroup_type: CGroupType) -> CpuCGroup {
    match cgroup_type {
        CGroupType::V1 => CpuCGroup::V1(CpuCGroupV1 {
            full_path: PathBuf::from(format!(
                "{}/cpuacct/{}",
                mount_point,
                user_path.to_string_lossy()
            )),
            user_path: user_path.to_path_buf(),
            ..Default::default()
        }),
        CGroupType::V2 => CpuCGroup::V2(CpuCGroupV2 {
            full_path: PathBuf::from(format!("{}/{}", mount_point, user_path.to_string_lossy())),
            user_path: user_path.to_path_buf(),
            ..Default::default()
        }),
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Default, ToSchema)]
pub struct CpuCGroupV2 {
    full_path: PathBuf,
    user_path: PathBuf,
    cpu_stats: CpuStatsV2,
    cpu_pressure: PressureStallInfo,
    weight: Option<u64>,
    weight_nice: Option<u64>,
    max_burst: Option<u64>,
    max: Option<u64>,
    max_period: Option<u64>,
    task_nr_iowait: u64,
    task_nr_unint: u64,
    task_nr_runnable: u64,
    load: Option<LoadAvg>,
    cpu_usage_ratio: f32,
    cpu_user_usage_ratio: f32,
    cpu_sys_usage_ratio: f32,
    update_time: u64,
    cycles: u64,
    instructions: u64,
    ocr_read_drams: u64,
    store_ins: u64,
    store_all_ins: u64,
    imc_writes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, ToSchema)]
pub struct CpuStatsV2 {
    usage_usec: u64,
    user_usec: u64,
    system_usec: u64,
    nr_periods: u64,
    nr_throttled: u64,
    throttled_usec: u64,
    update_time: u64, // unit: s
}

impl CpuCGroupV2 {
    pub fn full_path(&self) -> &Path {
        self.full_path.as_path()
    }

    fn update_cpu_stat(&mut self) -> common::Result<bool> {
        let usage_usec = self.cpu_stats.usage_usec;
        let user_usec = self.cpu_stats.user_usec;
        let system_usec = self.cpu_stats.system_usec;
        let update_time = self.cpu_stats.update_time;

        let mut file = PathBuf::from(&self.full_path());
        file.push("cpu.stat");
        let contents = fs::read_to_string(&file)?;
        let data = utils::parse_cgroup_file(contents.as_str());

        let stat = CpuStatsV2::default();
        let r = serde_json::to_string(&stat).unwrap();
        let mut stat_map: Map<String, Value> = serde_json::from_str(&r).unwrap();
        for (key, value) in data {
            stat_map.insert(String::from(key), Value::Number(Number::from(value)));
        }
        stat_map.insert(
            String::from("update_time"),
            Value::Number(Number::from(get_secs_since_epoch())),
        );

        let s = serde_json::to_string(&stat_map).unwrap();
        let cpu_stat: CpuStatsV2 = serde_json::from_str(&s).unwrap();
        self.cpu_stats = cpu_stat;

        if update_time == 0 {
            return Ok(true);
        }

        let delta_time = (self.cpu_stats.update_time - update_time) * 1_000_000;
        self.cpu_usage_ratio = (self.cpu_stats.usage_usec - usage_usec) as f32 / delta_time as f32;
        self.cpu_user_usage_ratio =
            (self.cpu_stats.user_usec - user_usec) as f32 / delta_time as f32;
        self.cpu_sys_usage_ratio =
            (self.cpu_stats.system_usec - system_usec) as f32 / delta_time as f32;
        Ok(true)
    }

    fn update_cpu_distribution_setting(&mut self) -> common::Result<bool> {
        let v = vec!["cpu.weight", "cpu.weight.nice", "cpu.max.burst"];
        let data: HashMap<&str, Option<u64>> = v
            .iter()
            .map(|s| {
                let mut path = PathBuf::from(&self.full_path);
                path.push(*s);
                (*s, utils::get_cgroup_value(path))
            })
            .collect();

        if let Some(value) = data.get(&"cpu.weight") {
            self.weight = *value;
        }
        if let Some(value) = data.get(&"cpu.weight.nice") {
            self.weight_nice = *value;
        }
        if let Some(value) = data.get(&"cpu.max.burst") {
            self.max_burst = *value;
        }

        Ok(true)
    }

    fn update_cpu_max(&mut self) -> common::Result<bool> {
        let mut path = PathBuf::from(&self.full_path);
        path.push("cpu.max");
        let ret = File::open(&path);
        match ret {
            Ok(file) => {
                let reader = BufReader::new(file);
                if let Some(Ok(data)) = reader.lines().next() {
                    let mut s = data.split_whitespace();
                    if let Some(max) = s.next() {
                        if max == "max" {
                            self.max = Some(std::u64::MAX);
                        } else {
                            self.max = Some(max.parse::<u64>().unwrap_or(0));
                        }
                    }

                    if let Some(period) = s.next() {
                        self.max_period = Some(period.parse::<u64>().unwrap_or(0));
                    }
                }
            }
            Err(error) => {
                warn!("open path {:?} failed: {:?}", path, error);
                // TODO: get cpu.max from parent path
                self.max = Some(std::u64::MAX);
                self.max_period = Some(100000);
            }
        };

        Ok(true)
    }

    fn update_cpu_pressure(&mut self) -> common::Result<bool> {
        let mut file = PathBuf::from(&self.full_path());
        file.push("cpu.pressure");
        let contents = fs::read_to_string(&file)?;
        self.cpu_pressure = PressureStallInfo::from(contents.as_str());
        Ok(true)
    }

    pub fn update_cgroup_load(&mut self) -> common::Result<bool> {
        let user_path = PathBuf::from(&self.user_path);
        let task_nr_data = wrapper_get_cgroup_nr_tasks(user_path);
        self.task_nr_iowait = task_nr_data.nr_iowait;
        self.task_nr_unint = task_nr_data.nr_unint;
        self.task_nr_runnable = task_nr_data.nr_runnable;
        if self.load.is_none() {
            self.load = Some(LoadAvg::new())
        }
        self.load
            .as_mut()
            .unwrap()
            .calc_load(self.task_nr_unint + self.task_nr_runnable);
        Ok(true)
    }

    pub fn update(&mut self) {
        if let Err(e) = self.update_cpu_stat() {
            warn!(
                "[cpucg] update cpu stat error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }
        if let Err(e) = self.update_cpu_distribution_setting() {
            warn!(
                "[cpucg] update cpu distribution setting error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }
        if let Err(e) = self.update_cpu_max() {
            warn!(
                "[cpucg] update cpu max error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }
        if let Err(e) = self.update_cpu_pressure() {
            warn!(
                "[cpucg] update cpu pressure error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }
    }

    pub fn update_ebpf(&mut self) {
        if !is_bpf_moudule_valid(BPF_MODULE_CGROUP_SCHED) {
            info!("bpf sched module is invalid");
        } else if let Err(e) = self.update_cgroup_load() {
            warn!(
                "[cpucg] update cgroup load with bpf error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }

        if !is_bpf_moudule_valid(BPF_MODULE_CGROUP_PMU) {
            info!("bpf pmu module is invalid");
        } else if let Err(e) = self.update_cgroup_pmu() {
            warn!(
                "[cpucg] update cgroup cpi with bpf error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }
    }

    pub fn update_cgroup_pmu(&mut self) -> common::Result<bool> {
        let user_path = PathBuf::from(&self.user_path);
        let pmu_data = wrapper_get_cgroup_pmu_data(user_path);
        self.cycles = pmu_data.cycles;
        self.instructions = pmu_data.instructions;
        self.ocr_read_drams = pmu_data.ocr_read_drams;
        self.store_ins = pmu_data.store_ins;
        self.store_all_ins = pmu_data.store_all_ins;
        self.imc_writes = pmu_data.imc_writes;
        Ok(true)
    }

    pub fn reset(&mut self) {
        *self = Self {
            full_path: self.full_path.clone(),
            user_path: self.user_path.clone(),
            ..Default::default()
        };
        self.update_time = get_secs_since_epoch()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Default, ToSchema)]
pub struct CpuCGroupV1 {
    full_path: PathBuf,
    user_path: PathBuf,
    cfs_quota_us: i64,
    cfs_period_us: u64,
    cpu_shares: u64,

    old_cpu_basic_info: CpuCGroupBasicInfo,
    new_cpu_basic_info: CpuCGroupBasicInfo,

    task_nr_iowait: u64,
    task_nr_unint: u64,
    task_nr_runnable: u64,

    cpu_usage_ratio: f32,
    cpu_user_usage_ratio: f32,
    cpu_sys_usage_ratio: f32,

    percpu_usage: Vec<u64>,

    cpu_nr_throttled: Option<u64>,
    cpu_nr_periods: Option<u64>,
    cpu_throttled_time: Option<u64>,

    load: Option<LoadAvg>,
    update_time: u64,

    cycles: u64,
    instructions: u64,
    ocr_read_drams: u64,
    store_ins: u64,
    store_all_ins: u64,
    imc_writes: u64,
}

impl CpuCGroupV1 {
    pub fn full_path(&self) -> &Path {
        self.full_path.as_path()
    }

    pub fn update_cpu_stat(&mut self) -> common::Result<bool> {
        let mut file = PathBuf::from(&self.full_path);
        file.push("cpu.stat");
        let contents = fs::read_to_string(&file)?;
        let data: HashMap<&str, u64> = utils::parse_cgroup_file(&contents);
        self.cpu_nr_throttled = data.get(&"nr_throttled").cloned();
        self.cpu_throttled_time = data.get(&"throttled_time").cloned();
        self.cpu_nr_periods = data.get(&"nr_periods").cloned();
        Ok(true)
    }

    fn get_cpu_usage(&self) -> u64 {
        let mut path = PathBuf::from(&self.full_path);
        path.push("cpuacct.usage");
        let file = File::open(&path).unwrap();
        let reader = BufReader::new(file);
        if let Some(Ok(data)) = reader.lines().next() {
            return data.parse::<u64>().unwrap();
        }
        0
    }

    fn get_cpu_sys_time(&self) -> u64 {
        let mut path = PathBuf::from(&self.full_path);
        path.push("cpuacct.usage_sys");
        let file = File::open(&path).unwrap();
        let reader = BufReader::new(file);
        if let Some(Ok(data)) = reader.lines().next() {
            return data.parse::<u64>().unwrap();
        }
        0
    }

    fn get_cpu_user_time(&self) -> u64 {
        let mut path = PathBuf::from(&self.full_path);
        path.push("cpuacct.usage_user");
        let file = File::open(&path).unwrap();
        let reader = BufReader::new(file);
        if let Some(Ok(data)) = reader.lines().next() {
            return data.parse::<u64>().unwrap_or(0);
        }
        0
    }

    pub fn update_cpu_basic_info(&mut self) -> common::Result<bool> {
        let cpu_usage = self.get_cpu_usage();
        let cpu_usage_user = self.get_cpu_user_time();
        let cpu_usage_sys = self.get_cpu_sys_time();
        self.old_cpu_basic_info = self.new_cpu_basic_info;
        self.new_cpu_basic_info
            .update(cpu_usage, cpu_usage_user, cpu_usage_sys);
        let delta_cpu_basic_info = self.new_cpu_basic_info - self.old_cpu_basic_info;
        let update_time = delta_cpu_basic_info.update_time * 1_000_000_000;
        self.cpu_usage_ratio = delta_cpu_basic_info.cpu_usage as f32 / update_time as f32;
        self.cpu_user_usage_ratio = delta_cpu_basic_info.cpu_user_time as f32 / update_time as f32;
        self.cpu_sys_usage_ratio = delta_cpu_basic_info.cpu_sys_time as f32 / update_time as f32;
        Ok(true)
    }

    pub fn update_period_us(&mut self) -> common::Result<bool> {
        let mut path = PathBuf::from(&self.full_path);
        path.push("cpu.cfs_period_us");
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        if let Some(Ok(data)) = reader.lines().next() {
            self.cfs_period_us = data.parse::<u64>()?;
        }
        Ok(true)
    }

    pub fn update_quota_us(&mut self) -> common::Result<bool> {
        let mut path = PathBuf::from(&self.full_path);
        path.push("cpu.cfs_quota_us");
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        if let Some(Ok(data)) = reader.lines().next() {
            self.cfs_quota_us = data.parse::<i64>()?;
        }
        Ok(true)
    }

    pub fn update_cpu_shares(&mut self) -> common::Result<bool> {
        let mut path = PathBuf::from(&self.full_path);
        path.push("cpu.shares");
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        if let Some(Ok(data)) = reader.lines().next() {
            self.cpu_shares = data.parse::<u64>()?;
        }
        Ok(true)
    }

    pub fn update_cgroup_load(&mut self) -> common::Result<bool> {
        let user_path = PathBuf::from(&self.user_path);
        let task_nr_data = wrapper_get_cgroup_nr_tasks(user_path);
        self.task_nr_iowait = task_nr_data.nr_iowait;
        self.task_nr_unint = task_nr_data.nr_unint;
        self.task_nr_runnable = task_nr_data.nr_runnable;
        if self.load.is_none() {
            self.load = Some(LoadAvg::new())
        }
        self.load
            .as_mut()
            .unwrap()
            .calc_load(self.task_nr_unint + self.task_nr_runnable);
        Ok(true)
    }

    pub fn update_cgroup_pmu(&mut self) -> common::Result<bool> {
        let user_path = PathBuf::from(&self.user_path);
        let pmu_data = wrapper_get_cgroup_pmu_data(user_path);
        self.cycles = pmu_data.cycles;
        self.instructions = pmu_data.instructions;
        self.ocr_read_drams = pmu_data.ocr_read_drams;
        self.store_ins = pmu_data.store_ins;
        self.store_all_ins = pmu_data.store_all_ins;
        self.imc_writes = pmu_data.imc_writes;
        Ok(true)
    }

    pub fn update_percpu_usage(&mut self) -> common::Result<bool> {
        let mut path = PathBuf::from(&self.full_path);
        path.push("cpuacct.usage_percpu");
        let contents = fs::read_to_string(&path)?;
        self.percpu_usage = contents
            .split_whitespace()
            .map(|x| x.parse::<u64>().unwrap())
            .collect();

        Ok(true)
    }

    pub fn update(&mut self) {
        if let Err(e) = self.update_period_us() {
            warn!(
                "[cpucg] update cpu period us error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }
        if let Err(e) = self.update_quota_us() {
            warn!(
                "[cpucg] update cpu quota us error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }
        if let Err(e) = self.update_cpu_basic_info() {
            warn!(
                "[cpucg] update cpu basic info error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }
        if let Err(e) = self.update_cpu_stat() {
            warn!(
                "[cpucg] update cpu stat error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }
        if let Err(e) = self.update_cpu_shares() {
            warn!(
                "[cpucg] update cpu shares error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }
        if let Err(e) = self.update_percpu_usage() {
            warn!(
                "[cpucg] update percpu usage error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }
        self.update_time = get_secs_since_epoch()
    }

    pub fn update_ebpf(&mut self) {
        if !is_bpf_moudule_valid(BPF_MODULE_CGROUP_SCHED) {
            info!("bpf sched module is invalid");
        } else if let Err(e) = self.update_cgroup_load() {
            warn!(
                "[cpucg] update cgroup load with bpf error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }

        if !is_bpf_moudule_valid(BPF_MODULE_CGROUP_PMU) {
            info!("bpf pmu module is invalid");
        } else if let Err(e) = self.update_cgroup_pmu() {
            warn!(
                "[cpucg] update cgroup cpi with bpf error: {}, path= {}",
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

#[cfg(test)]
mod tests_cpu_cg {
    use super::*;
    use crate::cgroup::{new_cpu_cgroup, new_cpuset_cgroup, SubSystem};
    use std::env;
    use std::ops::Deref;
    use std::path::PathBuf;

    #[test]
    fn test_cpuset_cg() {
        let mount_point: String = String::from(format!(
            "{}/tests/sample",
            env::current_dir().unwrap().to_string_lossy()
        ));
        let user_path: PathBuf = PathBuf::from("pod_user_path");

        let mut cg = new_cpuset_cgroup(&mount_point, &user_path, CGroupType::V1 {});
        cg.update();

        let correct_full_path = PathBuf::from(format!("{}/cpuset/pod_user_path", mount_point));

        if let CpuSetCGroup::V1(cpuset_cg) = cg {
            assert_eq!(cpuset_cg.full_path(), correct_full_path);

            let correct_memory_node_vec: Vec<usize> = vec![1, 2];
            assert_eq!(cpuset_cg.mems().meta().clone(), String::from("1-2"));
            assert_eq!(cpuset_cg.mems().deref().clone(), correct_memory_node_vec);

            let correct_cpu_node_vec = vec![
                10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 30, 31, 32, 33, 34, 35, 36, 37, 38, 40,
            ];
            assert_eq!(
                cpuset_cg.cpus().meta().clone(),
                String::from("10-19,30-38,40")
            );
            assert_eq!(cpuset_cg.cpus().deref().clone(), correct_cpu_node_vec);

            // test for cpuset subsystem
            {
                let cpuset_subsystem = SubSystem::CpuSet(CpuSetCGroup::V1(cpuset_cg));
                assert_eq!(
                    cpuset_subsystem.get_full_path(),
                    correct_full_path.as_path()
                );
                assert_eq!(cpuset_subsystem.sub_system_path_exists(), true);
                assert_eq!(cpuset_subsystem.is_net_cgroup(), false);
                assert_eq!(cpuset_subsystem.is_cpuset_cgroup(), true);
            }
        }
    }

    #[test]
    fn test_cpuset_cg_v2() {
        let mount_point: String = env::current_dir()
            .unwrap()
            .into_os_string()
            .into_string()
            .unwrap();
        let user_path: PathBuf = PathBuf::from("tests/sample/cgroupv2");

        let mut cg = new_cpuset_cgroup(&mount_point, &user_path, CGroupType::V2 {});
        cg.update();

        let correct_full_path = PathBuf::from(format!("{}/tests/sample/cgroupv2", mount_point));

        if let CpuSetCGroup::V2(cpuset_cg) = cg {
            assert_eq!(cpuset_cg.full_path(), correct_full_path);

            let correct_memory_node_vec: Vec<usize> = vec![1, 2];
            assert_eq!(cpuset_cg.mems().meta().clone(), String::from("1-2"));
            assert_eq!(cpuset_cg.mems().deref().clone(), correct_memory_node_vec);

            let correct_cpu_node_vec = vec![
                10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 30, 31, 32, 33, 34, 35, 36, 37, 38, 40,
            ];
            assert_eq!(
                cpuset_cg.cpus().meta().clone(),
                String::from("10-19,30-38,40")
            );
            assert_eq!(cpuset_cg.cpus().deref().clone(), correct_cpu_node_vec);

            // test for cpuset subsystem
            {
                let cpuset_subsystem = SubSystem::CpuSet(CpuSetCGroup::V2(cpuset_cg));
                assert_eq!(
                    cpuset_subsystem.get_full_path(),
                    correct_full_path.as_path()
                );
                assert_eq!(cpuset_subsystem.sub_system_path_exists(), true);
                assert_eq!(cpuset_subsystem.is_net_cgroup(), false);
                assert_eq!(cpuset_subsystem.is_cpuset_cgroup(), true);
            }
        }
    }

    #[test]
    fn test_cpu_cg() {
        let mount_point: String = String::from(format!(
            "{}/tests/sample",
            env::current_dir().unwrap().to_string_lossy()
        ));
        let user_path: PathBuf = PathBuf::from("pod_user_path");

        let mut cg = new_cpu_cgroup(&mount_point, &user_path, CGroupType::V1 {});
        cg.update();

        let correct_full_path = PathBuf::from(format!("{}/cpuacct/pod_user_path", mount_point));

        if let CpuCGroup::V1(mut cpu_cg) = cg {
            assert_eq!(cpu_cg.full_path(), correct_full_path);
            assert_eq!(cpu_cg.cfs_period_us, 100000 as u64);
            assert_eq!(cpu_cg.cfs_quota_us, 2000000 as i64);
            assert_eq!(cpu_cg.cpu_shares, 19456 as u64);

            assert_eq!(
                cpu_cg.new_cpu_basic_info.cpu_user_time,
                145405339206537 as u64
            );
            assert_eq!(cpu_cg.new_cpu_basic_info.cpu_sys_time, 1 as u64);
            assert_eq!(cpu_cg.new_cpu_basic_info.cpu_usage, 145405357644162 as u64);

            assert_eq!(cpu_cg.cpu_nr_periods, Some(422162 as u64));
            assert_eq!(cpu_cg.cpu_nr_throttled, Some(2 as u64));
            assert_eq!(cpu_cg.cpu_throttled_time, Some(10 as u64));

            cpu_cg.update();
            assert_eq!(
                cpu_cg.new_cpu_basic_info.cpu_user_time,
                cpu_cg.old_cpu_basic_info.cpu_user_time
            );
            assert_eq!(
                cpu_cg.new_cpu_basic_info.cpu_sys_time,
                cpu_cg.old_cpu_basic_info.cpu_sys_time
            );
            assert_eq!(
                cpu_cg.new_cpu_basic_info.cpu_usage,
                cpu_cg.old_cpu_basic_info.cpu_usage
            );

            // test for cpu subsystem
            {
                let cpu_subsystem = SubSystem::Cpu(CpuCGroup::V1(cpu_cg));
                assert_eq!(cpu_subsystem.get_full_path(), correct_full_path.as_path());
                assert_eq!(cpu_subsystem.sub_system_path_exists(), true);
                assert_eq!(cpu_subsystem.is_blkio_cgroup(), false);
                assert_eq!(cpu_subsystem.is_cpu_cgroup(), true);
            }
        }
    }

    #[test]
    fn test_cpu_cg_v2() {
        let mount_point: String = env::current_dir()
            .unwrap()
            .into_os_string()
            .into_string()
            .unwrap();
        let user_path: PathBuf = PathBuf::from("tests/sample/cgroupv2");

        let mut cg = new_cpu_cgroup(&mount_point, &user_path, CGroupType::V2 {});
        cg.update();

        let correct_full_path = PathBuf::from(format!("{}/tests/sample/cgroupv2", mount_point));

        if let CpuCGroup::V2(cpu_cg) = cg {
            assert_eq!(cpu_cg.full_path(), correct_full_path.as_path());
            assert_eq!(cpu_cg.weight, Some(100 as u64));
            assert_eq!(cpu_cg.max_burst, Some(100 as u64));

            // cpu.stat
            assert_eq!(cpu_cg.cpu_stats.usage_usec, 6983392776);
            assert_eq!(cpu_cg.cpu_stats.system_usec, 2777685920);

            // cpu.cpu_pressure
            assert_eq!(cpu_cg.cpu_pressure.get_some().unwrap().get_avg10(), 1.23);
            assert_eq!(
                cpu_cg.cpu_pressure.get_full().unwrap().get_total(),
                428423424.0
            );

            // test for cpu subsystem
            {
                let cpu_subsystem = SubSystem::Cpu(CpuCGroup::V2(cpu_cg));
                assert_eq!(cpu_subsystem.get_full_path(), correct_full_path.as_path());
                assert_eq!(cpu_subsystem.sub_system_path_exists(), true);
                assert_eq!(cpu_subsystem.is_blkio_cgroup(), false);
                assert_eq!(cpu_subsystem.is_cpu_cgroup(), true);
            }
        }
    }
}
