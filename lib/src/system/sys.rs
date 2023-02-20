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

use crate::cgroup::{CGroup, CGroupUserPath, PerfEventCGroup, SubSystemType};
use crate::common;
use crate::cpu::SystemProcessorInfo;
use crate::ffi;
use crate::ffi::bpf::ffi as bpf_ffi;
use crate::ffi::pmu::ffi::{
    byteperf_cgroup_buffer_malachite, byteperf_cpu_buffer_malachite, byteperf_imc_buffer_malachite,
};
use crate::net::{NetInfo, Traffic};
use crate::process::SystemProcessStats;
use crate::settings;
use crate::system::diskstat::Disk;
use crate::system::get_secs_since_epoch;
use crate::system::load::LoadAvg;
use crate::system::memory::MemoryInfo;
use crate::system::numa_node::{ImcChannelInfo, SystemDeviceNode};
use crate::system::SystemPSI;
use lazy_static::*;
use libc::{sysconf, _SC_PAGESIZE};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::ffi::CStr;
use std::path::Path;
use std::str;
use std::{collections::HashMap, path::PathBuf};
use strum::IntoEnumIterator;

lazy_static! {
    pub static ref PAGE_SIZE_KB: u64 = unsafe { sysconf(_SC_PAGESIZE) as u64 / 1024 };
}

pub type ProcessorId = usize;

#[derive(Default, Clone, Deserialize, Serialize, Debug)]
pub struct SystemEventData {
    event_data: bpf_ffi::system_event_data,
    update_time: u64,
}
impl SystemEventData {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[derive(Default, Clone, Deserialize, Serialize, Debug)]
pub struct BPFProgStats {
    stats: Vec<bpf_ffi::bpf_program_stats>,
    update_time: u64,
}
impl BPFProgStats {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[derive(Clone)]
pub struct System {
    processors: SystemProcessorInfo,
    process_stats: SystemProcessStats,
    cgroup_list: HashMap<CGroupUserPath, CGroup>,
    memory: MemoryInfo,
    load: LoadAvg,
    net_traffic: Traffic,
    net_info: NetInfo,
    system_devices_node: SystemDeviceNode,
    disk_io: Disk,
    system_pressure: Option<SystemPSI>,
    system_events: SystemEventData,
    bpf_stats: BPFProgStats,
}

impl System {
    pub fn new() -> System {
        System {
            processors: SystemProcessorInfo::default(),
            process_stats: Default::default(),
            cgroup_list: Default::default(),
            memory: MemoryInfo::default(),
            load: LoadAvg::default(),
            net_traffic: Traffic::default(),
            net_info: NetInfo::default(),
            system_devices_node: SystemDeviceNode::default(),
            disk_io: Disk::default(),
            system_pressure: None,
            system_events: SystemEventData::default(),
            bpf_stats: BPFProgStats::default(),
        }
    }

