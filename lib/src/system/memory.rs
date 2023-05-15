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

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct MemoryInfo {
    mem_total: u64,
    mem_free: u64,
    mem_used: u64,
    mem_shm: u64,
    mem_available: u64,
    mem_buffers: u64,
    mem_page_cache: u64,
    mem_slab_reclaimable: u64,
    mem_dirty_page_cache: u64,
    mem_write_back_page_cache: u64,
    mem_swap_total: u64,
    mem_swap_free: u64,
    mem_util: f64,
    vm_watermark_scale_factor: u64,
    vmstat_pgsteal_kswapd: u64,
}

impl Default for MemoryInfo {
    fn default() -> Self {
        MemoryInfo::new()
    }
}

impl MemoryInfo {
    pub const fn new() -> MemoryInfo {
        MemoryInfo {
            mem_total: 0,
            mem_free: 0,
            mem_used: 0,
            mem_shm: 0,
            mem_available: 0,
            mem_buffers: 0,
            mem_page_cache: 0,
            mem_slab_reclaimable: 0,
            mem_dirty_page_cache: 0,
            mem_write_back_page_cache: 0,
            mem_swap_total: 0,
            mem_swap_free: 0,
            mem_util: 0.0,
            vm_watermark_scale_factor: 0,
            vmstat_pgsteal_kswapd: 0,
        }
    }
    pub fn util(&self) -> f64 {
        self.mem_util
    }
    pub fn total_memory(&self) -> u64 {
        self.mem_total
    }

    pub fn free_memory(&self) -> u64 {
        self.mem_free
    }

    pub fn available_memory(&self) -> u64 {
        self.mem_available
    }
    pub fn mem_buffers(&self) -> u64 {
        self.mem_buffers
    }
    pub fn used_memory(&self) -> u64 {
        self.mem_total
            - self.mem_free
            - self.mem_buffers
            - self.mem_page_cache
            - self.mem_slab_reclaimable
    }

    pub fn total_swap(&self) -> u64 {
        self.mem_swap_total
    }

    pub fn free_swap(&self) -> u64 {
        self.mem_swap_free
    }

    pub fn vm_watermark_scale_factor(&self) -> u64 {
        self.vm_watermark_scale_factor
    }

    pub fn vmstat_pgsteal_kswapd(&self) -> u64 {
        self.vmstat_pgsteal_kswapd
    }

    #[cfg(not(tarpaulin_include))]
    pub fn refresh(&mut self) {
        self.refresh_mem_info(PathBuf::from("/proc/meminfo"));
        self.refresh_vm_watermark(PathBuf::from("/proc/sys/vm/watermark_scale_factor"));
        self.refresh_vm_stat(PathBuf::from("/proc/vmstat"));
    }

    pub fn refresh_mem_info(&mut self, path: PathBuf) {
        let file_data = fs::read_to_string(&path).unwrap();
        let mem_info: HashMap<&str, &str> = file_data
            .split('\n')
            .into_iter()
            .map(|line| {
                let mut iter = line.split_whitespace();
                let key = iter.next().unwrap_or(" ");
                (&key[0..key.len() - 1], iter.next().unwrap_or(" "))
            })
            .collect::<HashMap<_, _>>();
        self.mem_total = mem_info
            .get(&"MemTotal")
            .unwrap_or(&"0")
            .parse::<u64>()
            .unwrap();
        self.mem_free = mem_info
            .get(&"MemFree")
            .unwrap_or(&"0")
            .parse::<u64>()
            .unwrap();
        self.mem_available = mem_info
            .get(&"MemAvailable")
            .unwrap()
            .parse::<u64>()
            .unwrap();
        self.mem_buffers = mem_info
            .get(&"Buffers")
            .unwrap_or(&"0")
            .parse::<u64>()
            .unwrap();
        self.mem_page_cache = mem_info
            .get(&"Cached")
            .unwrap_or(&"0")
            .parse::<u64>()
            .unwrap();
        self.mem_dirty_page_cache = mem_info
            .get(&"Dirty")
            .unwrap_or(&"0")
            .parse::<u64>()
            .unwrap();
        self.mem_write_back_page_cache = mem_info
            .get(&"Writeback")
            .unwrap_or(&"0")
            .parse::<u64>()
            .unwrap();
        self.mem_slab_reclaimable = mem_info
            .get(&"SReclaimable")
            .unwrap_or(&"0")
            .parse::<u64>()
            .unwrap();
        self.mem_swap_total = mem_info
            .get(&"SwapTotal")
            .unwrap_or(&"0")
            .parse::<u64>()
            .unwrap();
        self.mem_swap_free = mem_info
            .get(&"SwapFree")
            .unwrap_or(&"0")
            .parse::<u64>()
            .unwrap();
        self.mem_shm = mem_info
            .get(&"Shmem")
            .unwrap_or(&"0")
            .parse::<u64>()
            .unwrap();
        self.mem_used = self.used_memory();
        if self.mem_total != 0 {
            self.mem_util = (self.used_memory() / self.mem_total) as f64;
        }
    }

    pub fn refresh_vm_stat(&mut self, path: PathBuf) {
        let file_data = fs::read_to_string(&path).unwrap();
        let vm_stat_info: HashMap<&str, &str> = file_data
            .split('\n')
            .map(|line| {
                let mut iter = line.split_whitespace();
                (iter.next().unwrap_or(""), iter.next().unwrap_or(""))
            })
            .collect();

        self.vmstat_pgsteal_kswapd = vm_stat_info
            .get(&"pgsteal_kswapd")
            .unwrap()
            .parse::<u64>()
            .unwrap();
    }
    pub fn refresh_vm_watermark(&mut self, path: PathBuf) {
        let file = File::open(&path).unwrap();
        let reader = BufReader::new(file);
        if let Some(Ok(data)) = reader.lines().next() {
            self.vm_watermark_scale_factor = data.parse::<u64>().unwrap();
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}
#[cfg(test)]
mod tests_memory {
    use super::*;
    use std::env;

    #[test]
    fn test_memory() {
        let current_path: PathBuf = env::current_dir().unwrap();
        let mut mem_info = MemoryInfo::new();
        mem_info.refresh_mem_info(PathBuf::from(format!(
            "{}/tests/sample/proc_mem_info",
            current_path.to_string_lossy()
        )));

        mem_info.refresh_vm_watermark(PathBuf::from(format!(
            "{}/tests/sample/proc_sys_vm_watermark_scale_factor",
            current_path.to_string_lossy()
        )));

        mem_info.refresh_vm_stat(PathBuf::from(format!(
            "{}/tests/sample/proc_vm_stat",
            current_path.to_string_lossy()
        )));

        assert_eq!(mem_info.mem_total, 16166776);
        assert_eq!(mem_info.mem_free, 1817388);
        assert_eq!(mem_info.mem_available, 14523644);
        assert_eq!(mem_info.mem_buffers, 376960);
        assert_eq!(mem_info.mem_page_cache, 12172232);
        assert_eq!(mem_info.mem_slab_reclaimable, 611564);
        assert_eq!(mem_info.mem_dirty_page_cache, 272);
        assert_eq!(mem_info.mem_write_back_page_cache, 0);
        assert_eq!(mem_info.mem_swap_total, 0);
        assert_eq!(mem_info.mem_swap_free, 0);
        assert_eq!(mem_info.mem_util, 0.0);
        assert_eq!(mem_info.mem_shm, 153344);

        assert_eq!(mem_info.vm_watermark_scale_factor, 10);
        assert_eq!(mem_info.vmstat_pgsteal_kswapd, 2635);
    }
}
