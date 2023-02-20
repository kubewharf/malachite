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
use lib::settings;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket_basicauth::BasicAuth;
use std::thread;

#[post("/", data = "<setting>")]
async fn update_settings(
    auth: BasicAuth,
    setting: Json<settings::Settings>,
) -> Result<Json<Resp<String>>, Status> {
    if auth.username != "malachite-user" || auth.password != "malachite-pswd" {
        return Err(Status::Forbidden);
    }

    thread::spawn(move || {
        let sender = system::CHANGE_CHANNEL.get_channel_sender().clone();
        sender
            .send(system::ChangeEvent::SettingsEvent(Box::new(
                setting.into_inner(),
            )))
            .unwrap();
    })
    .join()
    .unwrap();

    Ok(Json(Resp::new(String::from("success"))))
}

pub fn settings_v1_router() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("JSON", |rocket| async {
        rocket.mount("/v1/settings", routes![update_settings])
    })
}