    pub fn get_processors(&self) -> &SystemProcessorInfo {
        &self.processors
    }
    pub fn get_system_device_nodes(&self) -> &SystemDeviceNode {
        &self.system_devices_node
    }
    pub fn get_memory_info(&self) -> &MemoryInfo {
        &self.memory
    }
    pub fn get_load(&self) -> &LoadAvg {
        &self.load
    }
    pub fn get_net_info(&self) -> &NetInfo {
        &self.net_info
    }
    pub fn get_net_traffic(&self) -> &Traffic {
        &self.net_traffic
    }
    pub fn get_disk_io(&self) -> &Disk {
        &self.disk_io
    }
    pub fn get_self(&self) -> &System {
        self
    }
    pub fn get_cgroups(&self, path: PathBuf) -> Option<&CGroup> {
        self.cgroup_list.get(&path)
    }
    pub fn get_process_stats(&self) -> &SystemProcessStats {
        &self.process_stats
    }
    pub fn get_system_pressure(&self) -> &Option<SystemPSI> {
        &self.system_pressure
    }
    pub fn get_system_event(&self) -> &SystemEventData {
        &self.system_events
    }
    pub fn get_bpf_prog_stats(&self) -> &BPFProgStats {
        &self.bpf_stats
    }
    pub fn insert_cgroups_item(&mut self, item: CGroup) -> common::Result<bool> {
        self.cgroup_list
            .entry(PathBuf::from(item.user_path()))
            .or_insert(item);
        Ok(true)
    }
    fn refresh_cgroup_pmu_data(
        &mut self,
        cgroup_num: usize,
        cg_perf_data: [byteperf_cgroup_buffer_malachite; 256usize],
    ) {
        debug!("[byteperf] cgroup_num: {}", cgroup_num);
        let mut perf_cgs: Vec<PerfEventCGroup> = Vec::new();
        for i in 0..cgroup_num {
            let perf_event_cg = PerfEventCGroup::from(cg_perf_data.get(i).unwrap());
            perf_cgs.push(perf_event_cg);
        }
        info!("[byteperf] perf cgs: {:?}", perf_cgs);
        let perf_event_hash: HashMap<PathBuf, PerfEventCGroup> = perf_cgs
            .iter()
            .map(|item| (PathBuf::from(item.user_path()), item.clone()))
            .collect();
        for (path, cgroup) in self.cgroup_list.iter_mut() {
            if let Some(perf_event_cg) = perf_event_hash.get(path) {
                if let Some(instance) = cgroup.sub_system_groups.get_mut(&SubSystemType::PerfEvent)
                {
                    instance
                        .update_perf_event(perf_event_cg)
                        .expect("[byteperf] update perf event error");
                }
            }
        }
    }
    fn reset_cgroup_pmu_data(&mut self) {
        info!("reset_cgroup_pmu_data");
        let perf_event_cg = PerfEventCGroup::default();
        for (_, cgroup) in self.cgroup_list.iter_mut() {
            if let Some(instance) = cgroup.sub_system_groups.get_mut(&SubSystemType::PerfEvent) {
                instance
                    .update_perf_event(&perf_event_cg)
                    .expect("[byteperf] update perf event error");
            }
        }
    }
    fn refresh_cpu_cpi_data(
        &mut self,
        cpu_nums: usize,
        cpu_cpi_data: [byteperf_cpu_buffer_malachite; 256usize],
    ) {
        info!("[byteperf] update cpu cpi data, cpu_nums: {}", cpu_nums);
        if cpu_nums > 256 {
            warn!("[byteperf] cpu nums > 256, skip cpi update");
            return;
        }
        let data: HashMap<String, byteperf_cpu_buffer_malachite> = cpu_cpi_data[..cpu_nums]
            .iter()
            .map(|item| (format!("cpu{}", item.cpu), *item))
            .collect();
        self.processors.iter_mut().for_each(|(name, processor)| {
            if let Some(perf_cpu_cpi_entry) = data.get(name) {
                processor.update_cpu_cpi(
                    perf_cpu_cpi_entry.cpi,
                    perf_cpu_cpi_entry.instructions,
                    perf_cpu_cpi_entry.cycles,
                    perf_cpu_cpi_entry.l3_misses,
                    perf_cpu_cpi_entry.utilization,
                )
            }
        });
    }
    fn refresh_numa_pmu_data(
        &mut self,
        imcs_num: usize,
        numa_perf_data: [byteperf_imc_buffer_malachite; 24usize],
    ) {
        info!("[byteperf] imcs_num: {}", imcs_num);
        struct NumaDataTrans {
            memory_read_bandwidth: f64,
            memory_write_bandwidth: f64,
            memory_idle_bandwidth: f64,
            memory_read_latency: f64,
            memory_write_latency: f64,
            channels: Vec<ImcChannelInfo>,
        }

        let mut perf_numa_data: HashMap<usize, NumaDataTrans> = HashMap::new();

        for i in 0..imcs_num {
            let imc_data = numa_perf_data.get(i).unwrap();
            let mut entry =
                perf_numa_data
                    .entry(imc_data.numa_id as usize)
                    .or_insert(NumaDataTrans {
                        memory_read_bandwidth: 0.0,
                        memory_write_bandwidth: 0.0,
                        memory_idle_bandwidth: 0.0,
                        memory_read_latency: 0.0,
                        memory_write_latency: 0.0,
                        channels: Vec::new(),
                    });

            entry.memory_idle_bandwidth += imc_data.memory_idle_bandwidth;
            entry.memory_write_bandwidth += imc_data.memory_write_bandwidth;
            entry.memory_read_bandwidth += imc_data.memory_read_bandwidth;
            entry.memory_read_latency += imc_data.memory_read_latency;
            entry.memory_write_latency += imc_data.memory_write_latency;

            let channel_name: String = unsafe {
                CStr::from_ptr(imc_data.name.as_ptr())
                    .to_string_lossy()
                    .into_owned()
            };
            let channel = ImcChannelInfo::new(
                channel_name,
                imc_data.memory_read_bandwidth,
                imc_data.memory_write_bandwidth,
                imc_data.memory_idle_bandwidth,
                imc_data.memory_bandwidth_util,
                imc_data.memory_read_latency,
                imc_data.memory_write_latency,
            );
            entry.channels.push(channel);
        }

        self.system_devices_node
            .nodes
            .iter_mut()
            .for_each(|numa_node| {
                let numa_node_id = numa_node.get_id();
                if let Some(perf_numa_entry) = perf_numa_data.get(&numa_node_id) {
                    numa_node.refresh_numa_mem(
                        perf_numa_entry.memory_read_bandwidth,
                        perf_numa_entry.memory_write_bandwidth,
                        perf_numa_entry.memory_idle_bandwidth,
                        perf_numa_entry.memory_read_latency,
                        perf_numa_entry.memory_write_latency,
                        perf_numa_entry.channels.clone(),
                    );
                }
            });
    }
    fn reset_cpu_cpi_data(&mut self) {
        info!("reset cpu cpi data");
        self.processors
            .iter_mut()
            .for_each(|(_, processor)| processor.reset_cpu_cpi())
    }
    fn reset_numa_pmu_data(&mut self) {
        info!("reset_numa_pmu_data");
        self.system_devices_node
            .nodes
            .iter_mut()
            .for_each(|numa_node| {
                numa_node.reset_numa_mem_latency();
                numa_node.reset_numa_mem_bandwidth();
            });
    }

