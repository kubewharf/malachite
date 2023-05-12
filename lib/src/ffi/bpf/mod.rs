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
/// wrapper for ffi/bpf
///
pub(crate) mod ffi;

use crate::ffi::bpf::ffi::{
    bpf_module_ctl, cgroup_fs_data, cgroup_io_latpcts, cgroup_mem_data, cgroup_net_data,
    cgroup_nr_tasks, free_bpf, fs_data, get_bpf_mask, get_cgroup_pmu_data, init_bpf, init_options,
    io_latpcts, mem_data, net_data, nr_tasks, pmu_data,
};

use crate::ffi::bpf::ffi as bpf_ffi;
use crate::ffi::common::{ModuleMask, ModuleMaskConfig};
use std::collections::HashMap;

pub const BPF_MODULE_CGROUP_SCHED: i32 = 1;
const BPF_MODULE_CGROUP_THROTTLE: i32 = 2;
pub const BPF_MODULE_CGROUP_MEM: i32 = 3;
pub const BPF_MODULE_CGROUP_IO: i32 = 4;
pub const BPF_MODULE_CGROUP_FS: i32 = 5;
pub const BPF_MODULE_CGROUP_NET: i32 = 6;
pub const BPF_MODULE_CGROUP_PMU: i32 = 10;
pub const BPF_MODULE_SYSTEM_EVENT: i32 = 11;

use log::{info, warn};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::ffi::CString;
use std::ops::Sub;
use std::path::PathBuf;
use std::sync::Mutex;

pub static BPF_MODULE_MASK_CONFIG: Lazy<ModuleMaskConfig> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("BPF_MODULE_ID_BPF_MODULE_CGROUP", 0);
    m.insert("BPF_MODULE_ID_BPF_MODULE_CGROUP_SCHED", 1);
    m.insert("BPF_MODULE_ID_BPF_MODULE_CGROUP_THROTTLE", 2);
    m.insert("BPF_MODULE_ID_BPF_MODULE_CGROUP_MEM", 3);
    m.insert("BPF_MODULE_ID_BPF_MODULE_CGROUP_IO", 4);
    m.insert("BPF_MODULE_ID_BPF_MODULE_CGROUP_FS", 5);
    m.insert("BPF_MODULE_ID_BPF_MODULE_CGROUP_NET", 6);
    m.insert("BPF_MODULE_ID_BPF_MODULE_SYSTEM", 7);
    m.insert("BPF_MODULE_ID_BPF_MODULE_BLOCK", 8);
    m.insert("BPF_MODULE_ID_BPF_MODULE_IOCOST", 9);
    m.insert("BPF_MODULE_ID_BPF_MODULE_PMU", 10);
    m.insert("BPF_MODULE_ID_BPF_MODULE_BPFSTAT", 11);
    m.insert("BPF_MODULE_ID_SYSTEM_EVENT", 12);
    m.insert("BPF_MODULE_ID_BPF_MODULE_MAX_ID", 13);
    ModuleMaskConfig::new(m)
});

static BPF_PROGRAM_MAX_CNT: Lazy<Mutex<i32>> = Lazy::new(|| Mutex::new(100));

#[derive(Default, Debug, Copy, Clone, Deserialize, Serialize)]
pub struct WrapperIoLatpcts {
    pub pcts: ::std::os::raw::c_uint,
    pub sum_latency: IoPercentLatency,
    pub driver_latency: IoPercentLatency,
}
#[derive(Default, Debug, Copy, Clone, Deserialize, Serialize)]
pub struct IoPercentLatency {
    pub read_latency: ::std::os::raw::c_uint,
    pub write_latency: ::std::os::raw::c_uint,
    pub discard_latency: ::std::os::raw::c_uint,
}

impl Sub for fs_data {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        Self {
            fs_created: self.fs_created - other.fs_created,
            fs_open: self.fs_open - other.fs_open,
            fs_read: self.fs_read - other.fs_read,
            fs_read_bytes: self.fs_read_bytes - other.fs_read_bytes,
            fs_write: self.fs_write - other.fs_write,
            fs_write_bytes: self.fs_write_bytes - other.fs_write_bytes,
            fs_fsync: self.fs_fsync - other.fs_fsync,
        }
    }
}

#[test]
fn test_sub_for_fs_data() {
    let fs_data_1 = fs_data {
        fs_created: 10,
        fs_open: 10,
        fs_read: 10,
        fs_read_bytes: 10,
        fs_write: 10,
        fs_write_bytes: 10,
        fs_fsync: 10,
    };
    let fs_data_2 = fs_data {
        fs_created: 3,
        fs_open: 3,
        fs_read: 3,
        fs_read_bytes: 3,
        fs_write: 3,
        fs_write_bytes: 3,
        fs_fsync: 3,
    };
    let fs_data_answer = fs_data {
        fs_created: 7,
        fs_open: 7,
        fs_read: 7,
        fs_read_bytes: 7,
        fs_write: 7,
        fs_write_bytes: 7,
        fs_fsync: 7,
    };
    assert_eq!(fs_data_1 - fs_data_2, fs_data_answer);
}

