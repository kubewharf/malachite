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
use crate::process::ProcessStatus;
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};

pub type Pid = usize;

pub struct Process {
    pub(crate) name: String,
    pub(crate) cmd: Vec<String>,
    pub(crate) exe: PathBuf,
    pub(crate) pid: Pid,
    parent: Option<Pid>,
    pub(crate) environ: Vec<String>,
    pub(crate) cwd: PathBuf,
    pub(crate) root: PathBuf,
    pub(crate) memory: u64,
    pub(crate) virtual_memory: u64,
    pub(crate) cpu_usage: f32,
    utime: u64,
    stime: u64,
    old_utime: u64,
    old_stime: u64,
    start_time: u64,
    updated: bool,
    pub(crate) status: ProcessStatus,
    pub tasks: HashMap<Pid, Process>,
    pub(crate) cgroup_path: HashMap<String, PathBuf>,
    pub(crate) stat_file: Option<File>,
    old_read_bytes: u64,
    old_written_bytes: u64,
    read_bytes: u64,
    written_bytes: u64,
}

impl Process {
    fn new(pid: Pid, parent: Option<Pid>, start_time: u64) -> Process {
        Process {
            name: String::with_capacity(20),
            pid,
            parent,
            cmd: Vec::with_capacity(2),
            environ: Vec::with_capacity(10),
            exe: PathBuf::new(),
            cwd: PathBuf::new(),
            root: PathBuf::new(),
            memory: 0,
            virtual_memory: 0,
            cpu_usage: 0.,
            utime: 0,
            stime: 0,
            old_utime: 0,
            old_stime: 0,
            updated: true,
            start_time,
            status: ProcessStatus::Unknown(0),
            tasks: if pid == 0 {
                HashMap::with_capacity(1000)
            } else {
                HashMap::new()
            },
            cgroup_path: HashMap::new(),
            stat_file: None,
            old_read_bytes: 0,
            old_written_bytes: 0,
            read_bytes: 0,
            written_bytes: 0,
        }
    }

    fn pid(&self) -> &Pid {
        &self.pid
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn status(&self) -> ProcessStatus {
        self.status
    }

    fn exe(&self) -> &Path {
        self.exe.as_path()
    }

    fn cmd(&self) -> &[String] {
        &self.cmd
    }

    fn environ(&self) -> &[String] {
        &self.environ
    }

    fn cwd(&self) -> &Path {
        self.cwd.as_path()
    }

    fn root(&self) -> &Path {
        self.root.as_path()
    }

    fn update_time(&mut self, utime: u64, stime: u64) {
        self.old_utime = self.utime;
        self.old_stime = self.stime;
        self.utime = utime;
        self.stime = stime;
    }

    fn update_status(&mut self, string_from_stat_file: &str) {
        self.status = string_from_stat_file
            .chars()
            .next()
            .map(ProcessStatus::from)
            .unwrap_or_else(|| ProcessStatus::Unknown(0));
    }

    fn update_memory(&mut self, vsize: u64, rss: u64) {
        self.memory = rss;
        self.virtual_memory = vsize;
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if self.stat_file.is_some() {
            if let Ok(ref mut x) = unsafe { crate::system::REMAINING_FILES.lock() } {
                **x += 1;
            }
        }
    }
}
