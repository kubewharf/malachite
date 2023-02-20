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

//parse /proc/net/snmp or /proc/net/netstat
pub fn parse_net_file(contents: &str) -> HashMap<&str, HashMap<&str, &str>> {
    let data_iter = contents.split('\n');
    let data_iter_clone = data_iter.clone();
    data_iter
        .step_by(2)
        .zip(data_iter_clone.skip(1).step_by(2))
        .map(|(a, b)| {
            let mut first_iter = a.split_whitespace();
            let second_iter = b.split_whitespace();
            let mut first_item = (*first_iter.next().unwrap()).chars();
            first_item.next_back();
            let net_type = first_item.as_str();
            let net_kv = first_iter
                .zip(second_iter.skip(1))
                .collect::<HashMap<_, _>>();
            (net_type, net_kv)
        })
        .collect::<HashMap<_, _>>()
}

#[cfg(test)]
mod tests_utils {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_parse_net_file() {
        let file_content =
            "field_a: k1 k2 k3\nfield_a: v1 v2 v3\nfield_b: k11 k22\nfield_b: v11 v22";
        let output = parse_net_file(file_content);
        let field_1: HashMap<&str, &str> = vec![("k1", "v1"), ("k2", "v2"), ("k3", "v3")]
            .into_iter()
            .collect::<HashMap<&str, &str>>();
        let field_2: HashMap<&str, &str> = vec![("k11", "v11"), ("k22", "v22")]
            .into_iter()
            .collect::<HashMap<_, _>>();
        let correct_output: HashMap<&str, HashMap<&str, &str>> =
            vec![("field_a", field_1), ("field_b", field_2)]
                .into_iter()
                .collect();
        assert_eq!(output, correct_output)
    }
}
