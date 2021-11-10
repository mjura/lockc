#!/bin/bash

# ensure running as root
if [ "$(id -u)" != "0" ]; then
  exec sudo "$0" "$@"
fi

DOCKER_VERSION=$(curl -s https://api.github.com/repos/moby/moby/releases/latest | jq -r '.tag_name' | sed -e 's/^v//')
curl -L "https://download.docker.com/linux/static/stable/x86_64/docker-${DOCKER_VERSION}.tgz" | sudo tar -C /usr/local/bin -xz

systemctl enable docker