    // at least run 1s
    pub fn refresh_pmu(&mut self, ds_settings: &dyn settings::DataSource) {
        if !ds_settings.is_enable() {
            info!("data source byteperf is disabled");
            return;
        }

        let (_, cur_mask) = ffi::wrapper_byteperf_check_module_health();
        if cur_mask == 0 {
            return;
        }

        let perf_data = ffi::wrapper_byteperf_gather_count_malachite();
        if let Some(sub_sys) = ds_settings.get_sub_system(settings::SubSystemType::Cpu) {
            if sub_sys.is_enable() {
                self.refresh_cpu_cpi_data(perf_data.cpu_num as usize, perf_data.cpus);
            }
        }

        if let Some(sub_sys) = ds_settings.get_sub_system(settings::SubSystemType::Mem) {
            if sub_sys.is_enable() {
                self.refresh_numa_pmu_data(perf_data.imc_num as usize, perf_data.imcs);
            }
        }
    }

    pub fn refresh_cgroups(&mut self, ds_settings: &dyn settings::DataSource) {
        // delete invalid cgroup item
        self.cgroup_list
            .retain(|_, cgroup| !cgroup.sub_system_groups.is_empty());

        if !ds_settings.is_enable() {
            info!("data source cgroup is disabled");
            return;
        }

        // update valid cgroup info
        if !self.cgroup_list.is_empty() {
            info!(
                "[refresh cgroups] cgroup list = {:?}",
                self.cgroup_list.keys()
            );
            for (cgroup_path, cgroup) in self.cgroup_list.iter_mut() {
                match cgroup.update(ds_settings) {
                    Ok(_) => {}
                    Err(e) => warn!(
                        "update cgroup error, cgroup.path={}, e={}",
                        cgroup_path.display(),
                        e
                    ),
                }
            }
        }
    }

