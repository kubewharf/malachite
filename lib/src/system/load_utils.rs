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

//! translate from:
//! https://github.com/torvalds/linux/blob/master/kernel/sched/loadavg.c
//! https://github.com/torvalds/linux/blob/master/include/linux/sched/loadavg.h
//! https://github.com/torvalds/linux/blob/master/fs/proc/loadavg.c
//!
pub(crate) const EXP_1: u64 = 1884;
pub(crate) const EXP_5: u64 = 2014;
pub(crate) const EXP_15: u64 = 2037;
pub(crate) const FSHIFT: u64 = 11;
pub(crate) const FIXED_1: u64 = 1 << FSHIFT;

pub(crate) fn get_avenrun(avenrun: u64, offset: u64, shift: u8) -> u64 {
    (avenrun + offset) << shift
}

pub(crate) fn calc_load(load: u64, exp: u64, active: u64) -> u64 {
    let mut new_load: u64 = load * exp + active * (FIXED_1 - exp);
    if active >= load {
        new_load = new_load + FIXED_1 - 1;
    }
    new_load / FIXED_1
}

pub(crate) fn load_int(x: u64) -> u64 {
    (x) >> FSHIFT
}

pub(crate) fn load_frac(x: u64) -> u64 {
    load_int(((x) & (FIXED_1 - 1)) * 100)
}
