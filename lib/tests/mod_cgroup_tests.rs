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

#[cfg(test)]
mod tests_blkio_cg {
    use lib::cgroup::{
        new_blkio_cgroup, parse_blkio_file, BlkIOCGroup, BlkOperationType, SubSystem,
    };
    use lib::common::CGroupType;
    use std::collections::HashMap;
    use std::env;
    use std::path::PathBuf;

    #[test]
    fn test_parse_blkio_file() {
        let content = "8:0 Read 6737920\n8:0 Write 255721472\n8:0 Sync 6737920\n8:0 Async 255721472\n8:0 Total 262459392\n8:16 Read 239214592\n8:16 Write 69632\n8:16 Sync 239247360\n8:16 Async 36864\n8:16 Total 239284224\n254:0 Read 239214592\n254:0 Write 69632\n254:0 Sync 239247360\n254:0 Async 36864\n254:0 Total 239284224\nTotal 5263360\n";
        let output = parse_blkio_file(content);
        let mut blk_info: HashMap<String, HashMap<BlkOperationType, u64>> = HashMap::new();
        let mut blk_device_1 = HashMap::new();
        blk_device_1.insert(BlkOperationType::Read, 6737920);
        blk_device_1.insert(BlkOperationType::Write, 255721472);
        blk_device_1.insert(BlkOperationType::Sync, 6737920);
        blk_device_1.insert(BlkOperationType::Async, 255721472);
        blk_device_1.insert(BlkOperationType::Total, 262459392);
        blk_info.insert(String::from("8:0"), blk_device_1);

        let mut blk_device_2 = HashMap::new();
        blk_device_2.insert(BlkOperationType::Read, 239214592);
        blk_device_2.insert(BlkOperationType::Write, 69632);
        blk_device_2.insert(BlkOperationType::Sync, 239247360);
        blk_device_2.insert(BlkOperationType::Async, 36864);
        blk_device_2.insert(BlkOperationType::Total, 239284224);
        blk_info.insert(String::from("8:16"), blk_device_2);

        let mut blk_device_3 = HashMap::new();
        blk_device_3.insert(BlkOperationType::Read, 239214592);
        blk_device_3.insert(BlkOperationType::Write, 69632);
        blk_device_3.insert(BlkOperationType::Sync, 239247360);
        blk_device_3.insert(BlkOperationType::Async, 36864);
        blk_device_3.insert(BlkOperationType::Total, 239284224);
        blk_info.insert(String::from("254:0"), blk_device_3);

        let correct_answer = (5263360 as u64, blk_info);
        assert_eq!(output.unwrap(), correct_answer);
    }

    #[test]
    fn test_blk_operation_type() {
        let vec_str = vec!["Read", "Write", "Async", "Sync", "Total", "xxxx"];
        let mut vec_enum_item: Vec<BlkOperationType> = vec![];
        for i in 0..vec_str.len() {
            vec_enum_item.push(BlkOperationType::from(vec_str[i]))
        }
        for i in 0..vec_enum_item.len() {
            let value = vec_enum_item[i].to_owned();
            match value {
                BlkOperationType::Unknown => assert_eq!(value.as_str(), "Unknown"),
                _ => assert_eq!(value.as_str(), vec_str[i]),
            }
        }
        assert_eq!(format!("{}", BlkOperationType::Unknown), "Unknown");
    }

