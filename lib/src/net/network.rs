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
use crate::net::utils::parse_net_file;
use crate::system::get_secs_since_epoch;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct NetInfo {
    tcp_delay_acks: u64,
    tcp_listen_overflows: u64,
    tcp_listen_drops: u64,
    tcp_abort_on_memory: u64,
    tcp_req_q_full_drop: u64,
    tcp_retran: f32,
    tcp_retrans_segs: u64,
    tcp_old_retrans_segs: u64,
    tcp_out_segs: u64,
    tcp_old_out_segs: u64,
    tcp_close_wait: usize,
    update_time: u64,
}

impl Default for NetInfo {
    fn default() -> Self {
        NetInfo::new()
    }
}

impl NetInfo {
    pub const fn new() -> NetInfo {
        NetInfo {
            tcp_delay_acks: 0,
            tcp_listen_overflows: 0,
            tcp_listen_drops: 0,
            tcp_abort_on_memory: 0,
            tcp_req_q_full_drop: 0,
            tcp_retran: 0.0,
            tcp_retrans_segs: 0,
            tcp_old_retrans_segs: 0,
            tcp_out_segs: 0,
            tcp_old_out_segs: 0,
            tcp_close_wait: 0,
            update_time: 0,
        }
    }

    #[cfg(not(tarpaulin_include))]
    pub fn refresh(&mut self) {
        self.refresh_snmp_data(PathBuf::from("/proc/net/snmp"));
        self.refresh_netstat_data(PathBuf::from("/proc/net/netstat"));
        //self.refresh_tcp_close_wait(PathBuf::from("/proc/net/tcp"));
        self.update_time = get_secs_since_epoch();
    }

    fn refresh_tcp_close_wait(&mut self, path: PathBuf) {
        let file = File::open(&path).unwrap();
        let reader = BufReader::with_capacity(8 * 1024 * 1024, file);
        self.tcp_close_wait = reader
            .lines()
            .filter_map(|line| {
                let line_str = line.unwrap();
                let mut iter = line_str.split_whitespace();

                match iter.nth(3).unwrap_or("") {
                    "08" => Some(line_str),
                    _ => None,
                }
            })
            .count();
    }

    fn refresh_netstat_data(&mut self, path: PathBuf) {
        let file_data = fs::read_to_string(&path).unwrap();
        let contents = parse_net_file(&file_data);
        let tcp_contents = contents.get(&"TcpExt").unwrap();
        self.tcp_delay_acks = tcp_contents
            .get(&"DelayedACKs")
            .unwrap_or(&"0")
            .parse::<u64>()
            .unwrap();
        self.tcp_listen_overflows = tcp_contents
            .get(&"ListenOverflows")
            .unwrap_or(&"0")
            .parse::<u64>()
            .unwrap();
        self.tcp_listen_drops = tcp_contents
            .get(&"ListenDrops")
            .unwrap_or(&"0")
            .parse::<u64>()
            .unwrap();
        self.tcp_abort_on_memory = tcp_contents
            .get(&"TCPAbortOnMemory")
            .unwrap_or(&"0")
            .parse::<u64>()
            .unwrap();
        self.tcp_req_q_full_drop = tcp_contents
            .get(&"TCPReqQFullDrop")
            .unwrap_or(&"0")
            .parse::<u64>()
            .unwrap();
    }
    fn refresh_snmp_data(&mut self, path: PathBuf) {
        let file_data = fs::read_to_string(&path).unwrap();
        let contents = parse_net_file(&file_data);
        let tcp_contents = contents.get(&"Tcp").unwrap();
        self.tcp_old_out_segs = self.tcp_out_segs;
        self.tcp_old_retrans_segs = self.tcp_retrans_segs;
        self.tcp_out_segs = tcp_contents
            .get(&"OutSegs")
            .unwrap_or(&"0")
            .parse::<u64>()
            .unwrap();
        self.tcp_retrans_segs = tcp_contents
            .get(&"RetransSegs")
            .unwrap_or(&"0")
            .parse::<u64>()
            .unwrap();
        self.tcp_retran = (self.tcp_retrans_segs - self.tcp_old_retrans_segs) as f32
            / (self.tcp_out_segs - self.tcp_old_out_segs) as f32;
    }
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

#[cfg(test)]
mod tests_net {
    use super::*;
    use std::env;

    #[test]
    fn test_net() {
        let current_path: PathBuf = env::current_dir().unwrap();
        let mut net_info = NetInfo::new();
        net_info.refresh_netstat_data(PathBuf::from(format!(
            "{}/tests/sample/proc_net_netstat",
            current_path.to_string_lossy()
        )));
        net_info.refresh_snmp_data(PathBuf::from(format!(
            "{}/tests/sample/proc_net_snmp",
            current_path.to_string_lossy()
        )));

        net_info.refresh_tcp_close_wait(PathBuf::from(format!(
            "{}/tests/sample/proc_net_tcp",
            current_path.to_string_lossy()
        )));
        assert_eq!(net_info.tcp_delay_acks, 6388849);
        assert_eq!(net_info.tcp_listen_overflows, 0);
        assert_eq!(net_info.tcp_listen_drops, 0);
        assert_eq!(net_info.tcp_abort_on_memory, 0);
        assert_eq!(net_info.tcp_req_q_full_drop, 0);
        assert_eq!(net_info.tcp_retran, 9.704428e-5);
        assert_eq!(net_info.tcp_retrans_segs, 46059890);
        assert_eq!(net_info.tcp_old_retrans_segs, 0);
        assert_eq!(net_info.tcp_out_segs, 474627530725);
        assert_eq!(net_info.tcp_old_out_segs, 0);
        assert_eq!(net_info.tcp_close_wait, 2);

        net_info.refresh_snmp_data(PathBuf::from(format!(
            "{}/tests/sample/proc_net_snmp",
            current_path.to_string_lossy()
        )));

        assert_eq!(net_info.tcp_old_retrans_segs, 46059890);
        assert_eq!(net_info.tcp_old_out_segs, 474627530725);
    }
}