    pub fn refresh_ebpf(&mut self, ds_settings: &dyn settings::DataSource) {
        // delete invalid cgroup item
        self.cgroup_list
            .retain(|_, cgroup| !cgroup.sub_system_groups.is_empty());

        if !ds_settings.is_enable() {
            info!("data source ebpf is disabled");
            return;
        }

        // refresh system event
        self.refresh_system_event(ds_settings);

        //
        self.refresh_bpf_prog_stats();

        // update valid cgroup info
        if !self.cgroup_list.is_empty() {
            info!(
                "[refresh cgroups] cgroup list = {:?}",
                self.cgroup_list.keys()
            );
            for (cgroup_path, cgroup) in self.cgroup_list.iter_mut() {
                match cgroup.update_ebpf(ds_settings) {
                    Ok(_) => {}
                    Err(e) => warn!(
                        "update ebpf error, cgroup.path={}, e={}",
                        cgroup_path.display(),
                        e
                    ),
                }
            }
        }
    }

    fn refresh_system_event(&mut self, ds_settings: &dyn settings::DataSource) {
        if !ds_settings.is_enable() {
            return;
        }

        self.system_events.event_data = ffi::wrapper_get_system_event_count();
        self.system_events.update_time = get_secs_since_epoch();
    }

    fn refresh_bpf_prog_stats(&mut self) {
        self.bpf_stats.stats = ffi::wrapper_get_bpf_prog_stats();
        self.bpf_stats.update_time = get_secs_since_epoch();
    }

    fn refresh_psi(&mut self) {
        if self.system_pressure.is_none() {
            let dir = Path::new("/proc/pressure");
            if !dir.exists() {
                warn!("[PSI] /proc/pressure not exists, skip");
                return;
            }
            if !dir.is_dir() {
                warn!("[PSI] /proc/pressure is not dir, skip");
                return;
            }
            self.system_pressure = Some(SystemPSI::new());
        }

        if let Some(ref mut x) = self.system_pressure {
            x.update()
        }
    }

    pub fn refresh_proc_cpu(&mut self, ds_settings: &dyn settings::DataSource) {
        if let Some(sub_sys) = ds_settings.get_sub_system(settings::SubSystemType::Cpu) {
            if !sub_sys.is_enable() {
                info!("proc {:?} is disabled", settings::SubSystemType::Cpu);
                return;
            }

            self.load.refresh_system_load();
            self.processors.refresh();
            self.process_stats.refresh();
        } else {
            warn!(
                "get subsys {:?} from data source proc failed",
                settings::SubSystemType::Cpu
            );
        }
    }

    pub fn refresh_proc_mem(&mut self, ds_settings: &dyn settings::DataSource) {
        if let Some(sub_sys) = ds_settings.get_sub_system(settings::SubSystemType::Mem) {
            if !sub_sys.is_enable() {
                info!("proc {:?} is disabled", settings::SubSystemType::Mem);
                return;
            }

            self.memory.refresh();
        } else {
            warn!(
                "get subsys {:?} from data source proc failed",
                settings::SubSystemType::Mem
            );
        }
    }

    pub fn refresh_proc_net(&mut self, ds_settings: &dyn settings::DataSource) {
        if let Some(sub_sys) = ds_settings.get_sub_system(settings::SubSystemType::Net) {
            if !sub_sys.is_enable() {
                info!("proc {:?} is disabled", settings::SubSystemType::Net);
                return;
            }

            self.net_traffic.refresh();
            self.net_info.refresh();
        } else {
            warn!(
                "get subsys {:?} from data source proc failed",
                settings::SubSystemType::Net
            );
        }
    }

