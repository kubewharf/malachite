<h1 align="center">
  <p align="center">Malachite</p>
</h1>

## Overview

- Malachite is a daemon written in Rust, which is responsible for node-level and cgroup-level metrics collection, e.g., memory usage and network bandwidth.
- Malachite support cgroup, both V1 and V2, which can provide running information of k8s Pods and Containers.
- Malachite mainly obtains relevant metrics from pseudo-filesystems, e.g., /proc and /sys, and expand its capabilities with EBPF program. All of these can be switched on and off dynamically to save metric collection overhead.

## Quick Start

### Build
`git clone https://github.com/kubewharf/malachite`

`cd malachite`

`make build`

### Run
make run

### Use Case 
get Node Memory info

`curl "http://localhost:8000/api/v1/system/memory"`

get cgroup info with relative path

`curl "http://localhost:8000/api/v1/cgroup/groups/?cgroup_user_path=/kubepods/burstable/xxx"`

### Deploying
Please refer to [Charts](https://github.com/kubewharf/charts/tree/main/charts/malachite) for detailed helm charts. 


## Community

### Contact

If you have any questions or wish to contribute, you are welcome to communicate via GitHub issues or pull requests.
Alternatively, you may reach out to our [Maintainers](./MAINTAINERS.md).

## License

Malachite is under the Apache 2.0 license. See the [LICENSE](LICENSE) file for details.
