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
use lib::cgroup;
use lib::cgroup::CGroup;
use lib::common::MOUNT_POINT;
use lib::settings;
use rocket::http::Status;
use rocket::serde::json::Json;
use std::path::PathBuf;
use std::thread;

#[get("/groups?<cgroup_user_path>")]
async fn get_cgroup_info(cgroup_user_path: String) -> Result<Json<Resp<cgroup::CGroup>>, Status> {
    let reader_lock = system::MONITOR.get_monitor_reader();
    if let Some(cgroup_info) = reader_lock
        .read()
        .get_cgroups(PathBuf::from(&cgroup_user_path))
    {
        return Ok(Json(Resp::new(cgroup_info.clone())));
    };

    let settings = system::MONITOR.get_settings().read().clone();
    let mut cgroup = CGroup::new(MOUNT_POINT, PathBuf::from(&cgroup_user_path));

    if let Some(ds_settings) = settings.get_data_source(settings::DataSourceType::CgroupFS) {
        if cgroup.update(&*ds_settings).is_err() {
            warn!(
                "[server] new cgroup update error, path= {}",
                cgroup.user_path().display()
            )
        }

        if !cgroup.is_valid() {
            return Err(Status::NotFound);
        }

        let cgroup_tmp = cgroup.clone();
        thread::spawn(move || {
            let sender = system::CHANGE_CHANNEL.get_channel_sender().clone();
            sender
                .send(system::ChangeEvent::CGroupEvent(cgroup_tmp))
                .unwrap();
        })
        .join()
        .unwrap();

        return Ok(Json(Resp::new(cgroup)));
    }

    Err(Status::NotFound)
}

pub fn cgroup_v1_router() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("JSON", |rocket| async {
        rocket.mount("/api/v1/cgroup", routes![get_cgroup_info])
    })
}
