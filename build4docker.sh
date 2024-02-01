echo "Building linux binary..."
./build_linux.sh
echo "Building docker image..."
docker rm -f extract_link:latest
docker build --platform=linux/amd64 -t extract_link:latest .

echo "Pushing to docker hub..."
docker tag extract_link:latest nickmsft/extract_link:latest
docker push nickmsft/extract_link:latest

