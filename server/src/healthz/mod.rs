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
use crate::system;
use lib::ffi::{ModuleMask, ModuleMaskIDType};
use lib::settings::Settings;
use lib::common::CGroupType;
use rocket::http::Status;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
struct ModuleDetails {
    status: bool,
    mask: u64,
    enable_list: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Hash, ToSchema)]
pub struct Healths {
    status: String,
    ebpf_mask: Vec<String>,
    perf_mask: Vec<String>,
    cgroup_type: CGroupType,
    settings: Option<Settings>,
}

/// malachite health
#[utoipa::path(
    context_path = "/v1/health",
    responses(
        (status = 200, description = "health api", body = [Healths])
    )
)]
#[get("/")]
async fn health() -> Result<Json<Resp<Healths>>, Status> {
    let mask = lib::ffi::wrapper_get_bpf_mask();
    let mask_ids: Vec<ModuleMaskIDType> = Vec::from(ModuleMask::new(mask));
    let ebpf_mask_list: Vec<String> = lib::ffi::BPF_MODULE_MASK_CONFIG.get_names_vec(mask_ids);

    let (_, cur_mask) = lib::ffi::wrapper_byteperf_check_module_health();
    let mask = cur_mask as u64;
    let perf_mask_ids: Vec<ModuleMaskIDType> = Vec::from(ModuleMask::new(mask));
    let perf_mask_list: Vec<String> =
        lib::ffi::PERF_MODULE_MASK_CONFIG.get_names_vec(perf_mask_ids);

    let mut settings = None;
    if let Some(s) = system::MONITOR.get_settings().try_read() {
        settings = Some(s.clone());
    }
    Ok(Json(Resp::new(Healths {
        status: "Ok".to_string(),
        ebpf_mask: ebpf_mask_list,
        perf_mask: perf_mask_list,
        cgroup_type: lib::common::MODULE_LIST.cgroup_type.actual_status(),
        settings,
    })))
}

pub fn healthz_v1_router() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("JSON", |rocket| async {
        rocket.mount("/v1/health", routes![health])
        //rocket.mount("/v1/health/details", routes![health_details])
    })
}
