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
use utoipa::ToSchema;

#[derive(Clone, Debug, Copy, Deserialize, Serialize, ToSchema)]
pub struct PSIItem {
    avg10: f64,
    avg60: f64,
    avg300: f64,
    total: f64,
}

impl PSIItem {
    pub fn new(avg10: f64, avg60: f64, avg300: f64, total: f64) -> PSIItem {
        PSIItem {
            avg10,
            avg60,
            avg300,
            total,
        }
    }
    pub fn get_avg10(&self) -> f64 {
        self.avg10
    }
    pub fn get_avg60(&self) -> f64 {
        self.avg60
    }
    pub fn get_avg300(&self) -> f64 {
        self.avg300
    }
    pub fn get_total(&self) -> f64 {
        self.total
    }
}

#[derive(Clone, Debug, Copy, Deserialize, Serialize, ToSchema)]
pub struct PressureStallInfo {
    some: Option<PSIItem>,
    full: Option<PSIItem>,
}

impl Default for PressureStallInfo {
    fn default() -> Self {
        PressureStallInfo::new()
    }
}

impl PressureStallInfo {
    pub fn new() -> PressureStallInfo {
        PressureStallInfo {
            some: None,
            full: None,
        }
    }
    pub fn get_some(&self) -> Option<PSIItem> {
        self.some
    }
    pub fn get_full(&self) -> Option<PSIItem> {
        self.full
    }

    pub fn set_some(&mut self, some: Option<PSIItem>) {
        self.some = some
    }
    pub fn set_full(&mut self, full: Option<PSIItem>) {
        self.full = full
    }
}

impl From<&str> for PressureStallInfo {
    fn from(input: &str) -> Self {
        let pressure_data = parse_cgroup_pressure(input);
        let pressure_some_data = pressure_data.get(&"some");
        let pressure_full_data = pressure_data.get(&"full");

        let psi_some = pressure_some_data.map(|x| {
            PSIItem::new(
                *x.get(&"avg10").unwrap_or(&0.0),
                *x.get(&"avg60").unwrap_or(&0.0),
                *x.get(&"avg300").unwrap_or(&0.0),
                *x.get(&"total").unwrap_or(&0.0),
            )
        });
        let psi_full = pressure_full_data.map(|x| {
            PSIItem::new(
                *x.get(&"avg10").unwrap_or(&0.0),
                *x.get(&"avg60").unwrap_or(&0.0),
                *x.get(&"avg300").unwrap_or(&0.0),
                *x.get(&"total").unwrap_or(&0.0),
            )
        });
        PressureStallInfo {
            some: psi_some,
            full: psi_full,
        }
    }
}

pub fn parse_cgroup_pressure(contents: &str) -> HashMap<&str, HashMap<&str, f64>> {
    let mut ret_map: HashMap<&str, HashMap<&str, f64>> = HashMap::new();
    //some avg10=0.00 avg60=0.00 avg300=0.00 total=0
    //full avg10=0.00 avg60=0.00 avg300=0.00 total=0
    contents.split('\n').into_iter().for_each(|s| {
        let mut space = s.split_whitespace();
        if let Some(field) = space.next() {
            let item_map = ret_map.entry(field).or_insert_with(HashMap::new);
            space.for_each(|s| {
                let mut item = s.split('=');
                if let Some(item_key) = item.next() {
                    if let Some(item_value) = item.next() {
                        item_map.insert(item_key, item_value.parse::<f64>().unwrap());
                    }
                }
            });
        }
    });
    ret_map
}

#[cfg(test)]
mod tests_psi {
    use super::*;

    #[test]
    fn test_parse_cgroup_pressure() {
        let file_content = "some avg10=1.23 avg60=4.56 avg300=7.89 total=12345\nfull avg10=9.87 avg60=6.54 avg300=3.21 total=54321\n";
        let output = parse_cgroup_pressure(file_content);
        let mut correct_content: HashMap<&str, HashMap<&str, f64>> = HashMap::new();
        let mut some_hash: HashMap<&str, f64> = HashMap::new();
        some_hash.insert(&"avg10", 1.23);
        some_hash.insert(&"avg60", 4.56);
        some_hash.insert(&"avg300", 7.89);
        some_hash.insert(&"total", 12345.0);
        let mut full_hash: HashMap<&str, f64> = HashMap::new();
        full_hash.insert(&"avg10", 9.87);
        full_hash.insert(&"avg60", 6.54);
        full_hash.insert(&"avg300", 3.21);
        full_hash.insert(&"total", 54321.0);
        correct_content.insert("some", some_hash);
        correct_content.insert("full", full_hash);
        assert_eq!(output, correct_content);
    }

    #[test]
    fn test_psi_item() {
        let file_content = "some avg10=1.23 avg60=4.56 avg300=7.89 total=12345\nfull avg10=9.87 avg60=6.54 avg300=3.21 total=54321\n";
        let psi_item = PressureStallInfo::from(file_content);
        assert_eq!(psi_item.some.unwrap().avg10, 1.23);
        assert_eq!(psi_item.some.unwrap().total, 12345.0);
        assert_eq!(psi_item.full.unwrap().avg60, 6.54);
        assert_eq!(psi_item.full.unwrap().total, 54321.0);
    }
}
