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

use crate::cpu::NodeVec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::ops::Deref;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NumaNode {
    id: usize,
    path: PathBuf,
    cpu_list: NodeVec,
    mem_free: u64,
    mem_used: u64,
    mem_total: u64,
    mem_shmem: u64,
    mem_available: u64,
    mem_file_pages: u64,
    mem_read_bandwidth_mb: f64,
    mem_write_bandwidth_mb: f64,
    mem_read_latency: f64,
    mem_write_latency: f64,
    mem_mx_bandwidth_mb: f64,
    mem_theory_mx_bandwidth_mb: f64,
    channels: Vec<ImcChannelInfo>,
}

impl NumaNode {
    pub fn new(id: usize, path: PathBuf) -> NumaNode {
        let mut cpu_list_file = path.clone();
        cpu_list_file.push("cpulist");
        let file = File::open(&cpu_list_file).unwrap();
        let reader = BufReader::new(file);
        let file_data = reader.lines().next().unwrap().unwrap();

        NumaNode {
            id,
            path,
            cpu_list: NodeVec::from(file_data),
            mem_free: 0,
            mem_used: 0,
            mem_total: 0,
            mem_shmem: 0,
            mem_available: 0,
            mem_file_pages: 0,
            mem_read_bandwidth_mb: 0.0,
            mem_write_bandwidth_mb: 0.0,
            mem_read_latency: 0.0,
            mem_write_latency: 0.0,
            mem_mx_bandwidth_mb: 0.0,
            mem_theory_mx_bandwidth_mb: 0.0,
            channels: Vec::new(),
        }
    }

    pub fn get_id(&self) -> usize {
        self.id
    }
    pub fn refresh_numa_max_bandwidth(&mut self, mem_mx_bd: f64) {
        self.mem_theory_mx_bandwidth_mb = mem_mx_bd
    }
    pub fn refresh_numa_mem(
        &mut self,
        mem_read_bandwidth_mb: f64,
        mem_write_bandwidth_mb: f64,
        mem_theory_mx_bandwidth_mb: f64,
        mem_read_latency: f64,
        mem_write_latency: f64,
        channels: Vec<ImcChannelInfo>,
    ) {
        if mem_theory_mx_bandwidth_mb == 0.0
            && mem_write_bandwidth_mb == 0.0
            && mem_read_bandwidth_mb == 0.0
            && mem_read_latency == 0.0
            && mem_write_latency == 0.0
        {
            return;
        }
        self.mem_read_bandwidth_mb = mem_read_bandwidth_mb;
        self.mem_write_bandwidth_mb = mem_write_bandwidth_mb;
        self.mem_theory_mx_bandwidth_mb = mem_theory_mx_bandwidth_mb;
        self.mem_read_latency = mem_read_latency;
        self.mem_write_latency = mem_write_latency;
        self.channels = channels;
    }

    pub fn reset_numa_mem_bandwidth(&mut self) {
        self.mem_read_bandwidth_mb = 0.0;
        self.mem_write_bandwidth_mb = 0.0;
        self.mem_theory_mx_bandwidth_mb = 0.0;
        self.channels = Vec::new();
    }
    pub fn reset_numa_mem_latency(&mut self) {
        self.mem_read_latency = 0.0;
        self.mem_write_latency = 0.0;
    }
    pub fn refresh_numa_mem_info(&mut self) {
        let mut file_path = PathBuf::from(&self.path);
        file_path.push("meminfo");
        let file_data = fs::read_to_string(&file_path).unwrap();
        let numa_mem_info: HashMap<&str, &str> = file_data
            .split('\n')
            .map(|line| {
                let mut iter = line.split_whitespace().take(4);
                let key = iter.nth(2).unwrap_or(" ");
                (&key[0..key.len() - 1], iter.next().unwrap_or(" "))
            })
            .collect::<HashMap<_, _>>();
        self.mem_used = numa_mem_info
            .get(&"MemUsed")
            .unwrap()
            .parse::<u64>()
            .unwrap();
        self.mem_shmem = numa_mem_info.get(&"Shmem").unwrap().parse::<u64>().unwrap();
        self.mem_total = numa_mem_info
            .get(&"MemTotal")
            .unwrap()
            .parse::<u64>()
            .unwrap();
        self.mem_free = numa_mem_info
            .get(&"MemFree")
            .unwrap()
            .parse::<u64>()
            .unwrap();
        self.mem_file_pages = numa_mem_info
            .get(&"FilePages")
            .unwrap()
            .parse::<u64>()
            .unwrap();
    }

    pub fn reset_numa_mem_info(&mut self) {
        self.mem_used = 0;
        self.mem_shmem = 0;
        self.mem_total = 0;
        self.mem_free = 0;
        self.mem_file_pages = 0;
    }

    pub fn refresh_numa_mem_availabe(&mut self, system_mem_water_mark: u64) {
        self.mem_available =
            self.mem_free + self.mem_file_pages - (self.mem_total * (system_mem_water_mark / 1000));
    }

