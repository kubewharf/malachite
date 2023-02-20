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

use crate::common;
use crate::psi::PressureStallInfo;
use log::warn;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SystemPSI {
    cpu: PressureStallInfo,
    memory: PressureStallInfo,
    io: PressureStallInfo,
}

impl SystemPSI {
    pub fn new() -> SystemPSI {
        SystemPSI {
            cpu: PressureStallInfo::new(),
            memory: PressureStallInfo::new(),
            io: PressureStallInfo::new(),
        }
    }

    pub fn cpu(&self) -> &PressureStallInfo {
        &self.cpu
    }
    pub fn memory(&self) -> &PressureStallInfo {
        &self.memory
    }
    pub fn io(&self) -> &PressureStallInfo {
        &self.io
    }

    fn update_cpu_psi(&mut self, path: PathBuf) -> common::Result<bool> {
        let file_data = fs::read_to_string(&path)?;
        let mut psi_data = PressureStallInfo::from(file_data.as_str());
        psi_data.set_full(None);
        self.cpu = psi_data;
        Ok(true)
    }
    fn update_memory_psi(&mut self, path: PathBuf) -> common::Result<bool> {
        let file_data = fs::read_to_string(&path)?;
        let psi_data = PressureStallInfo::from(file_data.as_str());
        self.memory = psi_data;
        Ok(true)
    }
    fn update_io_psi(&mut self, path: PathBuf) -> common::Result<bool> {
        let file_data = fs::read_to_string(&path)?;
        let psi_data = PressureStallInfo::from(file_data.as_str());
        self.io = psi_data;
        Ok(true)
    }

    #[cfg(not(tarpaulin_include))]
    pub fn update(&mut self) {
        if let Err(e) = self.update_cpu_psi(PathBuf::from("/proc/pressure/cpu")) {
            warn!("[PSI] update cpu psi error: {}", e);
        }
        if let Err(e) = self.update_memory_psi(PathBuf::from("/proc/pressure/memory")) {
            warn!("[PSI] update memory psi error: {}", e);
        }
        if let Err(e) = self.update_io_psi(PathBuf::from("/proc/pressure/io")) {
            warn!("[PSI] update io psi error: {}", e);
        }
    }
}

impl Default for SystemPSI {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests_pressure {
    use super::*;
    use std::env;
    #[test]
    fn test_pressure() {
        let current_path: PathBuf = env::current_dir().unwrap();
        let cpu_path: PathBuf = PathBuf::from(format!(
            "{}/tests/sample/proc_pressure_cpu",
            current_path.to_string_lossy()
        ));
        let mem_path: PathBuf = PathBuf::from(format!(
            "{}/tests/sample/proc_pressure_mem",
            current_path.to_string_lossy()
        ));
        let io_path: PathBuf = PathBuf::from(format!(
            "{}/tests/sample/proc_pressure_io",
            current_path.to_string_lossy()
        ));
        let mut system_psi_instance = SystemPSI::new();
        system_psi_instance
            .update_memory_psi(mem_path)
            .expect("update memory psi error");
        system_psi_instance
            .update_cpu_psi(cpu_path)
            .expect("update cpu psi error");
        system_psi_instance
            .update_io_psi(io_path)
            .expect("update io psi error");

        assert_eq!(system_psi_instance.cpu.get_full().is_none(), true);
        assert_eq!(
            system_psi_instance.cpu.get_some().unwrap().get_avg300(),
            2.62
        );
        assert_eq!(
            system_psi_instance.memory.get_some().unwrap().get_total(),
            503450.0
        );
        assert_eq!(
            system_psi_instance.memory.get_full().unwrap().get_total(),
            136733.0
        );
        assert_eq!(system_psi_instance.io.get_some().unwrap().get_avg60(), 0.11);
        assert_eq!(
            system_psi_instance.io.get_full().unwrap().get_avg300(),
            0.02
        );
    }
}
