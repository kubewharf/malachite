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

// use clap::{App, AppSettings, Arg};
//
// fn main() {
//     let matches = App::new("Malachite Ctl")
//         .about("Malachite Control Manager")
//         .version("0.0.1")
//         .setting(AppSettings::ArgRequiredElseHelp)
//         .author("lihuaxuan <lihuaxuan@bytedance.com>")
//         .subcommand(
//             App::new("process info")
//                 .short_flag('p')
//                 .long_flag("process")
//                 .about("Get Process cpu/memory basic info")
//                 .arg(
//                     Arg::new("ebpf")
//                         .long("ebpf")
//                         .takes_value(true)
//                         .about("Get Process ebpf info"),
//                 ),
//         )
//         .get_matches();
//     match matches.subcommand() {
//         Some(("", process_maches)) => {
//             if process_maches.is_present("ebpf") {
//                 // TODO: mcctl -p $pid ebpf xxx
//                 println!("mcctl -p $pid ebpf xxx")
//             } else {
//                 println!("mcctl -p $pid -a")
//             }
//         }
//         _ => unreachable!(),
//     }
// }
fn main() {}