    pub fn refresh_proc_disk(&mut self, ds_settings: &dyn settings::DataSource) {
        if let Some(sub_sys) = ds_settings.get_sub_system(settings::SubSystemType::Storage) {
            if !sub_sys.is_enable() {
                info!("proc {:?} is disabled", settings::SubSystemType::Storage);
                return;
            }

            self.disk_io.refresh();
        } else {
            warn!(
                "get subsys {:?} from data source proc failed",
                settings::SubSystemType::Storage
            );
        }
    }

    pub fn refresh_proc(&mut self, ds_settings: &dyn settings::DataSource) {
        if !ds_settings.is_enable() {
            info!("data source proc is disabled");
            return;
        }

        self.refresh_proc_cpu(ds_settings);
        self.refresh_proc_mem(ds_settings);
        self.refresh_proc_net(ds_settings);
        self.refresh_proc_disk(ds_settings);

        self.refresh_psi(); // TODO: split cpu/men/io
    }

    pub fn refresh_sys(&mut self, ds_settings: &dyn settings::DataSource) {
        if !ds_settings.is_enable() {
            info!("data source sys is disabled");
            return;
        }

        self.system_devices_node.refresh_basic_info();
        self.system_devices_node
            .refresh_numa_avaiable_mem(self.memory.vm_watermark_scale_factor());
    }

    pub fn refresh(&mut self, s: &settings::Settings) {
        if !s.is_enable() {
            warn!("all data source is disabled");
            return;
        }

        if let Some(ds_settings) = s.get_data_source(settings::DataSourceType::ProcFS) {
            self.refresh_proc(&*ds_settings);
        } else {
            warn!(
                "get data source {:?} from settings failed",
                settings::DataSourceType::ProcFS
            );
        }

        if let Some(ds_settings) = s.get_data_source(settings::DataSourceType::SysFS) {
            self.refresh_sys(&*ds_settings);
        } else {
            warn!(
                "get data source {:?} from settings failed",
                settings::DataSourceType::SysFS
            );
        }

        if let Some(ds_settings) = s.get_data_source(settings::DataSourceType::CgroupFS) {
            self.refresh_cgroups(&*ds_settings);
        } else {
            warn!(
                "get data source {:?} from settings failed",
                settings::DataSourceType::CgroupFS
            );
        }

        if let Some(ds_settings) = s.get_data_source(settings::DataSourceType::Ebpf) {
            self.refresh_ebpf(&*ds_settings);
        } else {
            warn!(
                "get data source {:?} from settings failed",
                settings::DataSourceType::Ebpf
            );
        }

        if let Some(ds_settings) = s.get_data_source(settings::DataSourceType::BytePerf) {
            self.refresh_pmu(&*ds_settings);
        } else {
            warn!(
                "get data source {:?} from settings failed",
                settings::DataSourceType::BytePerf
            );
        }
    }

    fn turn_off_all(&mut self) {
        let ds = settings::get_disabeld_data_source();
        self.handle_proc_knob(&*ds);
        self.handle_sys_knob(&*ds);
        self.handle_cgroup_knob(&*ds);
        self.handle_ebpf_knob(&*ds);
        self.handle_byteperf_knob(&*ds);
    }

    pub fn switch(&mut self, s: &settings::Settings) {
        if !s.is_enable() {
            self.turn_off_all();
            return;
        }

        if let Some(ds_settings) = s.get_data_source(settings::DataSourceType::ProcFS) {
            self.handle_proc_knob(&*ds_settings);
        } else {
            warn!(
                "get data source {:?} from settings failed",
                settings::DataSourceType::ProcFS
            );
        }

        if let Some(ds_settings) = s.get_data_source(settings::DataSourceType::SysFS) {
            self.handle_sys_knob(&*ds_settings);
        } else {
            warn!(
                "get data source {:?} from settings failed",
                settings::DataSourceType::SysFS
            );
        }

        if let Some(ds_settings) = s.get_data_source(settings::DataSourceType::CgroupFS) {
            self.handle_cgroup_knob(&*ds_settings);
        } else {
            warn!(
                "get data source {:?} from settings failed",
                settings::DataSourceType::CgroupFS
            );
        }

        if let Some(ds_settings) = s.get_data_source(settings::DataSourceType::Ebpf) {
            self.handle_ebpf_knob(&*ds_settings);
        } else {
            warn!(
                "get data source {:?} from settings failed",
                settings::DataSourceType::Ebpf
            );
        }

        if let Some(ds_settings) = s.get_data_source(settings::DataSourceType::BytePerf) {
            self.handle_byteperf_knob(&*ds_settings);
        } else {
            warn!(
                "get data source {:?} from settings failed",
                settings::DataSourceType::BytePerf
            );
        }
    }

