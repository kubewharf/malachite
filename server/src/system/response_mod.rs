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

use lib::cpu::ProcessorCPIData;
use lib::net::{NetInfo, NetworkCardTraffic};
use lib::process::SystemProcessStats;
use lib::psi::PressureStallInfo;
use lib::system::{
    BPFProgStats, DiskStat, DiskUsage, LoadAvg, MemoryInfo, NumaNode, SystemEventData,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct RespComputeCpu {
    pub(crate) name: String,
    pub(crate) cpu_usage: f32,
    pub(crate) cpu_sys_usage: f32,
    pub(crate) cpu_iowait_ratio: f32,
    pub(crate) cpu_sched_wait: f32,
    pub(crate) cpi_data: Option<ProcessorCPIData>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct RespCompute {
    pub(crate) load: LoadAvg,
    pub(crate) cpu: Vec<RespComputeCpu>,
    pub(crate) global_cpu: RespComputeCpu,
    pub(crate) process_stats: SystemProcessStats,
    pub(crate) pressure: Option<PressureStallInfo>,
    pub(crate) bpf_prog_stats: Option<BPFProgStats>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct RespNetwork {
    pub(crate) networkcard: Vec<NetworkCardTraffic>,
    pub(crate) tcp: NetInfo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct RespIo {
    pub(crate) disk_io: Vec<DiskStat>,
    pub(crate) disk_usage: Vec<DiskUsage>,
    pub(crate) pressure: Option<PressureStallInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct RespMemory {
    pub(crate) system: MemoryInfo,
    pub(crate) pressure: Option<PressureStallInfo>,
    pub(crate) numa: Vec<NumaNode>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct RespSystemEvent {
    pub(crate) event: SystemEventData,
}

// struct RespStorage {
//
// }
