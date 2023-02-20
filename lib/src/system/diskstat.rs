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

use log::warn;
use nix::sys::statfs::*;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiskStat {
    primary_device_id: usize,
    secondary_device_id: usize,
    device_name: String,
    io_read: u64,
    io_write: u64,
    io_busy: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiskUsage {
    mount_point: String,
    filesystem_type: String,
    total: u64,
    free: u64,
    usage: f32,
    total_inodes: u64,
    free_inodes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Disk {
    stats: Vec<DiskStat>,
    usage: Vec<DiskUsage>,
}

impl Default for Disk {
    fn default() -> Self {
        Disk::new()
    }
}

impl Disk {
    pub fn new() -> Disk {
        Disk {
            stats: vec![],
            usage: vec![],
        }
    }
    pub fn get_stats(&self) -> &Vec<DiskStat> {
        &self.stats
    }
    pub fn get_usage(&self) -> &Vec<DiskUsage> {
        &self.usage
    }
    #[cfg(not(tarpaulin_include))]
    pub fn refresh(&mut self) {
        self.refresh_disk_stat(PathBuf::from("/proc/diskstats"));
        self.refresh_disk_usage(PathBuf::from("/proc/self/mountinfo"));
    }
    fn refresh_disk_stat(&mut self, path: PathBuf) {
        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        let mut stats: Vec<DiskStat> = vec![];
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    let vec = line.split_whitespace().collect::<Vec<&str>>();
                    let disk_stat = DiskStat {
                        primary_device_id: vec[0].parse::<usize>().unwrap(),
                        secondary_device_id: vec[1].parse::<usize>().unwrap(),
                        device_name: vec[2].to_string(),
                        io_read: vec[3].parse::<u64>().unwrap(),
                        io_write: vec[7].parse::<u64>().unwrap(),
                        io_busy: vec[12].parse::<u64>().unwrap(),
                    };
                    stats.push(disk_stat);
                }
                Err(e) => {
                    warn!("[lib] diskstat, parse /proc/diskstats error: {}", e);
                    continue;
                }
            }
        }
        self.stats = stats;
    }

    fn refresh_disk_usage(&mut self, path: PathBuf) {
        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        let mut usage_vec: Vec<DiskUsage> = vec![];
        for line in reader.lines() {
            // 36 35 98:0 /mnt1 /mnt2 rw,noatime master:1 - ext3 /dev/root rw,errors=continue
            // (1)(2)(3)   (4)   (5)      (6)      (7)   (8) (9)   (10)         (11)
            match line {
                Ok(line) => {
                    let vec = line.split_whitespace().collect::<Vec<&str>>();
                    let mount_point_idx = 4;
                    if vec.len() <= mount_point_idx {
                        warn!("[lib] disk usage with invalid mountinfo: {:?}", line);
                        continue;
                    }
                    let mount_point = vec[mount_point_idx].parse::<String>().unwrap();

                    let mut separator_idx = 6;
                    loop {
                        if vec.len() <= separator_idx {
                            warn!("[lib] disk usage with invalid mountinfo: {:?}", line);
                            break;
                        }

                        if vec[separator_idx].parse::<String>().unwrap() == "-" {
                            break;
                        }
                        separator_idx += 1;
                    }

                    let filesystem_type_idx = separator_idx + 1;
                    if vec.len() <= filesystem_type_idx {
                        continue;
                    }

                    let filesystem_type = vec[filesystem_type_idx].parse::<String>().unwrap();
                    match filesystem_type.as_str() {
                        "overlay" | "tmpfs" => continue,
                        _ => (),
                    }

                    let mp_file = match File::open(mount_point.clone()) {
                        Ok(file) => file,
                        Err(e) => {
                            warn!("open mount point {} failed: {}", mount_point, e);
                            continue;
                        }
                    };
                    let fs = match fstatfs(&mp_file) {
                        Ok(fs) => fs,
                        Err(e) => {
                            warn!("fstatfs mountpoint {} error: {}", mount_point, e);
                            continue;
                        }
                    };
                    let total = fs.blocks() * fs.block_size() as u64;
                    if total == 0 {
                        continue;
                    }
                    let free = fs.blocks_available() * fs.block_size() as u64;
                    if free > total {
                        continue;
                    }
                    let usage = (total - free) as f32 / total as f32 * 100.;

                    let total_inodes = fs.files();
                    let free_inodes = fs.files_free();

                    let disk_usage = DiskUsage {
                        mount_point,
                        filesystem_type,
                        total,
                        free,
                        usage,
                        total_inodes,
                        free_inodes,
                    };
                    usage_vec.push(disk_usage);
                }
                Err(e) => {
                    warn!("[lib] diskusage, parse /proc/self/mountinfo error: {}", e);
                    continue;
                }
            }
        }
        self.usage = usage_vec;
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

#[cfg(test)]
mod tests_disk {
    use super::*;
    use std::env;
    #[test]
    fn test_disk() {
        let current_path: PathBuf = env::current_dir().unwrap();
        let path: PathBuf = PathBuf::from(format!(
            "{}/tests/sample/proc_diskstats",
            current_path.to_string_lossy()
        ));
        let mut disk_instance = Disk::default();
        disk_instance.refresh_disk_stat(path);

        assert_eq!(disk_instance.stats.len(), 21);

        let item2 = &disk_instance.stats[2];
        assert_eq!(item2.device_name, "nvme0n1p2");
        assert_eq!(item2.primary_device_id, 259);
        assert_eq!(item2.secondary_device_id, 2);
        assert_eq!(item2.io_read, 5580695);
        assert_eq!(item2.io_write, 611239004);
        assert_eq!(item2.io_busy, 29607212);
    }
}
