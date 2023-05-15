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
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub(crate) struct RespComputeCpu {
    /// global: all , other: id
    pub(crate) name: String,
    /// cpu total usage
    pub(crate) cpu_usage: f32,
    /// cpu usage in kernel
    pub(crate) cpu_sys_usage: f32,
    /// cpu iowait ratio
    pub(crate) cpu_iowait_ratio: f32,
    /// schedule wait time in seconds
    pub(crate) cpu_sched_wait: f32,
    /// CPI info
    pub(crate) cpi_data: Option<ProcessorCPIData>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub(crate) struct RespCompute {
    /// system load
    pub(crate) load: LoadAvg,
    /// percpu info
    pub(crate) cpu: Vec<RespComputeCpu>,
    /// global cpu info
    pub(crate) global_cpu: RespComputeCpu,
    /// process stats, include running/blocking
    pub(crate) process_stats: SystemProcessStats,
    /// cpu PSI
    pub(crate) pressure: Option<PressureStallInfo>,
    /// ebpf prog stats
    pub(crate) bpf_prog_stats: Option<BPFProgStats>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub(crate) struct RespNetwork {
    /// traffic info per net card
    pub(crate) networkcard: Vec<NetworkCardTraffic>,
    /// tcp info
    pub(crate) tcp: NetInfo,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub(crate) struct RespIo {
    /// disk io info per device
    pub(crate) disk_io: Vec<DiskStat>,
    /// disk usage per mount point
    pub(crate) disk_usage: Vec<DiskUsage>,
    /// io PSI
    pub(crate) pressure: Option<PressureStallInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub(crate) struct RespMemory {
    /// global memory info
    pub(crate) system: MemoryInfo,
    /// memory PSI
    pub(crate) pressure: Option<PressureStallInfo>,
    /// per numa memory info
    pub(crate) numa: Vec<NumaNode>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub(crate) struct RespSystemEvent {
    pub(crate) event: SystemEventData,
}

// struct RespStorage {
//
// }
