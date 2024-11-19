FROM ubuntu:noble

RUN apt-get update \
    && apt-get -y install --no-install-recommends \
        build-essential \
        curl \
        git \
        man-db \
        nodejs npm \
        openssh-server \
        python3-pip \
        python3-virtualenv \
        unminimize \
        rsync \
        unzip \
        vim \
    && yes | unminimize \
    && rm -rf /var/lib/apt/lists/* \
    && sed -i 's/#PasswordAuthentication yes/PasswordAuthentication no/g' /etc/ssh/sshd_config \
    #  Remove default user to free up 1000 uid in case I want to use it
    && userdel ubuntu

COPY entrypoint.sh /tmp/

EXPOSE 22
CMD ["/tmp/entrypoint.sh"]
