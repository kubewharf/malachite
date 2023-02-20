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

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn parse_cgroup_file(contents: &str) -> HashMap<&str, u64> {
    // parse file like:
    // inactive_file 107535818752
    // active_file 60048904192
    // unevictable 4096
    // hierarchical_memory_limit 412316860416
    // hierarchical_memsw_limit 9223372036854771712
    // total_cache 167586779136
    // total_rss 223102943232
    // ....
    contents
        .split('\n')
        .into_iter()
        .map(|s| {
            let mut s = s.split_whitespace();
            if let Some(key) = s.next() {
                if let Some(value) = s.next() {
                    (key, value.parse::<u64>().unwrap())
                } else {
                    ("", 0)
                }
            } else {
                ("", 0)
            }
            //(s.next().unwrap_or(""), s.next().unwrap_or(&"0").parse::<u64>().unwrap())
        })
        .collect()
}

pub fn parse_cgroup_numa_stat_file(contents: &str) -> HashMap<&str, HashMap<&str, u64>> {
    // parse file like:
    // total=638459 N0=49505 N1=589301
    // file=315488 N0=24384 N1=291789
    // anon=322971 N0=25121 N1=297512
    // unevictable=0 N0=0 N1=0
    // hierarchical_total=638459 N0=49505 N1=589301
    // hierarchical_file=315488 N0=24384 N1=29178
    let mut results: HashMap<&str, HashMap<&str, u64>> = HashMap::new();
    contents.split('\n').into_iter().for_each(|s| {
        let mut items = s.split_whitespace();
        if let Some(first_line) = items.next() {
            let mut item = first_line.split('=');
            if let Some(field) = item.next() {
                if let Some(_field_value) = item.next() {
                    //let item_map = results.entry("summary").or_insert(HashMap::new());
                    //item_map.insert(field, field_value.parse::<u64>().unwrap());
                    items.for_each(|s| {
                        let mut iter = s.split('=');
                        if let Some(node_id) = iter.next() {
                            if let Some(value) = iter.next() {
                                let item_map = results.entry(node_id).or_insert_with(HashMap::new);
                                item_map.insert(field, value.parse::<u64>().unwrap());
                            }
                        }
                    });
                }
            }
        }
    });
    results
}

pub fn get_cgroup_value<P: AsRef<Path>>(path: P) -> Option<u64> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);
    if let Some(Ok(data)) = reader.lines().next() {
        if data == "max" {
            Some(std::u64::MAX)
        } else {
            Some(data.parse::<u64>().ok()?)
        }
    } else {
        None
    }
}

pub fn parse_cgroup_reverse_nested_u64(contents: &str) -> HashMap<String, HashMap<String, u64>> {
    let mut ret_map: HashMap<String, HashMap<String, u64>> = HashMap::new();
    //anon N0=5001216 N1=7028736 N2=4866048 N3=89346048
    //file N0=6082560 N1=5947392 N2=5677056 N3=37576704
    contents.split('\n').into_iter().for_each(|s| {
        let mut space = s.split_whitespace();
        if let Some(field) = space.next() {
            space.for_each(|s| {
                let mut item = s.split('=');
                if let Some(item_key) = item.next() {
                    if let Some(item_value) = item.next() {
                        let item_map = ret_map
                            .entry(String::from(item_key))
                            .or_insert_with(HashMap::new);
                        item_map.insert(String::from(field), item_value.parse::<u64>().unwrap());
                    }
                }
            })
        }
    });
    ret_map
}

pub fn parse_cgroup_nested_u64(contents: &str) -> HashMap<String, HashMap<String, u64>> {
    let mut ret_map: HashMap<String, HashMap<String, u64>> = HashMap::new();
    //8:16 rbytes=1459200 wbytes=314773504 rios=192 wios=353 dbytes=0 dios=0
    //8:0 rbytes=90430464 wbytes=299008000 rios=8950 wios=1252 dbytes=50331648 dios=3021
    contents.split('\n').into_iter().for_each(|s| {
        let mut space = s.split_whitespace();
        if let Some(field) = space.next() {
            let item_map = ret_map
                .entry(String::from(field))
                .or_insert_with(HashMap::new);
            space.for_each(|s| {
                let mut item = s.split('=');
                if let Some(item_key) = item.next() {
                    if let Some(item_value) = item.next() {
                        if item_value == "max" {
                            item_map.insert(String::from(item_key), std::u64::MAX);
                        } else {
                            item_map
                                .insert(String::from(item_key), item_value.parse::<u64>().unwrap());
                        }
                    }
                }
            })
        }
    });
    ret_map
}

