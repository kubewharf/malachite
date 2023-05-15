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

use config::{Config, ConfigError, File};
use serde::{Deserialize, Serialize};
use std::env;
use strum::EnumIter;
use utoipa::ToSchema;

pub trait DataSource {
    fn is_enable(&self) -> bool;
    fn get_sub_system(&self, t: SubSystemType) -> Option<Box<dyn SubSystem>>;
}

pub trait SubSystem {
    fn is_enable(&self) -> bool;
}

#[derive(Clone, Eq, Hash, PartialEq, Debug)]
pub enum DataSourceType {
    ProcFS,
    SysFS,
    CgroupFS,
    BytePerf,
    Ebpf,
}

#[derive(Debug, Clone, EnumIter)]
pub enum SubSystemType {
    Cpu,
    Mem,
    Net,
    Storage,
    Unknow,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, PartialOrd, Eq, Hash, Default)]
#[allow(unused)]
struct DataSourceDisabled {}

impl DataSource for DataSourceDisabled {
    fn is_enable(&self) -> bool {
        false
    }

    fn get_sub_system(&self, _t: SubSystemType) -> Option<Box<dyn SubSystem>> {
        Some(Box::new(SubSystemDisabled::default()))
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, PartialOrd, Eq, Hash, Default)]
#[allow(unused)]
struct SubSystemDisabled {}

impl SubSystem for SubSystemDisabled {
    fn is_enable(&self) -> bool {
        false
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, PartialOrd, Eq, Hash, ToSchema)]
#[allow(unused)]
pub struct DataSourceSubSys {
    enable: bool,
    interval: u64,
}

impl SubSystem for DataSourceSubSys {
    fn is_enable(&self) -> bool {
        self.enable
    }
}

impl Default for DataSourceSubSys {
    fn default() -> Self {
        Self {
            enable: true,
            interval: 0,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, PartialOrd, Eq, Hash, ToSchema)]
#[allow(unused)]
pub struct DataSourceProcFS {
    enable: bool,
    cpu: DataSourceSubSys,
    mem: DataSourceSubSys,
    storage: DataSourceSubSys,
    net: DataSourceSubSys,
}

impl Default for DataSourceProcFS {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSourceProcFS {
    fn new() -> Self {
        Self {
            enable: true,
            cpu: DataSourceSubSys::default(),
            mem: DataSourceSubSys::default(),
            storage: DataSourceSubSys::default(),
            net: DataSourceSubSys::default(),
        }
    }
}

impl DataSource for DataSourceProcFS {
    fn is_enable(&self) -> bool {
        self.enable
    }

    fn get_sub_system(&self, t: SubSystemType) -> Option<Box<dyn SubSystem>> {
        match t {
            SubSystemType::Cpu => Some(Box::new(self.cpu.clone())),
            SubSystemType::Mem => Some(Box::new(self.mem.clone())),
            SubSystemType::Net => Some(Box::new(self.net.clone())),
            SubSystemType::Storage => Some(Box::new(self.storage.clone())),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, PartialOrd, Eq, Hash, ToSchema)]
#[allow(unused)]
pub struct DataSourceSysFS {
    enable: bool,
    cpu: DataSourceSubSys,
    mem: DataSourceSubSys,
    storage: DataSourceSubSys,
    net: DataSourceSubSys,
}

impl Default for DataSourceSysFS {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSourceSysFS {
    fn new() -> Self {
        Self {
            enable: true,
            cpu: DataSourceSubSys::default(),
            mem: DataSourceSubSys::default(),
            storage: DataSourceSubSys::default(),
            net: DataSourceSubSys::default(),
        }
    }
}

impl DataSource for DataSourceSysFS {
    fn is_enable(&self) -> bool {
        self.enable
    }

    fn get_sub_system(&self, t: SubSystemType) -> Option<Box<dyn SubSystem>> {
        match t {
            SubSystemType::Cpu => Some(Box::new(self.cpu.clone())),
            SubSystemType::Mem => Some(Box::new(self.mem.clone())),
            SubSystemType::Net => Some(Box::new(self.net.clone())),
            SubSystemType::Storage => Some(Box::new(self.storage.clone())),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, PartialOrd, Eq, Hash, ToSchema)]
#[allow(unused)]
pub struct DataSourceCgroupFS {
    enable: bool,
    cpu: DataSourceSubSys,
    mem: DataSourceSubSys,
    storage: DataSourceSubSys,
    net: DataSourceSubSys,
}

impl Default for DataSourceCgroupFS {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSourceCgroupFS {
    fn new() -> Self {
        Self {
            enable: true,
            cpu: DataSourceSubSys::default(),
            mem: DataSourceSubSys::default(),
            storage: DataSourceSubSys::default(),
            net: DataSourceSubSys::default(),
        }
    }
}

impl DataSource for DataSourceCgroupFS {
    fn is_enable(&self) -> bool {
        self.enable
    }