    pub fn handle_byteperf_knob(&mut self, ds_settings: &dyn settings::DataSource) {
        if !ds_settings.is_enable() {
            self.turn_off_byteperf();
            return;
        }

        let mut module_vec: Vec<String> = Vec::new();
        if let Some(sub_sys) = ds_settings.get_sub_system(settings::SubSystemType::Cpu) {
            if !sub_sys.is_enable() {
                info!("reset_cpu_cpi_data");
                self.reset_cpu_cpi_data();
            } else {
                module_vec.push("BYTEPERF_CPU_MODULE".to_string());
            }
        }

        if let Some(sub_sys) = ds_settings.get_sub_system(settings::SubSystemType::Mem) {
            if !sub_sys.is_enable() {
                info!("reset_numa_pmu_data");
                self.reset_numa_pmu_data();
            } else {
                module_vec.push("BYTEPERF_MEMORY_MODULE".to_string());
                module_vec.push("BYTEPERF_AMD_DF_MODULE".to_string());
                module_vec.push("BYTEPERF_MCU_MODULE".to_string());
            }
        }

        match ffi::enable_byteperf() {
            Ok(()) => info!("enable_byteperf success"),
            Err(error) => warn!("enable_byteperf failed: {}", error),
        };

        let pmu_idx_vec = ffi::PERF_MODULE_MASK_CONFIG.get_idx_vec(module_vec);
        let pmu_module_mask = ffi::ModuleMask::from(pmu_idx_vec);
        info!(
            "wrapper_byteperf_module_control with mask: {:?}, {:#x}",
            pmu_module_mask.get_mask(),
            pmu_module_mask.get_mask()
        );
        ffi::wrapper_byteperf_module_control(pmu_module_mask);
    }

    pub fn handle_cgroup_knob(&mut self, ds_settings: &dyn settings::DataSource) {
        if !ds_settings.is_enable() {
            self.turn_off_cgroup(ds_settings);
            return;
        }

        for sub_sys in settings::SubSystemType::iter() {
            if let Some(sub_sys) = ds_settings.get_sub_system(sub_sys) {
                if !sub_sys.is_enable() {
                    self.turn_off_cgroup(ds_settings);
                    break;
                }
            }
        }
    }

    pub fn handle_sys_knob(&mut self, ds_settings: &dyn settings::DataSource) {
        if !ds_settings.is_enable() {
            self.turn_off_sys_all();
            return;
        }

        if let Some(sub_sys) = ds_settings.get_sub_system(settings::SubSystemType::Mem) {
            if !sub_sys.is_enable() {
                self.turn_off_sys_mem();
            }
        }
    }

    pub fn turn_off_sys_all(&mut self) {
        self.turn_off_sys_mem();
    }

    pub fn handle_proc_knob(&mut self, ds_settings: &dyn settings::DataSource) {
        if !ds_settings.is_enable() {
            self.turn_off_proc_all();
            return;
        }

        if let Some(sub_sys) = ds_settings.get_sub_system(settings::SubSystemType::Cpu) {
            if !sub_sys.is_enable() {
                self.turn_off_proc_cpu();
            }
        }

        if let Some(sub_sys) = ds_settings.get_sub_system(settings::SubSystemType::Mem) {
            if !sub_sys.is_enable() {
                self.turn_off_proc_mem();
            }
        }

        if let Some(sub_sys) = ds_settings.get_sub_system(settings::SubSystemType::Net) {
            if !sub_sys.is_enable() {
                self.turn_off_proc_net();
            }
        }

        if let Some(sub_sys) = ds_settings.get_sub_system(settings::SubSystemType::Storage) {
            if !sub_sys.is_enable() {
                self.turn_off_proc_disk();
            }
        }
    }

