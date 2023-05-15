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

/// wrapper for ffi/pmu
///
use crate::ffi::pmu::ffi::{
    byteperf_cgroup_buffer_malachite, byteperf_check_module_health, byteperf_config_malachite,
    byteperf_count_buffer_malachite, byteperf_cpu_buffer_malachite, byteperf_destroy_malachite,
    byteperf_gather_count_malachite, byteperf_hardware_malachite, byteperf_iio_buffer_malachite,
    byteperf_imc_buffer_malachite, byteperf_l3cache_buffer_malachite, byteperf_module_control,
    byteperf_power_buffer_malachite, byteperf_setup_malachite, byteperf_update_cgroup_path,
    byteperf_upi_buffer_malachite,
};
use crate::ffi::{ModuleMask, ModuleMaskConfig};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::error::Error;
use std::ffi::CString;
use std::sync::Once;
use std::thread::sleep;
use std::time::Duration;
use std::env;
use log::info;

pub(crate) mod ffi;

static WRAPPER_BYTEPERF_INIT_GATHER_FUNC: Once = Once::new();

#[cfg(not(tarpaulin_include))]
impl Default for byteperf_hardware_malachite {
    fn default() -> Self {
        byteperf_hardware_malachite {
            psm_name: [0 as ::std::os::raw::c_char; 255usize],
            ip_addr: [0 as ::std::os::raw::c_char; 255usize],
            ipv6_addr: [0 as ::std::os::raw::c_char; 255usize],
            host_name: [0 as ::std::os::raw::c_char; 255usize],
            arch_name: [0 as ::std::os::raw::c_char; 255usize],
            platform_name: [0 as ::std::os::raw::c_char; 255usize],
        }
    }
}

#[cfg(not(tarpaulin_include))]
impl Default for byteperf_cgroup_buffer_malachite {
    fn default() -> Self {
        byteperf_cgroup_buffer_malachite {
            name: [0 as ::std::os::raw::c_char; 255usize],
            numa: [0 as ::std::os::raw::c_char; 10usize],
            core_num: 0,
            cpi: 0.0,
            instructions: 0.0,
            cycles: 0.0,
            l3_misses: 0.0,
            utilization: 0.0,
        }
    }
}

#[cfg(not(tarpaulin_include))]
impl Default for byteperf_cpu_buffer_malachite {
    fn default() -> Self {
        byteperf_cpu_buffer_malachite {
            cpu: 0,
            cpi: 0.0,
            instructions: 0,
            cycles: 0,
            l3_misses: 0,
            utilization: 0.0,
        }
    }
}

#[cfg(not(tarpaulin_include))]
impl Default for byteperf_imc_buffer_malachite {
    fn default() -> Self {
        byteperf_imc_buffer_malachite {
            name: [0 as ::std::os::raw::c_char; 255usize],
            socket: 0,
            numa_id: 0,
            memory_read_bandwidth: 0.0,
            memory_write_bandwidth: 0.0,
            memory_total_bandwidth: 0.0,
            memory_idle_bandwidth: 0.0,
            memory_bandwidth_util: 0.0,
            memory_read_latency: 0.0,
            memory_write_latency: 0.0,
        }
    }
}

#[cfg(not(tarpaulin_include))]
impl Default for byteperf_iio_buffer_malachite {
    fn default() -> Self {
        byteperf_iio_buffer_malachite {
            name: [0 as ::std::os::raw::c_char; 255usize],
            socket: 0,
            p0_read_bw: 0.0,
            p1_read_bw: 0.0,
            p2_read_bw: 0.0,
            p3_read_bw: 0.0,
            p4_read_bw: 0.0,
            p5_read_bw: 0.0,
            p6_read_bw: 0.0,
            p7_read_bw: 0.0,
            p0_write_bw: 0.0,
            p1_write_bw: 0.0,
            p2_write_bw: 0.0,
            p3_write_bw: 0.0,
            p4_write_bw: 0.0,
            p5_write_bw: 0.0,
            p6_write_bw: 0.0,
            p7_write_bw: 0.0,
            p0_latency: 0.0,
            p1_latency: 0.0,
            p2_latency: 0.0,
            p3_latency: 0.0,
            p4_latency: 0.0,
            p5_latency: 0.0,
            p6_latency: 0.0,
            p7_latency: 0.0,
        }
    }
}

