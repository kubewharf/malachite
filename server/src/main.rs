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

mod cgroup;
mod common;
mod healthz;
mod setting;
mod system;

#[macro_use]
extern crate rocket;

mod monitor {
    use crate::system::{ChangeEvent, CHANGE_CHANNEL, MONITOR};
    use lib::settings;
    use std::ops::DerefMut;
    use std::process;
    use std::thread::{self, sleep};
    use std::time::Duration;

    fn update() {
        let start_settings = MONITOR.get_settings().read().clone();
        thread::spawn(move || {
            let sender = CHANGE_CHANNEL.get_channel_sender().clone();
            sender
                .send(ChangeEvent::SettingsEvent(Box::new(start_settings)))
                .unwrap();
        })
        .join()
        .unwrap();

        loop {
            let writer_lock = MONITOR.get_monitor_writer();
            let reader_lock = MONITOR.get_monitor_reader();

            {
                let mut writer_instance = writer_lock.write();
                info!("[moniter] updating instance {}", writer_instance.nr);
                writer_instance.update(reader_lock.read().clone());

                let mut new_settings = None;
                loop {
                    let receiver = CHANGE_CHANNEL.get_channel_receiver();
                    if receiver.is_empty() {
                        break;
                    }
                    let msg = receiver.try_recv().unwrap();
                    info!(
                        "[monitor] instance {}, receive {:?}",
                        writer_instance.nr, msg
                    );
                    match msg {
                        ChangeEvent::CGroupEvent(cgroup) => {
                            writer_instance.insert_cgroups_item(cgroup).unwrap();
                        }
                        ChangeEvent::SettingsEvent(s) => {
                            new_settings = Some(s);
                        }
                    }
                }

                if let Some(s) = new_settings {
                    info!("switch settings");
                    MONITOR
                        .get_settings()
                        .write()
                        .deref_mut()
                        .update(*s.clone());
                    writer_instance.switch(&s);
                    //settings.Save();
                }

                let settings = MONITOR.get_settings().read().clone();
                writer_instance.refresh(&settings);
                info!("[monitor] update instance {} done", writer_instance.nr);
            }

            MONITOR.change_pointer();
            sleep(Duration::from_millis(10000));
        }
    }

    fn monitor() {
        let header = thread::Builder::new()
            .name("monitor".into())
            .spawn(update)
            .unwrap();
        match header.join() {
            Ok(_r) => {
                warn!("[monitor] monitor over");
            }
            Err(_e) => {
                clean();
                process::exit(1);
            }
        };
    }

    fn enable_modules() {
        info!(
            "[Server] submodule list: cgroup_v2= {:?}\n",
            lib::common::MODULE_LIST.cgroup_type
        )
    }

    pub fn daemon() {
        enable_modules();
        let _handler = thread::Builder::new()
            .name("monitor-daemon".into())
            .spawn(monitor);
    }

    pub fn clean() {
        let mut ebpf_disable = false;
        let mut perf_disable = false;
        let settings = MONITOR.get_settings().read().clone();
        if !settings.is_enable() {
            ebpf_disable = true;
            perf_disable = true;
        }

        if let Some(ds_settings) = settings.get_data_source(settings::DataSourceType::Ebpf) {
            if !ds_settings.is_enable() {
                ebpf_disable = true;
            }
        }

        if let Some(ds_settings) = settings.get_data_source(settings::DataSourceType::BytePerf) {
            if !ds_settings.is_enable() {
                perf_disable = true;
            }
        }

        if ebpf_disable {
            lib::ffi::wrapper_free_bpf();
            info!("[clean] release bpf resource");
        }

        if perf_disable {
            lib::ffi::wrapper_byteperf_destroy_malachite();
            info!("[clean] release byteperf resource");
        }
    }
}

mod web_server {
    use crate::{cgroup, healthz, setting, system};
    use rocket::{Build, Rocket};
    #[launch]
    pub fn start() -> Rocket<Build> {
        rocket::build()
            .attach(cgroup::cgroup_v1_router())
            .attach(system::system_v1_router())
            .attach(healthz::healthz_v1_router())
            .attach(setting::settings_v1_router())
    }
}

mod init {
    use std::default::Default;
    use std::ops::Deref;
    use std::panic;
    use std::env;

    pub fn init() {
        logs();
        panic_hook();
    }

    fn logs() {
        let mut p = env::current_exe().unwrap();
        p.pop();
        p.push("static/config/log4rs.toml");
        log4rs::init_file(p, Default::default()).unwrap();
    }

    fn panic_hook() {
        panic::set_hook(Box::new(|panic_info| {
            let (filename, line) = panic_info
                .location()
                .map(|loc| (loc.file(), loc.line()))
                .unwrap_or(("<unknown>", 0));
            let cause = panic_info
                .payload()
                .downcast_ref::<String>()
                .map(String::deref);
            let cause = cause.unwrap_or_else(|| {
                panic_info
                    .payload()
                    .downcast_ref::<&str>()
                    .copied()
                    .unwrap_or("<cause unknown>")
            });
            error!("[Panic] {}:{}: {}", filename, line, cause);
        }));
    }
}
fn main() {
    init::init();
    monitor::daemon();
    web_server::main();
    monitor::clean();
}