    pub fn turn_off_proc_all(&mut self) {
        self.turn_off_proc_cpu();
        self.turn_off_proc_mem();
        self.turn_off_proc_net();
        self.turn_off_proc_disk();
    }

    pub fn turn_off_cgroup(&mut self, ds_settings: &dyn settings::DataSource) {
        info!("turn_off_cgroup");
        // delete invalid cgroup item
        self.cgroup_list
            .retain(|_, cgroup| !cgroup.sub_system_groups.is_empty());

        if !self.cgroup_list.is_empty() {
            info!(
                "[reset cgroups] cgroup list = {:?}",
                self.cgroup_list.keys()
            );
            for (cgroup_path, cgroup) in self.cgroup_list.iter_mut() {
                match cgroup.reset(ds_settings) {
                    Ok(_) => {}
                    Err(e) => warn!(
                        "reset cgroup error, cgroup.path={}, e={}",
                        cgroup_path.display(),
                        e
                    ),
                }
            }
        }
    }

    pub fn turn_off_sys(&mut self) {
        self.turn_off_sys_mem();
    }

    pub fn turn_off_sys_mem(&mut self) {
        info!("turn_off_sys_mem");
        self.system_devices_node.reset();
    }

    pub fn turn_off_byteperf(&mut self) {
        info!("turn_off_byteperf");
        let module_vec: Vec<String> = Vec::new();
        let pmu_idx_vec = ffi::PERF_MODULE_MASK_CONFIG.get_idx_vec(module_vec);
        let pmu_module_mask = ffi::ModuleMask::from(pmu_idx_vec);
        ffi::wrapper_byteperf_module_control(pmu_module_mask);
        //ffi::wrapper_byteperf_destroy_malachite();
        self.reset_byteperf();
    }

    pub fn reset_byteperf(&mut self) {
        //self.reset_cgroup_pmu_data();
        self.reset_numa_pmu_data();
        self.reset_cpu_cpi_data();
    }

    pub fn turn_off_ebpf(&mut self) {
        ffi::wrapper_free_bpf();
        self.turn_off_system_event();
        let ds = settings::get_disabeld_data_source();
        self.turn_off_cgroup(&*ds);
    }