#[cfg(not(tarpaulin_include))]
impl Default for byteperf_upi_buffer_malachite {
    fn default() -> Self {
        byteperf_upi_buffer_malachite {
            name: [0 as ::std::os::raw::c_char; 255usize],
            socket: 0,
            upi_rx_bandwidth: 0.0,
            upi_tx_bandwidth: 0.0,
            upi_tx_bandwidth_util: 0.0,
            upi_rx_latency: 0.0,
            upi_tx_latency: 0.0,
        }
    }
}

#[cfg(not(tarpaulin_include))]
impl Default for byteperf_power_buffer_malachite {
    fn default() -> Self {
        byteperf_power_buffer_malachite {
            name: [0 as ::std::os::raw::c_char; 255usize],
            socket: 0,
            base_freq: 0.0,
            runtime_freq: 0.0,
            package_energy: 0.0,
            core_energy: 0.0,
            dram_energy: 0.0,
            package_energy_limit: 0.0,
            dram_energy_limit: 0.0,
        }
    }
}

#[cfg(not(tarpaulin_include))]
impl Default for byteperf_l3cache_buffer_malachite {
    fn default() -> Self {
        byteperf_l3cache_buffer_malachite {
            name: [0 as ::std::os::raw::c_char; 255usize],
            socket: 0,
            numa_id: 0,
            l3_miss_ratio: 0.0,
            l3_miss_latency: 0.0,
        }
    }
}
#[cfg(not(tarpaulin_include))]
impl Default for byteperf_count_buffer_malachite {
    fn default() -> Self {
        byteperf_count_buffer_malachite {
            cpu_num: 0,
            cgroup_num: 0,
            device_num: 0,
            imc_num: 0,
            iio_num: 0,
            upi_num: 0,
            power_num: 0,
            l3_num: 0,
            hardware: byteperf_hardware_malachite::default(),
            cgroups: [byteperf_cgroup_buffer_malachite::default(); 256usize],
            cpus: [byteperf_cpu_buffer_malachite::default(); 256usize],
            imcs: [byteperf_imc_buffer_malachite::default(); 24usize],
            iios: [byteperf_iio_buffer_malachite::default(); 24usize],
            upis: [byteperf_upi_buffer_malachite::default(); 24usize],
            powers: [byteperf_power_buffer_malachite::default(); 24usize],
            l3cache: [byteperf_l3cache_buffer_malachite::default(); 64usize],
        }
    }
}

#[cfg(not(tarpaulin_include))]
pub fn wrapper_byteperf_destroy_malachite() {
    unsafe { byteperf_destroy_malachite() }
}

#[cfg(not(tarpaulin_include))]
pub fn wrapper_byteperf_setup_malachite() -> Result<(), Box<dyn Error>> {
    let mut is_failed = false;

    WRAPPER_BYTEPERF_INIT_GATHER_FUNC.call_once(|| {
        let mut p = env::current_exe().unwrap();
        p.pop();
        p.push("static/pmu_config");
        let xml_root = CString::new(p.to_str().unwrap()).unwrap().into_raw();
        let xml_module = CString::new("basic").unwrap().into_raw();

        let config = byteperf_config_malachite {
            command: 2, //
            xml_root,   // byteperf config 地址
            xml_module, // 固定值
            priority: 0,
            duration: 1000,  // 采集时间, ms
            interval: 30000, // 间隔，不能和 duration 相等, ms
            format: 0,
            cpus: [0; 256usize],
            cpu_num: 0,
            cgroups: [[0; 255usize]; 256usize],
            cgroup_num: 0,
        };

        let box_config = Box::new(config);
        let p_config = Box::into_raw(box_config);
        unsafe {
            let code = byteperf_setup_malachite(p_config);
            let _x = Box::from_raw(p_config);
            if code != 0 {
                info!("byteperf_setup_malachite failed: {}", code);
				info!("xml_root = {:?}", p.to_str().unwrap());
                is_failed = true;
            }
            let _ = CString::from_raw(xml_root);
            let _ = CString::from_raw(xml_module);
        }
    });

    if is_failed {
        Err("byteperf setup failed")?;
    }

    Ok(())
}