    #[test]
    fn test_blkio_cg_v1() {
        let mount_point: String = String::from(format!(
            "{}/tests/sample",
            env::current_dir().unwrap().to_string_lossy()
        ));
        let user_path: String = String::from("pod_user_path");
        let mut blkio = new_blkio_cgroup(
            &mount_point,
            &PathBuf::from(user_path.clone()),
            CGroupType::V1 {},
        );

        if let BlkIOCGroup::V1(ref mut blkio_cg) = blkio {
            blkio_cg.update_bps().unwrap();
            blkio_cg.update_iops().unwrap();

            let correct_full_path = PathBuf::from(format!("{}/blkio/pod_user_path", mount_point));

            assert_eq!(blkio_cg.full_path(), correct_full_path.as_path());
            assert_eq!(
                blkio_cg.user_path(),
                PathBuf::from(user_path.clone()).as_path()
            );

            let mut correct_bps_details: HashMap<String, HashMap<BlkOperationType, u64>> =
                HashMap::new();
            let mut bps_device_1 = HashMap::new();
            bps_device_1.insert(BlkOperationType::Read, 5226496);
            bps_device_1.insert(BlkOperationType::Write, 36864);
            bps_device_1.insert(BlkOperationType::Sync, 5259264);
            bps_device_1.insert(BlkOperationType::Async, 4096);
            bps_device_1.insert(BlkOperationType::Total, 5263360);
            correct_bps_details.insert(String::from("8:16"), bps_device_1);

            assert_eq!(*blkio_cg.bps_details(), correct_bps_details);
            assert_eq!(blkio_cg.bps_total(), 5263360 as u64);

            let mut correct_iops_details: HashMap<String, HashMap<BlkOperationType, u64>> =
                HashMap::new();
            let mut iops_device_1 = HashMap::new();
            iops_device_1.insert(BlkOperationType::Read, 129);
            iops_device_1.insert(BlkOperationType::Write, 4);
            iops_device_1.insert(BlkOperationType::Sync, 132);
            iops_device_1.insert(BlkOperationType::Async, 1);
            iops_device_1.insert(BlkOperationType::Total, 133);
            correct_iops_details.insert(String::from("8:16"), iops_device_1);

            assert_eq!(*blkio_cg.iops_details(), correct_iops_details);
            assert_eq!(blkio_cg.iops_total(), 133 as u64);

            // test for blkio subsystem
            {
                let blkio_subsystem = SubSystem::BlkIO(blkio);
                assert_eq!(blkio_subsystem.get_full_path(), correct_full_path.as_path());
                assert_eq!(blkio_subsystem.sub_system_path_exists(), true);
                assert_eq!(blkio_subsystem.is_blkio_cgroup(), true);
                assert_eq!(blkio_subsystem.is_cpu_cgroup(), false);
            }
        }
    }
}

#[cfg(test)]
mod tests_net_cg {
    use lib::cgroup::{new_net_cgroup, SubSystem};
    use lib::common::CGroupType;
    use std::env;
    use std::path::PathBuf;

    #[test]
    fn test_net_cg() {
        let mount_point: String = String::from(format!(
            "{}/tests/sample",
            env::current_dir().unwrap().to_string_lossy()
        ));
        let user_path: String = String::from("pod_user_path");
        let mut net_cg = new_net_cgroup(
            &mount_point,
            &PathBuf::from(user_path.clone()),
            CGroupType::V1 {},
        );

        let correct_full_path = PathBuf::from(format!("{}/net_cls/pod_user_path", mount_point));

        assert_eq!(net_cg.full_path(), correct_full_path.as_path());
        assert_eq!(
            net_cg.user_path(),
            PathBuf::from(user_path.clone()).as_path()
        );

        // test for net subsystem
        {
            let net_subsystem = SubSystem::Net(net_cg);
            assert_eq!(net_subsystem.get_full_path(), correct_full_path.as_path());
            assert_eq!(net_subsystem.sub_system_path_exists(), true);
            assert_eq!(net_subsystem.is_net_cgroup(), true);
            assert_eq!(net_subsystem.is_memory_cgroup(), false);
        }
    }
}

#[cfg(test)]
mod tests_cg {
    use lib::cgroup::CGroup;
    use std::env;
    use std::path::PathBuf;
    #[test]
    fn test_cgroup() {
        let mount_point: PathBuf = env::current_dir().unwrap();
        let user_path: PathBuf = PathBuf::from("pod_user_path");
        let cg = CGroup::new(mount_point.to_str().unwrap(), user_path.clone());

        assert_eq!(cg.user_path(), user_path.clone().as_path());
        assert_eq!(cg.mount_point(), mount_point.as_path());
        assert_eq!(cg.is_valid(), true);

        let mut cg_fake = CGroup::new("fake_mount_point", PathBuf::from("pod_user_path"));
        assert_eq!(cg_fake.is_valid(), true);
        cg_fake.clear_invalid_subsystem_item();
        assert_eq!(cg_fake.is_valid(), false);
        //TODO: unitest cgroup.update
    }
}