#[cfg(test)]
mod tests_utils {

    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_parse_cgroup_numa_stat_file() {
        let file_content = "total=638459 N0=49505 N1=589301\nfile=315488 N0=24384 N1=291789\nanon=322971 N0=25121 N1=297512\nunevictable=0 N0=0 N1=0\nhierarchical_total=638459 N0=49505 N1=589301\n";
        let output = parse_cgroup_numa_stat_file(file_content);
        assert_eq!(
            output.get("N0").unwrap().get("total").unwrap(),
            &(49505 as u64)
        );
        assert_eq!(
            output.get("N0").unwrap().get("anon").unwrap(),
            &(25121 as u64)
        );
        assert_eq!(
            output.get("N1").unwrap().get("total").unwrap(),
            &(589301 as u64)
        );

        assert_eq!(
            output.get("N1").unwrap().get("anon").unwrap(),
            &(297512 as u64)
        );

        assert_eq!(
            output.get("N1").unwrap().get("file").unwrap(),
            &(291789 as u64)
        );
    }
    #[test]
    fn test_parse_cgroup_file() {
        let file_content = "inactive_file 107535818752\nactive_file 60048904192\nunevictable 4096\nhierarchical_memory_limit 412316860416\nhierarchical_memsw_limit 9223372036854771712\ntotal_cache 167586779136\ntotal_rss 223102943232\n";
        let output = parse_cgroup_file(file_content);
        let mut correct_content: HashMap<&str, u64> = HashMap::new();
        correct_content.insert("inactive_file", 107535818752);
        correct_content.insert("active_file", 60048904192);
        correct_content.insert("unevictable", 4096);
        correct_content.insert("hierarchical_memory_limit", 412316860416);
        correct_content.insert("hierarchical_memsw_limit", 9223372036854771712);
        correct_content.insert("total_cache", 167586779136);
        correct_content.insert("total_rss", 223102943232);
        correct_content.insert("", 0);
        assert_eq!(output, correct_content);
    }

    #[test]
    fn test_parse_cgroup_nested_u64() {
        let file_content = "8:16 rbytes=1459200 wbytes=314773504 rios=192 wios=353 dbytes=0 dios=0\n8:0 rbytes=90430464 wbytes=299008000 rios=8950 wios=1252 dbytes=50331648 dios=3021\n";
        let output = parse_cgroup_nested_u64(file_content);
        let mut correct_content: HashMap<String, HashMap<String, u64>> = HashMap::new();
        let mut device1: HashMap<String, u64> = HashMap::new();
        device1.insert(String::from("rbytes"), 1459200);
        device1.insert(String::from("wbytes"), 314773504);
        device1.insert(String::from("rios"), 192);
        device1.insert(String::from("wios"), 353);
        device1.insert(String::from("dbytes"), 0);
        device1.insert(String::from("dios"), 0);
        correct_content.insert(String::from("8:16"), device1);

        let mut device2: HashMap<String, u64> = HashMap::new();
        device2.insert(String::from("rbytes"), 90430464);
        device2.insert(String::from("wbytes"), 299008000);
        device2.insert(String::from("rios"), 8950);
        device2.insert(String::from("wios"), 1252);
        device2.insert(String::from("dbytes"), 50331648);
        device2.insert(String::from("dios"), 3021);
        correct_content.insert(String::from("8:0"), device2);

        assert_eq!(output, correct_content);
    }

    #[test]
    fn test_parse_cgroup_reverse_nested_u64() {
        let file_content = "anon N0=5001216 N1=7028736\nfile N0=6082560 N1=5947392\n";
        let output = parse_cgroup_reverse_nested_u64(file_content);
        let mut correct_content: HashMap<String, HashMap<String, u64>> = HashMap::new();
        let mut numa0: HashMap<String, u64> = HashMap::new();
        numa0.insert(String::from("anon"), 5001216);
        numa0.insert(String::from("file"), 6082560);
        correct_content.insert(String::from("N0"), numa0);

        let mut numa1: HashMap<String, u64> = HashMap::new();
        numa1.insert(String::from("anon"), 7028736);
        numa1.insert(String::from("file"), 5947392);
        correct_content.insert(String::from("N1"), numa1);

        assert_eq!(output, correct_content);
    }
}
