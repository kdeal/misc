#!/bin/bash

# Setup ssh user
addgroup --gid "$SSH_GROUP_ID" "$SSH_USER"
adduser --disabled-password --gecos "" --uid "$SSH_USER_ID" --gid "$SSH_GROUP_ID" "$SSH_USER"
mkdir -p "/home/$SSH_USER/.ssh/"
chown "$SSH_USER:$SSH_USER" "/home/$SSH_USER/.ssh/"
echo "$SSH_PUB_KEY" > "/home/$SSH_USER/.ssh/authorized_keys"

# Setup ssh server
# Copy host keys into config dir
test -d /config && cp /config/*_key* /etc/ssh/
mkdir --mode=0755 /run/sshd

echo "Starting ssh server"
exec /usr/sbin/sshd -D
