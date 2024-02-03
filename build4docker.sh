#!/bin/bash
set -e

echo "Building linux binary..."
./build4linux.sh
status=$?

if [ $status -ne 0 ]; then
    echo "Failed to build linux binary"
    exit 1
fi
echo "Building docker image..."
docker rm -f extract_link:latest
docker build --platform=linux/amd64 -t extract_link:latest .

echo "Pushing to docker hub..."
docker tag extract_link:latest nickmsft/extract_link:latest
docker push nickmsft/extract_link:latest





