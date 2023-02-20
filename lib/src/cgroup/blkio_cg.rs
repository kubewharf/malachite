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
use crate::ffi::bpf::ffi::fs_data;
use crate::ffi::{
    is_bpf_moudule_valid, wrapper_get_cgroup_fs_data, wrapper_get_cgroup_io_latpcts,
    WrapperIoLatpcts, BPF_MODULE_CGROUP_FS, BPF_MODULE_CGROUP_IO,
};
use crate::psi::PressureStallInfo;
use crate::system::get_secs_since_epoch;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};
use std::collections::HashMap;
use std::fmt::{self, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

pub fn new_blkio_cgroup(
    mount_point: &str,
    user_path: &Path,
    cgroup_type: CGroupType,
) -> BlkIOCGroup {
    match cgroup_type {
        CGroupType::V1 => BlkIOCGroup::V1(BlkIOCGroupV1 {
            full_path: PathBuf::from(format!(
                "{}/blkio/{}",
                mount_point,
                user_path.to_string_lossy()
            )),
            user_path: user_path.to_path_buf(),
            iops_details: HashMap::new(),
            bps_details: HashMap::new(),
            bpf_fs_data: Default::default(),
            old_bpf_fs_data: Default::default(),
            ..Default::default()
        }),
        CGroupType::V2 => BlkIOCGroup::V2(BlkIOCGroupV2 {
            full_path: PathBuf::from(format!("{}/{}", mount_point, user_path.to_string_lossy())),
            user_path: user_path.to_path_buf(),
            io_stat: HashMap::new(),
            io_weight: HashMap::new(),
            io_max: HashMap::new(),
            io_latency: HashMap::new(),
            ..Default::default()
        }),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlkIOCGroup {
    V1(BlkIOCGroupV1),
    V2(BlkIOCGroupV2),
}

impl BlkIOCGroup {
    pub fn full_path(&self) -> &Path {
        match self {
            BlkIOCGroup::V1(v1) => v1.full_path(),
            BlkIOCGroup::V2(v2) => v2.full_path(),
        }
    }

    pub fn update(&mut self) {
        match self {
            BlkIOCGroup::V1(v1) => v1.update(),
            BlkIOCGroup::V2(v2) => v2.update(),
        }
    }

    pub fn update_ebpf(&mut self) {
        match self {
            BlkIOCGroup::V1(v1) => v1.update_ebpf(),
            BlkIOCGroup::V2(v2) => v2.update_ebpf(),
        }
    }

    pub fn reset(&mut self) {
        match self {
            BlkIOCGroup::V1(v1) => v1.reset(),
            BlkIOCGroup::V2(v2) => v2.reset(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BlkIOCGroupV2 {
    full_path: PathBuf,
    user_path: PathBuf,
    io_stat: HashMap<String, BlkIOStatV2>,
    io_max: HashMap<String, BlkIOMaxV2>,
    io_pressure: PressureStallInfo,
    io_latency: HashMap<String, u64>,
    io_weight: HashMap<String, u64>,
    pub(crate) bpf_fs_data: fs_data,
    pub(crate) old_bpf_fs_data: fs_data,
    bpf_io_latency: WrapperIoLatpcts,
    update_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BlkIOStatV2 {
    rbytes: u64,
    wbytes: u64,
    rios: u64,
    wios: u64,
    dbytes: u64,
    dios: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BlkIOMaxV2 {
    rbps: u64,
    wbps: u64,
    riops: u64,
    wiops: u64,
}

impl BlkIOCGroupV2 {
    fn full_path(&self) -> &Path {
        self.full_path.as_path()
    }

    fn update_io_stat(&mut self) -> common::Result<bool> {
        let mut file = PathBuf::from(&self.full_path());
        file.push("io.stat");
        let contents = fs::read_to_string(&file)?;
        let io_stats = utils::parse_cgroup_nested_u64(contents.as_str());
        for (device_no, io_stat) in io_stats {
            if device_no.is_empty() {
                continue;
            }

            let stat = BlkIOStatV2::default();
            let r = serde_json::to_string(&stat).unwrap();
            let mut stat_map: Map<String, Value> = serde_json::from_str(&r).unwrap();
            for (key, value) in io_stat {
                stat_map.insert(key, Value::Number(Number::from(value)));
            }

            let s = serde_json::to_string(&stat_map).unwrap();
            let io_stat_v2: BlkIOStatV2 = serde_json::from_str(&s).unwrap();
            self.io_stat.insert(device_no, io_stat_v2);
        }
        Ok(true)
    }

    fn update_io_max(&mut self) -> common::Result<bool> {
        let mut file = PathBuf::from(&self.full_path());
        file.push("io.max");
        let contents = fs::read_to_string(&file)?;
        let data = utils::parse_cgroup_nested_u64(contents.as_str());
        for (device_no, io_max) in data {
            if device_no.is_empty() {
                continue;
            }

            let stat = BlkIOMaxV2::default();
            let r = serde_json::to_string(&stat).unwrap();
            let mut stat_map: Map<String, Value> = serde_json::from_str(&r).unwrap();
            for (key, value) in io_max {
                stat_map.insert(key, Value::Number(Number::from(value)));
            }

            let s = serde_json::to_string(&stat_map).unwrap();
            let io_max_v2: BlkIOMaxV2 = serde_json::from_str(&s).unwrap();
            self.io_max.insert(device_no, io_max_v2);
        }

        Ok(true)
    }

    fn update_io_pressure(&mut self) -> common::Result<bool> {
        let mut file = PathBuf::from(&self.full_path());
        file.push("io.pressure");
        let contents = fs::read_to_string(&file)?;
        self.io_pressure = PressureStallInfo::from(contents.as_str());
        Ok(true)
    }

    fn update_io_weight(&mut self) -> common::Result<bool> {
        let mut file = PathBuf::from(&self.full_path());
        file.push("io.weight");
        let contents = fs::read_to_string(&file)?;
        let data = utils::parse_cgroup_file(contents.as_str());
        let mut stats = HashMap::new();
        for (key, value) in &data {
            if key.is_empty() {
                continue;
            }
            stats.insert(String::from(*key), *value);
        }
        self.io_weight = stats;
        Ok(true)
    }

    fn update_io_latency(&mut self) -> common::Result<bool> {
        let mut file = PathBuf::from(&self.full_path());
        file.push("io.latency");
        let contents = fs::read_to_string(&file)?;
        let data = utils::parse_cgroup_nested_u64(contents.as_str());
        for (device_no, latency) in &data {
            if device_no.is_empty() {
                continue;
            }

            for value in latency.values() {
                self.io_latency.insert((*device_no).clone(), *value);
            }
        }
        Ok(true)
    }

    #[cfg(not(tarpaulin_include))]
    pub fn update_bpf_data(&mut self) -> common::Result<bool> {
        let user_path = PathBuf::from(&self.user_path);
        self.old_bpf_fs_data = self.bpf_fs_data;
        self.bpf_fs_data = wrapper_get_cgroup_fs_data(user_path);
        Ok(true)
    }

    #[cfg(not(tarpaulin_include))]
    pub fn update_bpf_io_latency(&mut self) -> common::Result<bool> {
        let user_path = PathBuf::from(&self.user_path);
        self.bpf_io_latency = wrapper_get_cgroup_io_latpcts(user_path, 99);
        Ok(true)
    }

    fn update(&mut self) {
        if let Err(e) = self.update_io_stat() {
            warn!(
                "[blkiocg] update io stat error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }

        if let Err(e) = self.update_io_max() {
            warn!(
                "[blkiocg] update io max error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }

        if let Err(e) = self.update_io_pressure() {
            warn!(
                "[blkiocg] update io pressure error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }

        if let Err(e) = self.update_io_weight() {
            warn!(
                "[blkiocg] update io weight error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }

        if let Err(e) = self.update_io_latency() {
            warn!(
                "[blkiocg] update io latency error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }
        self.update_time = get_secs_since_epoch();
    }

    fn update_ebpf(&mut self) {
        if !is_bpf_moudule_valid(BPF_MODULE_CGROUP_FS) {
            info!("bpf fs module is invalid");
        } else if let Err(e) = self.update_bpf_data() {
            warn!(
                "[blkiocg] update bpf data error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }

        if !is_bpf_moudule_valid(BPF_MODULE_CGROUP_IO) {
            info!("bpf io module is invalid");
        } else if let Err(e) = self.update_bpf_io_latency() {
            warn!(
                "[blkiocg] update bpf io latency error: {}, path= {}",
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
        };
        self.update_time = get_secs_since_epoch();
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub enum BlkOperationType {
    Read,
    Write,
    Sync,
    Async,
    Total,
    Unknown,
}

impl BlkOperationType {
    pub fn as_str(&self) -> &str {
        match *self {
            BlkOperationType::Read => "Read",
            BlkOperationType::Write => "Write",
            BlkOperationType::Async => "Async",
            BlkOperationType::Sync => "Sync",
            BlkOperationType::Total => "Total",
            BlkOperationType::Unknown => "Unknown",
        }
    }
}

impl From<&str> for BlkOperationType {
    fn from(data: &str) -> Self {
        match data {
            "Read" => BlkOperationType::Read,
            "Write" => BlkOperationType::Write,
            "Async" => BlkOperationType::Async,
            "Sync" => BlkOperationType::Sync,
            "Total" => BlkOperationType::Total,
            _ => BlkOperationType::Unknown,
        }
    }
}

impl fmt::Display for BlkOperationType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BlkIOCGroupV1 {
    full_path: PathBuf,
    user_path: PathBuf,
    pub(crate) iops_details: HashMap<String, HashMap<BlkOperationType, u64>>,
    pub(crate) bps_details: HashMap<String, HashMap<BlkOperationType, u64>>,
    pub(crate) iops_total: u64,
    pub(crate) bps_total: u64,
    pub(crate) bpf_fs_data: fs_data,
    pub(crate) old_bpf_fs_data: fs_data,
    update_time: u64,
}

impl BlkIOCGroupV1 {
    pub fn iops_details(&self) -> &HashMap<String, HashMap<BlkOperationType, u64>> {
        &self.iops_details
    }

    pub fn bps_details(&self) -> &HashMap<String, HashMap<BlkOperationType, u64>> {
        &self.bps_details
    }

    pub fn iops_total(&self) -> u64 {
        self.iops_total
    }

    pub fn bps_total(&self) -> u64 {
        self.bps_total
    }

    pub fn full_path(&self) -> &Path {
        self.full_path.as_path()
    }
    pub fn user_path(&self) -> &Path {
        self.user_path.as_path()
    }

    #[cfg(not(tarpaulin_include))]
    pub fn update(&mut self) {
        if let Err(e) = self.update_bps() {
            warn!(
                "[blkio_cg] update bps error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }
        if let Err(e) = self.update_iops() {
            warn!(
                "[blkio_cg] update iops error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }
        self.update_time = get_secs_since_epoch();
    }

    pub fn reset(&mut self) {
        *self = Self {
            full_path: self.full_path.clone(),
            user_path: self.user_path.clone(),
            ..Default::default()
        };
        self.update_time = get_secs_since_epoch();
    }

    #[cfg(not(tarpaulin_include))]
    pub fn update_bpf_data(&mut self) -> common::Result<bool> {
        let user_path = PathBuf::from(&self.user_path);
        self.old_bpf_fs_data = self.bpf_fs_data;
        self.bpf_fs_data = wrapper_get_cgroup_fs_data(user_path);
        Ok(true)
    }

    pub fn update_iops(&mut self) -> common::Result<bool> {
        let mut file = PathBuf::from(&self.full_path);
        file.push("blkio.throttle.io_serviced");
        let contents = fs::read_to_string(&file).unwrap();
        let (total, details) = parse_blkio_file(contents.as_str());
        self.iops_total = total;
        self.iops_details = details;
        Ok(true)
    }

    pub fn update_bps(&mut self) -> common::Result<bool> {
        let mut file = PathBuf::from(&self.full_path);
        file.push("blkio.throttle.io_service_bytes");
        let contents = fs::read_to_string(&file).unwrap();
        let (total, details) = parse_blkio_file(contents.as_str());
        self.bps_total = total;
        self.bps_details = details;
        Ok(true)
    }

    pub fn update_ebpf(&mut self) {
        if !is_bpf_moudule_valid(BPF_MODULE_CGROUP_FS) {
            info!("bpf fs module is invalid");
            return;
        }

        if let Err(e) = self.update_bpf_data() {
            warn!(
                "[blkio_cg] update bpf error: {}, path= {}",
                e,
                self.full_path.display()
            );
        }
    }
}

pub fn parse_blkio_file(contents: &str) -> (u64, HashMap<String, HashMap<BlkOperationType, u64>>) {
    // parse file like:
    // 259:1 Read 930328576
    // 259:1 Write 1246900224
    // 259:1 Sync 1574629376
    // ....
    let mut result: HashMap<String, HashMap<BlkOperationType, u64>> = HashMap::new();
    let mut total: u64 = 0;
    contents.split('\n').for_each(|line| {
        if !line.is_empty() {
            let mut iter = line.split_whitespace();
            let count = iter.clone().count();
            match count {
                2 => total = iter.next_back().unwrap().parse::<u64>().unwrap(),
                3 => {
                    let tmp = result
                        .entry(String::from(iter.next().unwrap()))
                        .or_insert_with(HashMap::new);
                    tmp.insert(
                        BlkOperationType::from(iter.next().unwrap()),
                        iter.next().unwrap().parse::<u64>().unwrap(),
                    );
                }
                _ => {}
            }
        }
    });
    (total, result)
}