#[cfg(not(tarpaulin_include))]
pub fn wrapper_get_bpf_mask() -> ::std::os::raw::c_ulong {
    unsafe { get_bpf_mask() }
}

#[cfg(not(tarpaulin_include))]
pub fn is_bpf_moudule_valid(module: i32) -> bool {
    let mask = wrapper_get_bpf_mask();

    if mask & (1 << module) == 0 {
        return false;
    }

    true
}

#[cfg(not(tarpaulin_include))]
pub fn wrapper_init_bpf(mask: ModuleMask) -> ::std::os::raw::c_int {
    let opts = init_options {
        mask: mask.get_mask(),
    };
    let box_opts = Box::new(opts);
    let p_opts = Box::into_raw(box_opts);
    unsafe {
        let code = init_bpf(p_opts);
        let _x = Box::from_raw(p_opts);
        code
    }
}

#[cfg(not(tarpaulin_include))]
pub fn wrapper_free_bpf() {
    unsafe { free_bpf() }
}

#[cfg(not(tarpaulin_include))]
pub fn wrapper_bpf_module_ctl(mask: ModuleMask) -> ::std::os::raw::c_int {
    let mask_num = mask.get_mask();
    info!("ffi: wrapper_bpf_module_ctl with mask: {}", mask_num);
    unsafe { bpf_module_ctl(mask_num) }
}

#[cfg(not(tarpaulin_include))]
pub fn wrapper_get_cgroup_fs_data(cgroup_path: PathBuf) -> fs_data {
    let box_fs_data = Box::new(fs_data::default());
    let p_fs_data = Box::into_raw(box_fs_data);
    unsafe {
        let path = CString::new(cgroup_path.to_str().unwrap()).unwrap();
        let ret = cgroup_fs_data(path.as_ptr(), p_fs_data);
        if ret != 0 {
            warn!(
                "[ffi] bpf get cgroup fs data failed: {}, path= {}",
                ret,
                cgroup_path.display()
            );
        }
        return fs_data::clone(Box::from_raw(p_fs_data).as_ref());
    }
}

#[cfg(not(tarpaulin_include))]
pub fn wrapper_get_cgroup_net_data(cgroup_path: PathBuf) -> net_data {
    let box_net_data = Box::new(net_data::default());
    let p_net_data = Box::into_raw(box_net_data);
    unsafe {
        let path = CString::new(cgroup_path.to_str().unwrap()).unwrap();
        let ret = cgroup_net_data(path.as_ptr(), p_net_data);
        if ret != 0 {
            warn!(
                "[ffi] bpf get cgroup net data failed: {}, path= {}",
                ret,
                cgroup_path.display()
            );
        }
        return net_data::clone(Box::from_raw(p_net_data).as_ref());
    }
}

#[cfg(not(tarpaulin_include))]
pub fn wrapper_get_cgroup_nr_tasks(cgroup_path: PathBuf) -> nr_tasks {
    let box_nr_task_data = Box::new(nr_tasks::default());
    let p_nr_task_data = Box::into_raw(box_nr_task_data);
    unsafe {
        let path = CString::new(cgroup_path.to_str().unwrap()).unwrap();
        let ret = cgroup_nr_tasks(path.as_ptr(), p_nr_task_data);
        if ret != 0 {
            warn!(
                "[ffi] bpf get cgroup nr tasks failed: {}, path= {}",
                ret,
                cgroup_path.display()
            );
        }
        return nr_tasks::clone(Box::from_raw(p_nr_task_data).as_ref());
    }
}

#[cfg(not(tarpaulin_include))]
pub fn wrapper_get_cgroup_mem_data(cgroup_path: PathBuf) -> mem_data {
    let box_cgroup_mem_data = Box::new(mem_data::default());
    let p_cgroup_mem_data = Box::into_raw(box_cgroup_mem_data);
    unsafe {
        let path = CString::new(cgroup_path.to_str().unwrap()).unwrap();
        let ret = cgroup_mem_data(path.as_ptr(), p_cgroup_mem_data);
        if ret != 0 {
            warn!(
                "[ffi] bpf get cgroup mem data failed: {}, path= {}",
                ret,
                cgroup_path.display()
            );
        }
        return mem_data::clone(Box::from_raw(p_cgroup_mem_data).as_ref());
    }
}