    pub fn handle_ebpf_knob(&mut self, ds_settings: &dyn settings::DataSource) {
        if !ds_settings.is_enable() {
            return self.turn_off_ebpf();
        }

        let ebpf_enable_submodule_list = vec!["BPF_MODULE_ID_BPF_MODULE_CGROUP".to_string()];
        let ebpf_idx_vec = ffi::BPF_MODULE_MASK_CONFIG.get_idx_vec(ebpf_enable_submodule_list);
        let bpf_module_mask = ffi::ModuleMask::from(ebpf_idx_vec);
        let code = crate::ffi::wrapper_init_bpf(bpf_module_mask);
        if code != 0 {
            warn!("[lib] init bpf error: code = {}", code);
            return;
        }
        info!("[lib] init bpf success");

        let mut system_event_mask = bpf_ffi::system_event_mask::new();
        system_event_mask.enable_generic();
        let mut sub_sys_disabled = false;
        let mut ebpf_enable_submodule: Vec<String> = Vec::new();
        ebpf_enable_submodule.push("BPF_MODULE_ID_BPF_MODULE_CGROUP".to_string());
        ebpf_enable_submodule.push("BPF_MODULE_ID_BPF_MODULE_BPFSTAT".to_string());

        if let Some(sub_sys) = ds_settings.get_sub_system(settings::SubSystemType::Mem) {
            if sub_sys.is_enable() {
                ebpf_enable_submodule.push("BPF_MODULE_ID_BPF_MODULE_CGROUP_MEM".to_string());
                system_event_mask.enable_mem();
            } else {
                info!("ebpf sub system mem is disabled");
                sub_sys_disabled = true;
            }
        }

        if let Some(sub_sys) = ds_settings.get_sub_system(settings::SubSystemType::Cpu) {
            if sub_sys.is_enable() {
                ebpf_enable_submodule.push("BPF_MODULE_ID_BPF_MODULE_CGROUP_SCHED".to_string());
                //ebpf_enable_submodule.push("BPF_MODULE_ID_BPF_MODULE_PMU".to_string());
                system_event_mask.enable_sched();
            } else {
                info!("ebpf sub system cpu is disabled");
                sub_sys_disabled = true;
            }
        }

        if let Some(sub_sys) = ds_settings.get_sub_system(settings::SubSystemType::Storage) {
            if sub_sys.is_enable() {
                ebpf_enable_submodule.push("BPF_MODULE_ID_BPF_MODULE_CGROUP_FS".to_string());
                ebpf_enable_submodule.push("BPF_MODULE_ID_BPF_MODULE_CGROUP_IO".to_string());
                system_event_mask.enable_storage();
            } else {
                info!("ebpf sub system disk is disabled");
                sub_sys_disabled = true;
            }
        }

        if let Some(sub_sys) = ds_settings.get_sub_system(settings::SubSystemType::Net) {
            if sub_sys.is_enable() {
                ebpf_enable_submodule.push("BPF_MODULE_ID_BPF_MODULE_CGROUP_NET".to_string());
                system_event_mask.enable_net();
            } else {
                info!("ebpf sub system net is disabled");
                sub_sys_disabled = true;
            }
        }

        // down/up system_event moudle to enable config change
        let ebpf_idx_vec = ffi::BPF_MODULE_MASK_CONFIG.get_idx_vec(ebpf_enable_submodule.clone());
        let bpf_module_mask = ffi::ModuleMask::from(ebpf_idx_vec);
        let ret = ffi::wrapper_bpf_module_ctl(bpf_module_mask);
        if ret != 0 {
            warn!("bpf_module_ctl failed: {}", ret);
        } else {
            info!("bpf_module_ctl success");
        }

        ebpf_enable_submodule.push("BPF_MODULE_ID_SYSTEM_EVENT".to_string());

        let ebpf_idx_vec = ffi::BPF_MODULE_MASK_CONFIG.get_idx_vec(ebpf_enable_submodule);
        let bpf_module_mask = ffi::ModuleMask::from(ebpf_idx_vec);
        let ret = ffi::wrapper_bpf_module_ctl(bpf_module_mask);
        if ret != 0 {
            warn!("bpf_module_ctl failed: {}", ret);
        } else {
            info!("bpf_module_ctl success");
        }

        info!(
            "wrapper_system_event_config with mask: {}",
            system_event_mask.mask
        );
        ffi::wrapper_system_event_config(system_event_mask);

        if !ds_settings.is_enable() || sub_sys_disabled {
            // TODO: just reset ebpf field
            self.turn_off_cgroup(ds_settings);
        }
    }

    pub fn turn_off_proc_cpu(&mut self) {
        info!("turn_off_proc_cpu");
        self.load.reset();
        self.processors.reset();
        self.process_stats.reset();
    }
    pub fn turn_off_proc_mem(&mut self) {
        info!("turn_off_proc_mem");
        self.memory.reset();
    }
    pub fn turn_off_proc_net(&mut self) {
        info!("turn_off_proc_net");
        self.net_traffic.reset();
        self.net_info.reset();
    }

    pub fn turn_off_proc_disk(&mut self) {
        info!("turn_off_proc_disk");
        self.disk_io.reset();
    }

    pub fn turn_off_system_event(&mut self) {
        info!("turn_off_system_event");
        self.system_events.reset();
    }
}

impl Default for System {
    fn default() -> Self {
        Self::new()
    }
}
