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
use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::ops::{Deref, DerefMut, Sub};
use std::path::PathBuf;

///  name：指示CPU核
///  user：用户态花费的时间
///  nice：nice值为负的进程在用户态所占用的CPU时间
///  system：内核态占用的CPU时间
///  idle：空闲时间
///  iowait：磁盘IO等待的时间
///  irq：硬中断占用的时间
///  softirq：软中断占用的时间
///  steal：如果当前系统运行在虚拟化环境中，则可能会有时间片运行在操作系统上，这个值指的是运行其他操作系统花费的时间
///  guest：操作系统运行虚拟CPU花费的时间
///  guest_nice：运行一个带nice值的guest花费的时间
#[derive(Default, Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ProcessorInfo {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
    steal: u64,
    guest: u64,
    guest_nice: u64,
}

impl ProcessorInfo {
    pub fn set(&mut self, p: &ProcessorInfo) {
        self.user = p.user;
        self.nice = p.nice;
        self.system = p.system;
        self.idle = p.idle;
        self.iowait = p.iowait;
        self.irq = p.irq;
        self.softirq = p.softirq;
        self.steal = p.steal;
        self.guest = p.guest;
        self.guest_nice = p.guest_nice;
    }

    /// Return work time.
    pub fn work_time(&self) -> u64 {
        self.user + self.nice + self.system + self.irq + self.softirq + self.steal
    }

    /// Return sys time.
    pub fn sys_time(&self) -> u64 {
        self.system + self.irq + self.softirq
    }

    /// Return total time.
    pub fn total_time(&self) -> u64 {
        // `guest` is already included in `user`
        // `guest_nice` is already included in `nice`
        self.work_time() + self.idle + self.iowait
    }
}

impl Sub for ProcessorInfo {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        Self {
            user: self.user - other.user,
            nice: self.nice - other.nice,
            system: self.system - other.system,
            idle: self.idle - other.idle,
            iowait: self.iowait - other.iowait,
            irq: self.irq - other.irq,
            softirq: self.softirq - other.softirq,
            steal: self.steal - other.steal,
            guest: self.guest - other.guest,
            guest_nice: self.guest_nice - other.guest_nice,
        }
    }
}

#[derive(Default, Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ProcessorSchedStatData {
    sched_wait_data: u64,
    last_update_time: u64, // as_secs()
}