    pub fn reset_numa_mem_availabe(&mut self) {
        self.mem_available = 0;
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImcChannelInfo {
    channel_name: String,
    mem_read_bandwidth: f64,
    mem_write_bandwidth: f64,
    mem_idle_bandwidth: f64,
    mem_bandwidth_util: f64,
    mem_read_latency: f64,
    mem_write_latency: f64,
}

impl ImcChannelInfo {
    pub fn new(
        channel_name: String,
        mem_read_bandwidth: f64,
        mem_write_bandwidth: f64,
        mem_idle_bandwidth: f64,
        mem_bandwidth_util: f64,
        mem_read_latency: f64,
        mem_write_latency: f64,
    ) -> ImcChannelInfo {
        ImcChannelInfo {
            channel_name,
            mem_read_bandwidth,
            mem_write_bandwidth,
            mem_idle_bandwidth,
            mem_bandwidth_util,
            mem_read_latency,
            mem_write_latency,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemDeviceNode {
    has_cpu: NodeVec,
    has_memory: NodeVec,
    pub nodes: Vec<NumaNode>,
    path: PathBuf,
}

impl Default for SystemDeviceNode {
    fn default() -> Self {
        SystemDeviceNode::new(PathBuf::from("/sys/devices/system/node"))
    }
}

impl SystemDeviceNode {
    pub fn get_nodes(&self) -> &Vec<NumaNode> {
        &self.nodes
    }

    fn new(system_node_path: PathBuf) -> SystemDeviceNode {
        let mut path = system_node_path.clone();
        path.push("has_cpu");
        let file = File::open(&path).unwrap();
        let reader = BufReader::new(file);
        let cpu_file_data = reader.lines().next().unwrap().unwrap();
        let has_cpu = NodeVec::from(cpu_file_data);
        path.pop();
        path.push("has_memory");
        let file = match File::open(&path) {
            Ok(data) => data,
            Err(_) => {
                path.pop();
                path.push("has_normal_memory");
                File::open(&path).unwrap()
            }
        };
        let reader = BufReader::new(file);
        let memory_file_data = reader.lines().next().unwrap().unwrap();
        let has_memory = NodeVec::from(memory_file_data);
        path.pop();

        let numa_nodes: Vec<NumaNode> = has_memory
            .deref()
            .iter()
            .map(|id| {
                path.push(format!("node{}", id));
                let node = NumaNode::new(*id, PathBuf::from(&path));
                path.pop();
                node
            })
            .collect::<_>();

        SystemDeviceNode {
            has_cpu,
            has_memory,
            nodes: numa_nodes,
            path: system_node_path,
        }
    }
    pub fn refresh_basic_info(&mut self) {
        self.nodes
            .iter_mut()
            .for_each(|node| node.refresh_numa_mem_info());
    }
    pub fn reset_basic_info(&mut self) {
        self.nodes
            .iter_mut()
            .for_each(|node| node.reset_numa_mem_info());
    }
    pub fn refresh_numa_avaiable_mem(&mut self, system_mem_water_mark: u64) {
        self.nodes
            .iter_mut()
            .for_each(|node| node.refresh_numa_mem_availabe(system_mem_water_mark));
    }
    pub fn reset_numa_avaiable_mem(&mut self) {
        self.nodes
            .iter_mut()
            .for_each(|node| node.reset_numa_mem_availabe());
    }
    pub fn reset(&mut self) {
        self.reset_basic_info();
        self.reset_numa_avaiable_mem();
    }
}

#[cfg(test)]
mod tests_numa_node {
    use super::*;
    use std::env;
    #[test]
    fn test_numa_node_list() {
        let current_path: PathBuf = env::current_dir().unwrap();
        let path: PathBuf = PathBuf::from(format!(
            "{}/tests/sample/sys_devices_system_node",
            current_path.to_string_lossy()
        ));
        let mut system_node_instance = SystemDeviceNode::new(path);
        system_node_instance.refresh_basic_info();

        let correct_node_vec = NodeVec::from("0-3".to_string());

        assert_eq!(system_node_instance.has_cpu, correct_node_vec);
        assert_eq!(system_node_instance.has_memory, correct_node_vec);

        assert_eq!(system_node_instance.nodes.len(), 4);

        system_node_instance.refresh_numa_avaiable_mem(500);

        let node2 = &system_node_instance.nodes[2];

        assert_eq!(node2.id, 2);
        let correct_cpu_list =
            NodeVec::from("24-27,31-33,37-39,43-44,72-75,79-81,85-87,91-92".to_string());
        assert_eq!(node2.cpu_list, correct_cpu_list);
        assert_eq!(node2.mem_total, 49540944);
        assert_eq!(node2.mem_free, 17778964);
        assert_eq!(node2.mem_shmem, 1300328);
        assert_eq!(node2.mem_used, 31761980);
        assert_eq!(node2.mem_file_pages, 3851204);
        assert_eq!(node2.mem_available, 21630168);
    }
}