#[cfg(not(tarpaulin_include))]
pub fn wrapper_byteperf_update_cgroup_path(cgroup_paths: Vec<String>) {
    let mut cgroups: [[::std::os::raw::c_char; 255usize]; 256usize] = [[0; 255usize]; 256usize];
    for (index, value) in cgroup_paths.iter().enumerate() {
        let c_string = CString::new(value.to_string()).unwrap();
        let bytes = c_string.as_bytes_with_nul();
        #[cfg(target_arch = "x86_64")]
        let i8slice = unsafe { &*(bytes as *const _ as *const [i8]) };
        #[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
        let i8slice = unsafe { &*(bytes as *const _ as *const [u8]) };
        cgroups[index][..bytes.len()].copy_from_slice(i8slice);
    }
    let box_cgroups = Box::new(cgroups);
    let p_cgroups = Box::into_raw(box_cgroups);
    unsafe {
        byteperf_update_cgroup_path(cgroup_paths.len() as ::std::os::raw::c_int, p_cgroups);
        let _x = Box::from_raw(p_cgroups);
    }
}

#[cfg(not(tarpaulin_include))]
pub fn wrapper_byteperf_gather_count_malachite() -> byteperf_count_buffer_malachite {
    let box_buffer_data = Box::new(byteperf_count_buffer_malachite::default());
    let p_buffer_data = Box::into_raw(box_buffer_data);
    unsafe {
        // byteperf: cant get data first time
        byteperf_gather_count_malachite(p_buffer_data);
        sleep(Duration::from_millis(1000));
        byteperf_gather_count_malachite(p_buffer_data);
        byteperf_count_buffer_malachite::clone(Box::from_raw(p_buffer_data).as_ref())
    }
}

#[cfg(not(tarpaulin_include))]
pub fn wrapper_byteperf_module_control(mask: ModuleMask) {
    unsafe {
        byteperf_module_control(mask.get_mask() as ::std::os::raw::c_int);
        wrapper_byteperf_gather_count_malachite();
    }
}

#[cfg(not(tarpaulin_include))]
pub fn wrapper_byteperf_check_module_health() -> (::std::os::raw::c_int, ::std::os::raw::c_int) {
    let profile_module: ::std::os::raw::c_int = 0;
    let module_health: ::std::os::raw::c_int = 0;
    let box_profile_module = Box::new(profile_module);
    let box_module_health = Box::new(module_health);
    let p_profile_module = Box::into_raw(box_profile_module);
    let p_module_health = Box::into_raw(box_module_health);

    unsafe {
        byteperf_check_module_health(p_profile_module, p_module_health);
        return (
            *Box::from_raw(p_profile_module).as_ref(),
            *Box::from_raw(p_module_health).as_ref(),
        );
    }
}

pub static PERF_MODULE_MASK_CONFIG: Lazy<ModuleMaskConfig> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("BYTEPERF_CPU_MODULE", 1);
    m.insert("BYTEPERF_AMD_L3_MODULE", 2);
    m.insert("BYTEPERF_AMD_DF_MODULE", 3);
    m.insert("BYTEPERF_POWER_MODULE", 4);
    m.insert("BYTEPERF_MEMORY_MODULE", 5);
    m.insert("BYTEPERF_UPI_MODULE", 6);
    m.insert("BYTEPERF_IIO_MODULE", 7);
    m.insert("BYTEPERF_MCU_MODULE", 8);
    m.insert("BYTEPERF_CCIX_MODULE", 9);
    m.insert("BYTEPERF_PCIE_MODULE", 10);
    ModuleMaskConfig::new(m)
});

pub fn enable_byteperf() -> Result<(), Box<dyn Error>> {
    wrapper_byteperf_setup_malachite()?;
    Ok(())
}
