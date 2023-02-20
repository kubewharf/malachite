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

use crate::system::load_utils::*;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct LoadAvgOperator {
    one: u64,
    five: u64,
    fifteen: u64,
}

impl LoadAvgOperator {
    fn new() -> LoadAvgOperator {
        LoadAvgOperator {
            one: 0,
            five: 0,
            fifteen: 0,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LoadAvg {
    /// load within one minute.
    pub one: f64,
    /// load within five minutes.
    pub five: f64,
    /// load within fifteen minutes.
    pub fifteen: f64,
    operator: LoadAvgOperator,
}

impl Default for LoadAvg {
    fn default() -> Self {
        LoadAvg::new()
    }
}

impl LoadAvg {
    pub fn new() -> LoadAvg {
        LoadAvg {
            one: 0.0,
            five: 0.0,
            fifteen: 0.0,
            operator: LoadAvgOperator::new(),
        }
    }

    #[cfg(not(tarpaulin_include))]
    pub fn refresh_system_load(&mut self) {
        self.read_load_avg(PathBuf::from("/proc/loadavg"));
    }

    #[cfg(not(tarpaulin_include))]
    pub fn calc_load(&mut self, active: u64) {
        let mut active_tasks = 0;
        if active > 0 {
            active_tasks = active * FIXED_1
        }

        // update operator
        self.operator.one = calc_load(self.operator.one, EXP_1, active_tasks);
        self.operator.five = calc_load(self.operator.five, EXP_5, active_tasks);
        self.operator.fifteen = calc_load(self.operator.fifteen, EXP_15, active_tasks);

        // get load from operator
        let offset = FIXED_1 / 200;
        let one = get_avenrun(self.operator.one, offset, 0);
        let five = get_avenrun(self.operator.five, offset, 0);
        let fifteen = get_avenrun(self.operator.fifteen, offset, 0);

        self.one = format!("{}.{}", load_int(one), load_frac(one))
            .parse()
            .unwrap();
        self.five = format!("{}.{}", load_int(five), load_frac(five))
            .parse()
            .unwrap();
        self.fifteen = format!("{}.{}", load_int(fifteen), load_frac(fifteen))
            .parse()
            .unwrap();
    }

    fn read_load_avg(&mut self, path: PathBuf) {
        let mut s = String::new();
        if File::open(&path)
            .and_then(|mut f| f.read_to_string(&mut s))
            .is_err()
        {
            return;
        }
        let loads = s
            .trim()
            .split(' ')
            .take(3)
            .map(|val| val.parse::<f64>().unwrap())
            .collect::<Vec<f64>>();
        self.one = loads[0];
        self.five = loads[1];
        self.fifteen = loads[2];
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

#[cfg(test)]
mod tests_load {
    use super::*;
    use std::env;

    #[test]
    fn test_load() {
        let current_path: PathBuf = env::current_dir().unwrap();
        let mut load_instance = LoadAvg::new();
        load_instance.read_load_avg(PathBuf::from(format!(
            "{}/tests/sample/proc_loadavg",
            current_path.to_string_lossy()
        )));
        assert_eq!(load_instance.one, 1.29);
        assert_eq!(load_instance.five, 1.24);
        assert_eq!(load_instance.fifteen, 1.17);
    }
}
