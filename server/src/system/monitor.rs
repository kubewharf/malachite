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

use lib::settings;
use lib::system::System;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::ops::{Deref, DerefMut};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub static MONITOR: Lazy<SystemMonitor> = Lazy::new(|| SystemMonitor {
    instance_one: RwLock::new(SystemInstance {
        inner: System::new(),
        nr: 1,
    }),
    instance_two: RwLock::new(SystemInstance {
        inner: System::new(),
        nr: 2,
    }),
    pointer: Arc::new(AtomicBool::new(true)),
    settings: RwLock::new(settings::Settings::new().unwrap()),
});

pub struct SystemMonitor {
    instance_one: RwLock<SystemInstance>,
    instance_two: RwLock<SystemInstance>,
    pointer: Arc<AtomicBool>,
    settings: RwLock<settings::Settings>,
}

impl SystemMonitor {
    pub fn get_monitor_reader(&self) -> &RwLock<SystemInstance> {
        let flag = self.pointer.load(Ordering::SeqCst);
        if flag {
            return &self.instance_one;
        }
        &self.instance_two
    }
    pub fn get_monitor_writer(&self) -> &RwLock<SystemInstance> {
        let flag = self.pointer.load(Ordering::SeqCst);
        if !flag {
            return &self.instance_one;
        }
        &self.instance_two
    }
    pub fn change_pointer(&self) {
        let flag = self.pointer.load(Ordering::SeqCst);
        self.pointer.store(!flag, Ordering::SeqCst);
    }

    pub fn get_settings(&self) -> &RwLock<settings::Settings> {
        &self.settings
    }
}

pub struct SystemInstance {
    inner: System,
    pub nr: u64,
}

impl Deref for SystemInstance {
    type Target = System;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for SystemInstance {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl SystemInstance {
    pub fn update(&mut self, system: System) {
        self.inner = system
    }
}
