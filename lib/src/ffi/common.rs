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

use log::warn;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type ModuleMaskIDType = ::std::os::raw::c_uint;
pub type ModuleMaskType = ::std::os::raw::c_ulong;

#[derive(Clone, Debug, Copy, Serialize, Deserialize, Default)]
pub struct ModuleMask {
    mask: ModuleMaskType,
}

impl ModuleMask {
    pub fn new(mask: ModuleMaskType) -> ModuleMask {
        ModuleMask { mask }
    }
    pub fn get_mask(&self) -> ModuleMaskType {
        self.mask
    }
}

impl From<Vec<ModuleMaskIDType>> for ModuleMask {
    fn from(vec_input: Vec<ModuleMaskIDType>) -> Self {
        ModuleMask {
            mask: vec_input.iter().fold(0, |mask, &x| mask | (1 << x)),
        }
    }
}

impl From<ModuleMask> for Vec<ModuleMaskIDType> {
    fn from(module_mask: ModuleMask) -> Self {
        let mask = module_mask.mask;
        let binary: String = format!("{:b}", mask);
        binary
            .chars()
            .map(|c| c.to_digit(10).unwrap())
            .collect::<Vec<_>>()
    }
}

pub struct ModuleMaskConfig {
    name_to_index: HashMap<&'static str, ModuleMaskIDType>,
    index_to_name: HashMap<ModuleMaskIDType, &'static str>,
}

impl ModuleMaskConfig {
    pub fn new(name_to_value: HashMap<&'static str, ModuleMaskIDType>) -> ModuleMaskConfig {
        let value_to_name = name_to_value
            .clone()
            .into_iter()
            .map(|(k, v)| (v, k))
            .collect::<HashMap<ModuleMaskIDType, &'static str>>();
        ModuleMaskConfig {
            name_to_index: name_to_value,
            index_to_name: value_to_name,
        }
    }
    pub fn get_config_index(&self, name: String) -> Option<ModuleMaskIDType> {
        match self.name_to_index.get(name.as_str()).cloned() {
            None => {
                warn!(
                    "[lib-ffi] ModuleMaskConfig get config index err: name= {}",
                    name
                );
                None
            }
            Some(idx) => Some(idx),
        }
    }

    pub fn get_config_name(&self, idx: ModuleMaskIDType) -> Option<String> {
        match self.index_to_name.get(&idx).cloned() {
            None => {
                warn!(
                    "[lib-ffi] ModuleMaskConfig get config name err: idx= {}",
                    idx
                );
                None
            }
            Some(s) => Some(String::from(s)),
        }
    }
    pub fn get_idx_vec(&self, names_vec: Vec<String>) -> Vec<ModuleMaskIDType> {
        let mut idx_vec: Vec<ModuleMaskIDType> = Vec::new();
        for name in names_vec {
            match self.get_config_index(name) {
                None => continue,
                Some(idx) => idx_vec.push(idx),
            }
        }
        idx_vec
    }
    pub fn get_names_vec(&self, mask_ids: Vec<ModuleMaskIDType>) -> Vec<String> {
        let mut mask_enable_list: Vec<String> = Vec::new();
        for (i, v) in mask_ids.iter().rev().enumerate() {
            if *v == 1 {
                let config_name = self.get_config_name(i as ModuleMaskIDType);
                match config_name {
                    None => continue,
                    Some(name) => mask_enable_list.push(name),
                }
            }
        }
        mask_enable_list
    }
}

#[test]
fn test_module_mask() {
    let correct_mask: ModuleMaskType = (1 << 2) | (1 << 1) | (1 << 4) | (1 << 3) | (1 << 5);
    let mask_vec: Vec<ModuleMaskIDType> = vec![1, 2, 3, 4, 5];
    let module_mask = ModuleMask::from(mask_vec);
    assert_eq!(correct_mask, module_mask.get_mask());
}
