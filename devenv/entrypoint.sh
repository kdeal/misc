#!/bin/bash

# Setup ssh user
adduser --disabled-password --gecos "" "$SSH_USER"
mkdir -p "/home/$SSH_USER/.ssh/"
echo "$SSH_PUB_KEY" > "/home/$SSH_USER/.ssh/authorized_keys"

# Setup ssh server
# Copy host keys into config dir
test -d /config && cp /config/*_key* /etc/ssh/
mkdir --mode=0755 /run/sshd

echo "Starting ssh server"
exec /usr/sbin/sshd -D
