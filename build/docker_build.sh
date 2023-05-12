#!/bin/bash

arch=$(uname -m | sed 's/x86_64/x86/' | sed 's/aarch64/arm64/')

if [[ $arch == "x86" ]]; then
    image=kubewharf/malachite_build:0.0.3
else
    echo "unknown arch: $arch"
    exit
fi


malachite_path=`pwd`
docker run --rm -v $HOME:/root -v $malachite_path:/malachite $image /bin/bash -c "cd /malachite && bash build/build.sh"
