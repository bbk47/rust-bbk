#!/usr/bin/env bash

bbk_VERSION=$(git describe --tags --abbrev=0)
GIT_HASH=$(git rev-parse --short HEAD)
echo $bbk_VERSION'-'$GIT_HASH

cwd=$(pwd)

rm -rf ./output
mkdir -p ./output

platforms=(x86_64-apple-darwin
x86_64-pc-windows-gnu
x86_64-unknown-freebsd
x86_64-unknown-linux-gnu )

for platform in "${platforms[@]}"
do

    targetzipname="bbk_${bbk_VERSION}_${platform}"
    bbk_outdir="output/${targetzipname}"
    output_name=bbk
    if echo "$platform" | grep -q "windows"; then
        output_name+='.exe'
    fi
    echo "Build ${bbk_outdir}..."

    cargo build --release --target ${platform} --target-dir "abc123"

    if [ $? -eq 0 ]; then
        echo "Build ${bbk_outdir} done"
        
        mkdir -p $bbk_outdir
        cp ./output/target/release/bbk ${bbk_outdir}
        cp -rf ./etc ${bbk_outdir}
        cp -rf ./examples ${bbk_outdir}

        cd $bbk_outdir
        if echo "$platform" | grep -q "windows"; then
            cp $targetzipname/bbk $targetzipname/bbk.exe
            zip -rq ${targetzipname}.zip ${targetzipname}
        else
            tar -zcf ${targetzipname}.tar.gz ${targetzipname}
        fi
        rm -rf ${targetzipname}
        cd $cwd
    else
        echo "Failed to build ${bbk_outdir}"
    fi
done

cd -

