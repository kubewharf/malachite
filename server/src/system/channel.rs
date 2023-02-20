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

use crossbeam::channel::{unbounded, Receiver, Sender};
use lib::cgroup::CGroup;
use lib::settings;
use once_cell::sync::Lazy;

#[derive(Debug)]
pub enum ChangeEvent {
    CGroupEvent(CGroup),
    SettingsEvent(Box<settings::Settings>),
}

pub static CHANGE_CHANNEL: Lazy<ChangeChannel> = Lazy::new(|| {
    let (sender, receiver): (Sender<ChangeEvent>, Receiver<ChangeEvent>) = unbounded();
    ChangeChannel { sender, receiver }
});

pub struct ChangeChannel {
    sender: Sender<ChangeEvent>,
    receiver: Receiver<ChangeEvent>,
}

impl ChangeChannel {
    pub fn get_channel_sender(&self) -> &Sender<ChangeEvent> {
        &self.sender
    }
    pub fn get_channel_receiver(&self) -> &Receiver<ChangeEvent> {
        &self.receiver
    }
}

// static CHANGE_EVENT_CHANNEL: Lazy<(Sender<ChangeEvent>, Receiver<ChangeEvent>)> =
//     Lazy::new(move || unbounded());
//
// pub fn get_change_channel_sender() -> &'static Sender<ChangeEvent> {
//     &CHANGE_EVENT_CHANNEL.0
// }
//
// pub fn get_change_channel_receiver() -> &'static Receiver<ChangeEvent> {
//     &CHANGE_EVENT_CHANNEL.1
// }
