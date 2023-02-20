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

use crate::common::Resp;
use crate::system::response_mod::{RespCompute, RespComputeCpu, RespIo, RespMemory, RespNetwork};
use crate::system::MONITOR;
use lib::system::SystemEventData;
use rocket::http::Status;
use rocket::serde::json::Json;
use std::ops::Deref;

#[get("/compute")]
async fn compute() -> Result<Json<Resp<RespCompute>>, Status> {
    let reader_lock = MONITOR.get_monitor_reader();
    let load = reader_lock.read().get_load().clone();
    let process_stats = reader_lock.read().get_process_stats().clone();
    let cpu_info = reader_lock
        .read()
        .get_processors()
        .iter()
        .map({
            |(k, v)| RespComputeCpu {
                name: k.clone(),
                cpu_usage: v.get_cpu_usage(),
                cpu_sys_usage: v.get_cpu_sys_usage(),
                cpu_iowait_ratio: v.get_iowait_ratio(),
                cpu_sched_wait: v.get_sched_wait(),
                cpi_data: v.get_cpi_data().as_ref().cloned(),
            }
        })
        .collect::<Vec<RespComputeCpu>>();
    let global_cpu_info = reader_lock
        .read()
        .get_processors()
        .get_global_processor()
        .clone();
    let global_cpu = RespComputeCpu {
        name: global_cpu_info.name().to_string(),
        cpu_usage: global_cpu_info.get_cpu_usage(),
        cpu_sys_usage: global_cpu_info.get_cpu_sys_usage(),
        cpu_iowait_ratio: global_cpu_info.get_iowait_ratio(),
        cpu_sched_wait: global_cpu_info.get_sched_wait(),
        cpi_data: global_cpu_info.get_cpi_data().as_ref().cloned(),
    };
    let pressure = reader_lock.read().get_system_pressure().clone();
    let cpu_pressure = pressure.map(|x| *x.cpu());

    let stats = reader_lock.read().get_bpf_prog_stats().clone();

    Ok(Json(Resp::new(RespCompute {
        load,
        cpu: cpu_info,
        global_cpu,
        pressure: cpu_pressure,
        process_stats,
        bpf_prog_stats: Some(stats),
    })))
}

#[get("/network")]
async fn network() -> Result<Json<Resp<RespNetwork>>, Status> {
    let reader_lock = MONITOR.get_monitor_reader();
    let tcp_info = reader_lock.read().get_net_info().clone();
    let network_card_info = reader_lock.read().get_net_traffic().deref().clone();

    Ok(Json(Resp::new(RespNetwork {
        networkcard: network_card_info,
        tcp: tcp_info,
    })))
}

#[get("/io")]
async fn io() -> Result<Json<Resp<RespIo>>, Status> {
    let reader_lock = MONITOR.get_monitor_reader();
    let disk_io = reader_lock.read().get_disk_io().get_stats().clone();
    let disk_usage = reader_lock.read().get_disk_io().get_usage().clone();
    let pressure = reader_lock.read().get_system_pressure().clone();
    let io_pressure = pressure.map(|x| *x.io());
    Ok(Json(Resp::new(RespIo {
        disk_io,
        disk_usage,
        pressure: io_pressure,
    })))
}

#[get("/memory")]
async fn memory() -> Result<Json<Resp<RespMemory>>, Status> {
    let reader_lock = MONITOR.get_monitor_reader();
    let memory_info = reader_lock.read().get_memory_info().clone();
    let numa_info = reader_lock
        .read()
        .get_system_device_nodes()
        .get_nodes()
        .clone();
    let pressure = reader_lock.read().get_system_pressure().clone();
    let mem_pressure = pressure.map(|x| *x.memory());
    Ok(Json(Resp::new(RespMemory {
        system: memory_info,
        pressure: mem_pressure,
        numa: numa_info,
    })))
}

#[get("/system_event")]
async fn system_event() -> Result<Json<Resp<SystemEventData>>, Status> {
    let reader_lock = MONITOR.get_monitor_reader();
    let system_event = reader_lock.read().get_system_event().clone();
    Ok(Json(Resp::new(system_event)))
}

// #[get("/storage/", format = "json")]
// async fn storage() -> Result<Json<Resp<>>, Status> {
//     Ok(Json(resp))
// }

pub fn system_v1_router() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("JSON", |rocket| async {
        rocket.mount(
            "/api/v1/system",
            routes![compute, network, io, memory, system_event],
        )
    })
}
