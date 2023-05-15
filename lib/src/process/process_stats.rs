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
use std::path::PathBuf;
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, Default, ToSchema)]
pub struct SystemProcessStats {
    procs_running: Option<u64>,
    procs_blocked: Option<u64>,
}

impl SystemProcessStats {
    pub fn refresh_process_stats(&mut self, path: PathBuf) {
        let file_data = fs::read_to_string(&path).unwrap();
        let kvdata: HashMap<&str, u64> = file_data
            .split('\n')
            .filter(|s| s.starts_with("procs_"))
            .map(|line| {
                let mut iter = line.split_whitespace();
                (
                    iter.next().unwrap(),
                    iter.next().unwrap().parse::<u64>().unwrap(),
                )
            })
            .collect();
        self.procs_blocked = kvdata.get("procs_blocked").cloned();
        self.procs_running = kvdata.get("procs_running").cloned();
    }

    #[cfg(not(tarpaulin_include))]
    pub fn refresh(&mut self) {
        self.refresh_process_stats(PathBuf::from("/proc/stat"));
    }

    pub fn reset(&mut self) {
        *self = Self::default()
    }
}

#[cfg(test)]
mod tests_system_process_stats {
    use super::*;
    use std::env;
    #[test]
    fn test_system_process_stats() {
        let current_path: PathBuf = env::current_dir().unwrap();
        let proc_stat_file = format!("{}/tests/sample/proc_stat", current_path.to_string_lossy());
        let mut system_process_stats = SystemProcessStats::default();
        system_process_stats.refresh_process_stats(PathBuf::from(proc_stat_file));
        assert_eq!(system_process_stats.procs_running, Some(2));
        assert_eq!(system_process_stats.procs_blocked, Some(1));
    }
}
