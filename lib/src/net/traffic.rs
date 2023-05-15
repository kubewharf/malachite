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

use crate::system::get_secs_since_epoch;
use serde::{Deserialize, Serialize};
use std::fs;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct NetworkCardTraffic {
    pub(crate) name: String,
    receive_bytes: u64,
    receive_packets: u64,
    receive_errs: u64,
    receive_drop: u64,
    receive_fifo: u64,
    receive_frame: u64,
    receive_compressed: u64,
    receive_multicast: u64,
    transmit_bytes: u64,
    transmit_packets: u64,
    transmit_errs: u64,
    transmit_drop: u64,
    transmit_fifo: u64,
    transmit_colls: i32,
    transmit_carrier: u64,
    transmit_compressed: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Traffic {
    inner: Vec<NetworkCardTraffic>,
    update_time: u64,
}

impl Deref for Traffic {
    type Target = Vec<NetworkCardTraffic>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Traffic {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Default for Traffic {
    fn default() -> Self {
        Traffic::new()
    }
}

impl Traffic {
    pub const fn new() -> Traffic {
        Traffic {
            inner: vec![],
            update_time: 0,
        }
    }

    #[cfg(not(tarpaulin_include))]
    pub fn refresh(&mut self) {
        self.refresh_device_data(PathBuf::from("/proc/net/dev"));
        self.update_time = get_secs_since_epoch();
    }

    fn refresh_device_data(&mut self, path: PathBuf) {
        let file_data = fs::read_to_string(&path).unwrap();
        self.clear();
        file_data.split('\n').skip(2).for_each(|line| {
            if !line.is_empty() {
                let mut iter = line.split_whitespace();
                let name = iter.next().unwrap();
                self.push(NetworkCardTraffic {
                    name: name[..name.len() - 1].to_string(),
                    receive_bytes: iter.next().unwrap().parse().unwrap(),
                    receive_packets: iter.next().unwrap().parse().unwrap(),
                    receive_errs: iter.next().unwrap().parse().unwrap(),
                    receive_drop: iter.next().unwrap().parse().unwrap(),
                    receive_fifo: iter.next().unwrap().parse().unwrap(),
                    receive_frame: iter.next().unwrap().parse().unwrap(),
                    receive_compressed: iter.next().unwrap().parse().unwrap(),
                    receive_multicast: iter.next().unwrap().parse().unwrap(),
                    transmit_bytes: iter.next().unwrap().parse().unwrap(),
                    transmit_packets: iter.next().unwrap().parse().unwrap(),
                    transmit_errs: iter.next().unwrap().parse().unwrap(),
                    transmit_drop: iter.next().unwrap().parse().unwrap(),
                    transmit_fifo: iter.next().unwrap().parse().unwrap(),
                    transmit_colls: iter.next().unwrap().parse().unwrap(),
                    transmit_carrier: iter.next().unwrap().parse().unwrap(),
                    transmit_compressed: iter.next().unwrap().parse().unwrap(),
                })
            }
        });
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

#[cfg(test)]
mod tests_traffic {
    use super::*;
    use std::collections::HashMap;
    use std::env;

    #[test]
    fn test_traffic() {
        let current_path: PathBuf = env::current_dir().unwrap();
        let mut traffic = Traffic::default();
        traffic.refresh_device_data(PathBuf::from(format!(
            "{}/tests/sample/proc_net_dev",
            current_path.to_string_lossy()
        )));
        let inner_vec: Vec<NetworkCardTraffic> = traffic.deref().clone();
        let net_card_traffic_info = inner_vec
            .into_iter()
            .map(|a| (a.name.to_string(), a))
            .collect::<HashMap<String, NetworkCardTraffic>>();

        let carma_br0 = net_card_traffic_info.get("carma_br0").unwrap();
        assert_eq!(carma_br0.receive_bytes, 17477117943);
        assert_eq!(carma_br0.receive_packets, 190479148);
        assert_eq!(carma_br0.receive_errs, 0);
        assert_eq!(carma_br0.receive_drop, 0);
        assert_eq!(carma_br0.receive_fifo, 0);
        assert_eq!(carma_br0.receive_frame, 0);
        assert_eq!(carma_br0.receive_compressed, 0);
        assert_eq!(carma_br0.receive_multicast, 0);
        assert_eq!(carma_br0.transmit_bytes, 468290113624);
        assert_eq!(carma_br0.transmit_packets, 131093781);
        assert_eq!(carma_br0.transmit_errs, 0);
        assert_eq!(carma_br0.transmit_drop, 0);
        assert_eq!(carma_br0.transmit_fifo, 0);
        assert_eq!(carma_br0.transmit_colls, 0);
        assert_eq!(carma_br0.transmit_carrier, 0);
        assert_eq!(carma_br0.transmit_compressed, 0);
    }
}