pub fn wrapper_get_cgroup_io_latpcts(
    cgroup_path: PathBuf,
    pcts: ::std::os::raw::c_uint,
) -> WrapperIoLatpcts {
    let ret_lat: io_latpcts;
    let query_lat = io_latpcts {
        pcts,
        ..Default::default()
    };
    let box_io_latpcts = Box::new(query_lat);
    let p_io_latpcts = Box::into_raw(box_io_latpcts);
    unsafe {
        let path = CString::new(cgroup_path.to_str().unwrap()).unwrap();
        let ret = cgroup_io_latpcts(path.as_ptr(), p_io_latpcts, 1);
        if ret != 0 {
            warn!(
                "cgroup_io_latpcts failed: {}, path= {}",
                ret,
                cgroup_path.display()
            );
        }
        ret_lat = io_latpcts::clone(Box::from_raw(p_io_latpcts).as_ref());
    }

    WrapperIoLatpcts {
        pcts: ret_lat.pcts,
        sum_latency: IoPercentLatency {
            read_latency: ret_lat.sum_lat[0],
            write_latency: ret_lat.sum_lat[1],
            discard_latency: ret_lat.sum_lat[2],
        },
        driver_latency: IoPercentLatency {
            read_latency: ret_lat.driver_lat[0],
            write_latency: ret_lat.driver_lat[1],
            discard_latency: ret_lat.driver_lat[2],
        },
    }
}

#[cfg(not(tarpaulin_include))]
pub fn wrapper_get_cgroup_pmu_data(cgroup_path: PathBuf) -> pmu_data {
    let box_cgroup_pmu_data = Box::new(pmu_data::default());
    let p_cgroup_pmu_data = Box::into_raw(box_cgroup_pmu_data);
    unsafe {
        let path = CString::new(cgroup_path.to_str().unwrap()).unwrap();
        let ret = get_cgroup_pmu_data(path.as_ptr(), p_cgroup_pmu_data, 10);
        if ret != 0 {
            warn!(
                "[ffi] bpf get_cgroup_pmu_data failed: {}, path= {}",
                ret,
                cgroup_path.display()
            );
        }
        return pmu_data::clone(Box::from_raw(p_cgroup_pmu_data).as_ref());
    }
}

#[cfg(not(tarpaulin_include))]
pub fn wrapper_get_system_event_count() -> bpf_ffi::system_event_data {
    let box_system_event_data = Box::new(bpf_ffi::system_event_data::default());
    let p_system_event_data = Box::into_raw(box_system_event_data);
    unsafe {
        let ret = bpf_ffi::get_system_event_count(p_system_event_data);
        if ret != 0 {
            warn!("[ffi] bpf get_system_event_count failed: {}", ret);
        }
        return bpf_ffi::system_event_data::clone(Box::from_raw(p_system_event_data).as_ref());
    }
}

#[cfg(not(tarpaulin_include))]
pub fn wrapper_system_event_config(event_mask: bpf_ffi::system_event_mask) {
    let p_event_mask = Box::into_raw(Box::new(event_mask));
    unsafe {
        let ret = bpf_ffi::system_event_config(p_event_mask);
        if ret != 0 {
            warn!("[ffi] bpf system_event_config failed: {}", ret);
        }
        _ = Box::from_raw(p_event_mask);
    }
}

#[cfg(not(tarpaulin_include))]
pub fn wrapper_get_bpf_prog_stats() -> Vec<bpf_ffi::bpf_program_stats> {
    let mut cnt = BPF_PROGRAM_MAX_CNT.lock().unwrap();

    let mut stats = std::iter::repeat_with(bpf_ffi::bpf_program_stats::new)
        .take(*cnt as usize)
        .collect::<Vec<bpf_ffi::bpf_program_stats>>();

    let mut running_bpf_cnt = 0;
    let mut ret;
    unsafe {
        ret = bpf_ffi::get_bpf_prog_stats(stats.as_mut_ptr(), *cnt, &mut running_bpf_cnt);
        if ret != 0 && running_bpf_cnt > *cnt {
            // enlarge vec and retry once
            *cnt = running_bpf_cnt + 16;
            stats = std::iter::repeat_with(bpf_ffi::bpf_program_stats::new)
                .take(*cnt as usize)
                .collect::<Vec<bpf_ffi::bpf_program_stats>>();
            ret = bpf_ffi::get_bpf_prog_stats(stats.as_mut_ptr(), *cnt, &mut running_bpf_cnt);
        }
    }

    if ret != 0 {
        let vec: Vec<bpf_ffi::bpf_program_stats> = Vec::new();
        return vec;
    }

    if *cnt > running_bpf_cnt {
        stats.truncate(running_bpf_cnt as usize);
    }
    stats
}