    fn get_sub_system(&self, t: SubSystemType) -> Option<Box<dyn SubSystem>> {
        match t {
            SubSystemType::Cpu => Some(Box::new(self.cpu.clone())),
            SubSystemType::Mem => Some(Box::new(self.mem.clone())),
            SubSystemType::Net => Some(Box::new(self.net.clone())),
            SubSystemType::Storage => Some(Box::new(self.storage.clone())),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, PartialOrd, Eq, Hash, ToSchema)]
#[allow(unused)]
pub struct DataSourceBytePerf {
    enable: bool,
    cpu: DataSourceSubSys,
    mem: DataSourceSubSys,
    storage: DataSourceSubSys,
    net: DataSourceSubSys,
}

impl Default for DataSourceBytePerf {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSourceBytePerf {
    fn new() -> Self {
        Self {
            enable: true,
            cpu: DataSourceSubSys::default(),
            mem: DataSourceSubSys::default(),
            storage: DataSourceSubSys::default(),
            net: DataSourceSubSys::default(),
        }
    }
}

impl DataSource for DataSourceBytePerf {
    fn is_enable(&self) -> bool {
        self.enable
    }

    fn get_sub_system(&self, t: SubSystemType) -> Option<Box<dyn SubSystem>> {
        match t {
            SubSystemType::Cpu => Some(Box::new(self.cpu.clone())),
            SubSystemType::Mem => Some(Box::new(self.mem.clone())),
            SubSystemType::Net => Some(Box::new(self.net.clone())),
            SubSystemType::Storage => Some(Box::new(self.storage.clone())),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, PartialOrd, Eq, Hash, ToSchema)]
#[allow(unused)]
pub struct DataSourceEBPF {
    enable: bool,
    cpu: DataSourceSubSys,
    mem: DataSourceSubSys,
    storage: DataSourceSubSys,
    net: DataSourceSubSys,
}

impl Default for DataSourceEBPF {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSourceEBPF {
    fn new() -> Self {
        Self {
            enable: true,
            cpu: DataSourceSubSys::default(),
            mem: DataSourceSubSys::default(),
            storage: DataSourceSubSys::default(),
            net: DataSourceSubSys::default(),
        }
    }
}

impl DataSource for DataSourceEBPF {
    fn is_enable(&self) -> bool {
        self.enable
    }

    fn get_sub_system(&self, t: SubSystemType) -> Option<Box<dyn SubSystem>> {
        match t {
            SubSystemType::Cpu => Some(Box::new(self.cpu.clone())),
            SubSystemType::Mem => Some(Box::new(self.mem.clone())),
            SubSystemType::Net => Some(Box::new(self.net.clone())),
            SubSystemType::Storage => Some(Box::new(self.storage.clone())),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, Eq, Hash, ToSchema)]
#[allow(unused)]
pub struct Settings {
    enable: bool,
    proc: DataSourceProcFS,
    sys: DataSourceSysFS,
    cgroup: DataSourceCgroupFS,
    byteperf: DataSourceBytePerf,
    ebpf: DataSourceEBPF,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enable: true,
            proc: DataSourceProcFS::default(),
            sys: DataSourceSysFS::default(),
            cgroup: DataSourceCgroupFS::default(),
            byteperf: DataSourceBytePerf::default(),
            ebpf: DataSourceEBPF::default(),
        }
    }
}

impl Settings {
    pub fn new() -> Result<Settings, ConfigError> {
        let mut p = env::current_exe().unwrap();
        p.pop();
        p.push("static/config/default");
        let s = Config::builder()
            .add_source(File::with_name(&p.into_os_string().into_string().unwrap()))
            .build()?;

        s.try_deserialize()
    }

    pub fn is_enable(&self) -> bool {
        self.enable
    }

    pub fn get_data_source(&self, t: DataSourceType) -> Option<Box<dyn DataSource>> {
        match t {
            DataSourceType::ProcFS => Some(Box::new(self.proc.clone())),
            DataSourceType::SysFS => Some(Box::new(self.sys.clone())),
            DataSourceType::CgroupFS => Some(Box::new(self.cgroup.clone())),
            DataSourceType::BytePerf => Some(Box::new(self.byteperf.clone())),
            DataSourceType::Ebpf => Some(Box::new(self.ebpf.clone())),
        }
    }

    pub fn update(&mut self, new_settings: Settings) {
        self.enable = new_settings.enable;
        self.proc = new_settings.proc;
        self.sys = new_settings.sys;
        self.cgroup = new_settings.cgroup;
        self.byteperf = new_settings.byteperf;
        self.ebpf = new_settings.ebpf;
    }
}

pub fn get_disabeld_data_source() -> Box<dyn DataSource> {
    Box::new(DataSourceDisabled::default())
}
