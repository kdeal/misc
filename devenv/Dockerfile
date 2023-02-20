FROM ubuntu:jammy

RUN apt-get update \
    && apt-get -y install --no-install-recommends \
        fish vim \
        build-essential \
        nodejs npm \
        python3-pip \
        openssh-server \
    && rm -rf /var/lib/apt/lists/* \
    && sed -i 's/#PasswordAuthentication yes/PasswordAuthentication no/g' /etc/ssh/sshd_config

COPY entrypoint.sh /tmp/

EXPOSE 22
CMD ["/tmp/entrypoint.sh"]