impl ProcessorSchedStatData {
    pub fn update(&mut self, sched_wait: u64, now: u64) {
        self.sched_wait_data = sched_wait;
        self.last_update_time = now
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessorCPIData {
    cpi: f64,
    instructions: f64,
    cycles: f64,
    l3_misses: f64,
    utilization: f64,
}

impl Default for ProcessorCPIData {
    fn default() -> Self {
        ProcessorCPIData {
            cpi: 0.0,
            instructions: 0.0,
            cycles: 0.0,
            l3_misses: 0.0,
            utilization: 0.0,
        }
    }
}

impl ProcessorCPIData {
    pub fn update(
        &mut self,
        cpi: f64,
        instructions: f64,
        cycles: f64,
        l3_misses: f64,
        utilization: f64,
    ) {
        self.cpi = cpi;
        self.instructions = instructions;
        self.cycles = cycles;
        self.l3_misses = l3_misses;
        self.utilization = utilization
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Processor {
    name: String, // global: all , other: id
    old_processor_info: ProcessorInfo,
    new_processor_info: ProcessorInfo,
    delta_processor_info: ProcessorInfo,
    processor_sched_stat_info: ProcessorSchedStatData,
    processor_cpi_data: Option<ProcessorCPIData>,
    sched_wait: f32,
    cpu_usage_ratio: f32,
    cpu_sys_usage_ratio: f32,
    iowait_ratio: f32,
    total_time: u64,
    old_total_time: u64,
}

impl Processor {
    pub(crate) fn new(name: String) -> Processor {
        Processor {
            name,
            old_processor_info: ProcessorInfo::default(),
            new_processor_info: ProcessorInfo::default(),
            delta_processor_info: ProcessorInfo::default(),
            processor_sched_stat_info: ProcessorSchedStatData::default(),
            processor_cpi_data: None,
            sched_wait: 0f32,
            cpu_usage_ratio: 0f32,
            cpu_sys_usage_ratio: 0f32,
            iowait_ratio: 0f32,
            total_time: 0,
            old_total_time: 0,
        }
    }
    pub(crate) fn update_processor_sched_stat(&mut self, new_sched_wait_data: u64) {
        let now = get_secs_since_epoch();
        debug!(
            "[processor] processor sched stat: {:?},{:?}, now: {:?}",
            self, new_sched_wait_data, now
        );
        let last_sched_stat_update_time = self.processor_sched_stat_info.last_update_time;
        if last_sched_stat_update_time != 0 {
            let diff = now - last_sched_stat_update_time;
            self.sched_wait = (new_sched_wait_data - self.processor_sched_stat_info.sched_wait_data)
                as f32
                / diff as f32
                / 1000000_f32;
        }
        self.processor_sched_stat_info
            .update(new_sched_wait_data, now);
    }
    pub(crate) fn update_cpu_cpi(
        &mut self,
        cpi: f64,
        instructions: f64,
        cycles: f64,
        l3_misses: f64,
        utilization: f64,
    ) {
        if self.processor_cpi_data.is_none() {
            self.processor_cpi_data = Some(ProcessorCPIData::default())
        }
        self.processor_cpi_data.as_mut().unwrap().update(
            cpi,
            instructions,
            cycles,
            l3_misses,
            utilization,
        )
    }

    pub(crate) fn reset_cpu_cpi(&mut self) {
        self.processor_cpi_data = None
    }

    pub(crate) fn update_processor_info(&mut self, p: &ProcessorInfo) {
        self.old_processor_info = self.new_processor_info;
        self.new_processor_info.set(p);
        self.delta_processor_info = self.new_processor_info - self.old_processor_info;

        self.total_time = self.new_processor_info.total_time();
        self.old_total_time = self.old_processor_info.total_time();

        let work_time =
            if self.new_processor_info.work_time() >= self.old_processor_info.work_time() {
                self.new_processor_info.work_time() - self.old_processor_info.work_time()
            } else {
                u64::MAX - self.old_processor_info.work_time() + self.new_processor_info.work_time()
            };

        let sys_time = if self.new_processor_info.sys_time() >= self.old_processor_info.sys_time() {
            self.new_processor_info.sys_time() - self.old_processor_info.sys_time()
        } else {
            u64::MAX - self.old_processor_info.sys_time() + self.new_processor_info.sys_time()
        };

        let total_time = if self.total_time >= self.old_total_time {
            self.total_time - self.old_total_time
        } else {
            u64::MAX - self.old_total_time + self.total_time
        };

        self.cpu_usage_ratio = work_time as f32 / total_time as f32 * 100.;
        if self.cpu_usage_ratio > 100. {
            self.cpu_usage_ratio = 100.;
        }

        self.cpu_sys_usage_ratio = sys_time as f32 / total_time as f32 * 100.;
        if self.cpu_sys_usage_ratio > 100. {
            self.cpu_sys_usage_ratio = 100.;
        }

        self.iowait_ratio =
            self.delta_processor_info.iowait as f32 / self.delta_processor_info.total_time() as f32;
    }

    pub fn get_cpu_usage(&self) -> f32 {
        self.cpu_usage_ratio
    }

    pub fn get_cpu_sys_usage(&self) -> f32 {
        self.cpu_sys_usage_ratio
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn get_iowait_ratio(&self) -> f32 {
        self.iowait_ratio
    }
    pub fn get_sched_wait(&self) -> f32 {
        self.sched_wait
    }
    pub fn get_cpi_data(&self) -> &Option<ProcessorCPIData> {
        &self.processor_cpi_data
    }
    pub fn reset(&mut self) {
        *self = Self::new(self.name.to_string());
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct SystemProcessorInfo {
    inner: HashMap<String, Processor>,
    global_processor: Processor,
}

#[cfg(not(tarpaulin_include))]
impl Default for SystemProcessorInfo {
    fn default() -> Self {
        SystemProcessorInfo::new(PathBuf::from("/proc/cpuinfo"))
    }
}
impl Deref for SystemProcessorInfo {
    type Target = HashMap<String, Processor>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for SystemProcessorInfo {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl SystemProcessorInfo {
    pub fn new(cpu_info_file_path: PathBuf) -> SystemProcessorInfo {
        let processor_nums: usize = {
            let data = fs::read_to_string(&cpu_info_file_path).unwrap();
            #[cfg(any(target_arch = "aarch64"))]
            let num = data
                .split('\n')
                .filter(|line| line.starts_with("processor"))
                .count();
            #[cfg(target_arch = "x86_64")]
            let num = data
                .split('\n')
                .filter(|line| line.starts_with("physical id"))
                .count();
            num
        };

        let processors: HashMap<String, Processor> = (0..processor_nums)
            .into_iter()
            .map(|id| (format!("cpu{}", id), Processor::new(id.to_string())))
            .collect::<HashMap<_, _>>();

        SystemProcessorInfo {
            inner: processors,
            global_processor: Processor::new("cpu".to_owned()),
        }
    }

    pub fn get_global_processor(&self) -> &Processor {
        &self.global_processor
    }

    #[cfg(not(tarpaulin_include))]
    pub fn refresh(&mut self) {
        self.refresh_processor_stat(PathBuf::from("/proc/stat"));
        self.refresh_processor_sched_wait(PathBuf::from("/proc/schedstat"));
    }

    pub fn refresh_processor_sched_wait(&mut self, path: PathBuf) {
        let file_data = fs::read_to_string(&path).unwrap();

        file_data.split('\n').skip(2).step_by(2).for_each(|line| {
            let mut iter = line.split_whitespace();
            if let Some(cpu_name) = iter.next() {
                // $cpu_name 之后第 8 列是 sched wait 的值
                if let Some(sched_wait) = iter.nth(7) {
                    if let Some(p) = self.get_mut(cpu_name) {
                        p.update_processor_sched_stat(sched_wait.parse::<u64>().unwrap())
                    }
                }
            }
        });
    }
    pub fn reset_processor_sched_wait(&mut self, path: PathBuf) {
        let file_data = fs::read_to_string(&path).unwrap();

        file_data.split('\n').skip(2).step_by(2).for_each(|line| {
            let mut iter = line.split_whitespace();
            if let Some(cpu_name) = iter.next() {
                // $cpu_name 之后第 8 列是 sched wait 的值
                if let Some(_sched_wait) = iter.nth(7) {
                    if let Some(p) = self.get_mut(cpu_name) {
                        p.update_processor_sched_stat(0)
                    }
                }
            }
        });
    }
    pub fn refresh_processor_stat(&mut self, path: PathBuf) {
        let file_data = fs::read_to_string(&path).unwrap();
        file_data
            .split('\n')
            .filter(|s| s.starts_with("cpu"))
            .for_each(|line| {
                let mut iter = line.split_whitespace();
                if let Some(cpu_name) = iter.next() {
                    let p = if cpu_name.eq("cpu") {
                        &mut self.global_processor
                    } else {
                        self.get_mut(cpu_name).unwrap()
                    };
                    let info = &ProcessorInfo {
                        user: iter.next().unwrap().parse::<u64>().unwrap(),
                        nice: iter.next().unwrap().parse::<u64>().unwrap(),
                        system: iter.next().unwrap().parse::<u64>().unwrap(),
                        idle: iter.next().unwrap().parse::<u64>().unwrap(),
                        iowait: iter.next().unwrap().parse::<u64>().unwrap(),
                        irq: iter.next().unwrap().parse::<u64>().unwrap(),
                        softirq: iter.next().unwrap().parse::<u64>().unwrap(),
                        steal: iter.next().unwrap().parse::<u64>().unwrap(),
                        guest: iter.next().unwrap().parse::<u64>().unwrap(),
                        guest_nice: iter.next().unwrap().parse::<u64>().unwrap(),
                    };

                    p.update_processor_info(info)
                }
            });
    }

    pub fn reset_processor_stat(&mut self, path: PathBuf) {
        let file_data = fs::read_to_string(&path).unwrap();
        file_data
            .split('\n')
            .filter(|s| s.starts_with("cpu"))
            .for_each(|line| {
                let mut iter = line.split_whitespace();
                if let Some(cpu_name) = iter.next() {
                    let p = if cpu_name.eq("cpu") {
                        &mut self.global_processor
                    } else {
                        self.get_mut(cpu_name).unwrap()
                    };
                    p.reset();
                }
            });
    }

    pub fn reset(&mut self) {
        self.reset_processor_stat(PathBuf::from("/proc/stat"));
        self.reset_processor_sched_wait(PathBuf::from("/proc/schedstat"));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialOrd, PartialEq, Eq, Default)]
pub struct NodeVec {
    meta: String,
    inner: Vec<usize>,
}

impl Deref for NodeVec {
    type Target = Vec<usize>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for NodeVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl NodeVec {
    pub fn new() -> NodeVec {
        NodeVec {
            meta: String::with_capacity(1),
            inner: Vec::with_capacity(1),
        }
    }
    pub fn meta(&self) -> &String {
        &self.meta
    }
}

impl From<String> for NodeVec {
    fn from(str: String) -> Self {
        let node_vec: Vec<usize> = str
            .split(',')
            .map(|item| {
                let mut iter = item.split('-');
                let upper_bound = iter.next();
                match iter.next() {
                    None => vec![upper_bound.unwrap().parse::<usize>().unwrap()],
                    Some(lower_bound) => (upper_bound.unwrap().parse::<usize>().unwrap()
                        ..lower_bound.parse::<usize>().unwrap() + 1)
                        .collect::<Vec<usize>>(),
                }
            })
            .collect::<Vec<Vec<usize>>>()
            .concat();

        NodeVec {
            meta: str,
            inner: node_vec,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_node_vec() {
        let input = "1-4,6,8-10";
        let node_vec = NodeVec::from(input.to_string());
        assert_eq!(node_vec.meta, input.to_string());
        let correct = vec![1, 2, 3, 4, 6, 8, 9, 10]
            .into_iter()
            .collect::<Vec<usize>>();
        assert_eq!(node_vec.inner, correct);
    }

    #[test]
    fn test_processor_info() {
        let mut info = ProcessorInfo::default();
        let p = &ProcessorInfo {
            user: 51412878,
            nice: 1465,
            system: 43481807,
            idle: 4013677223,
            iowait: 8308571,
            irq: 0,
            softirq: 2255670,
            steal: 3344491,
            guest: 1,
            guest_nice: 2,
        };
        info.set(p);
        assert_eq!(info.work_time(), 100496311);
        assert_eq!(info.total_time(), 4122482105);
    }

    #[test]
    fn test_processor_sched_stat() {
        let mut instance = ProcessorSchedStatData::default();
        let now = get_secs_since_epoch();
        instance.update(264000, now);
        assert_eq!(instance.sched_wait_data, 264000);
    }

    #[test]
    fn test_system_processor_info() {
        let current_path: PathBuf = env::current_dir().unwrap();
        let mut system_processor_info = SystemProcessorInfo::new(PathBuf::from(format!(
            "{}/tests/sample/proc_cpu_info",
            current_path.to_string_lossy()
        )));
        let proc_stat_file = format!("{}/tests/sample/proc_stat", current_path.to_string_lossy());
        let proc_schedstat_file = PathBuf::from(format!(
            "{}/tests/sample/proc_schedstat",
            current_path.to_string_lossy()
        ));

        system_processor_info.refresh_processor_stat(PathBuf::from(&proc_stat_file));
        system_processor_info.refresh_processor_sched_wait(proc_schedstat_file);

        assert_eq!(system_processor_info.inner.len(), 8 as usize);

        let mut correct_global_processor_info = ProcessorInfo::default();
        let p = &ProcessorInfo {
            user: 364231878,
            nice: 10030,
            system: 314580781,
            idle: 32256231680,
            iowait: 44695443,
            irq: 0,
            softirq: 4911161,
            steal: 24544295,
            guest: 0,
            guest_nice: 0,
        };
        correct_global_processor_info.set(p);

        assert_eq!(
            system_processor_info.global_processor.new_processor_info,
            correct_global_processor_info
        );

        let mut correct_processor_6_info = ProcessorInfo::default();
        let p = &ProcessorInfo {
            user: 42822045,
            nice: 421,
            system: 37939328,
            idle: 4034085670,
            iowait: 8416881,
            irq: 0,
            softirq: 103893,
            steal: 2966388,
            guest: 0,
            guest_nice: 0,
        };
        correct_processor_6_info.set(p);

        assert_eq!(
            system_processor_info
                .get("cpu6")
                .unwrap()
                .new_processor_info,
            correct_processor_6_info
        );
        assert_eq!(
            system_processor_info
                .get("cpu6")
                .unwrap()
                .processor_sched_stat_info
                .sched_wait_data,
            302954360878359
        );
        assert_eq!(
            system_processor_info.get("cpu6").unwrap().get_cpu_usage(),
            2.0316353
        );

        system_processor_info.refresh_processor_stat(PathBuf::from(&proc_stat_file));

        assert_eq!(
            system_processor_info
                .get("cpu6")
                .unwrap()
                .old_processor_info,
            correct_processor_6_info
        );
    }
}
