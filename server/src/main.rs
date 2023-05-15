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
    use log::info;
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
            thread::spawn(|| {
                update_once();
            });
            sleep(Duration::from_millis(10000));
            MONITOR.change_pointer();
        }
    }

    fn update_once() {
        let mut writer_instance = MONITOR.get_monitor_writer().write();
        info!("[moniter] updating instance {}", writer_instance.nr);
        writer_instance.update(MONITOR.get_monitor_reader().read().clone());

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
    use lib;
    use lib::cgroup as lib_cgroup;
    use rocket::{Build, Rocket};
    use utoipa::OpenApi;
    use utoipa_swagger_ui::SwaggerUi;
    #[launch]
    pub fn start() -> Rocket<Build> {
        #[derive(OpenApi)]
        #[openapi(
            paths(
                //hello,
                system::compute,
                system::network,
                system::io,
                system::memory,
                system::system_event,
                cgroup::get_cgroup_info,
                healthz::health,
            ),
            components(
                schemas(lib_cgroup::CGroup, lib::common::CGroupType, lib_cgroup::SubSystem, lib_cgroup::SubSystemType,
                    lib_cgroup::MemoryCGroup,lib_cgroup::MemoryCGroupV1, lib_cgroup::MemoryCGroupV2, lib_cgroup::MemEventLocalV2,
                    lib_cgroup::MemNumaStatsV2, lib_cgroup::MemStatsV2, lib_cgroup::MemoryCGroupNumaStat,
                    lib_cgroup::CpuCGroup,lib_cgroup::CpuCGroupV1,lib_cgroup::CpuCGroupV2, lib_cgroup::CpuCGroupBasicInfo, lib_cgroup::CpuStatsV2,
                    lib_cgroup::CpuSetCGroup, lib_cgroup::CpuSetCGroupV1, lib_cgroup::CpuSetCGroupV2,
                    lib_cgroup::BlkIOCGroup,lib_cgroup::BlkIOCGroupV1,lib_cgroup::BlkIOCGroupV2,lib_cgroup::BlkIOMaxV2, lib_cgroup::BlkIOStatV2,
                    lib::ffi::WrapperIoLatpcts, lib::ffi::IoPercentLatency, lib::ffi::WrapperFSData, lib::ffi::WrapperBpfProgStat, lib::ffi::WrapperSystemEvent,
                    lib::ffi::WrapperSystemEventFS,lib::ffi::WrapperSystemEventGen,lib::ffi::WrapperSystemEventIO,lib::ffi::WrapperSystemEventMem,lib::ffi::WrapperSystemEventNet,lib::ffi::WrapperSystemEventSched,
                    lib_cgroup::NetCGroup, lib::ffi::WrapperNetData,
                    lib_cgroup::PerfEventCGroup,
                    lib::psi::PressureStallInfo, lib::psi::PSIItem,
                    lib::cpu::NodeVec, lib::cpu::ProcessorCPIData, lib::process::SystemProcessStats,
                    lib::system::LoadAvg, lib::system::LoadAvgOperator, lib::system::BPFProgStats,
                    lib::system::DiskStat, lib::system::DiskUsage,
                    lib::system::NumaNode, lib::system::MemoryInfo, lib::system::ImcChannelInfo,
                    lib::system::SystemEventData,
                    lib::net::NetworkCardTraffic, lib::net::NetInfo,
                    lib::settings::Settings, lib::settings::DataSourceProcFS, lib::settings::DataSourceSysFS,lib::settings::DataSourceCgroupFS,
                    lib::settings::DataSourceBytePerf,lib::settings::DataSourceEBPF, lib::settings::DataSourceSubSys,
                    system::RespCompute, system::RespComputeCpu, system::RespIo, system::RespMemory, system::RespNetwork, system::RespSystemEvent,
                    healthz::Healths, 
                )
            ),
            tags(
                (name = "Malachite", description = "Malachite core OpenAPI.")
            )
        )]
        struct ApiDoc;

        rocket::build()
            .mount(
                "/",
                SwaggerUi::new("/swagger-ui/<_..>")
                    .url("/api-docs/openapi.json", ApiDoc::openapi()),
            )
            //.mount("/", routes![hello])
            .attach(cgroup::cgroup_v1_router())
            .attach(system::system_v1_router())
            .attach(healthz::healthz_v1_router())
            .attach(setting::settings_v1_router())
    }

    /*
    /// hello world123
    ///
    /// hello world456
    #[utoipa::path(
        responses(
            (status = 200, description = "hello test789", body = [MyString])
        )
    )]
    #[get("/hello")]
    fn hello() -> MyString {
        MyString {
            str: String::from("hello world"),
        }
    }

    #[derive(Serialize, ToSchema, Responder, Debug)]
    pub(super) struct MyString {
        str: String,
    }
    */
}

mod init {
    use std::default::Default;
    use std::env;
    use std::ops::Deref;
    use std::panic;

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
