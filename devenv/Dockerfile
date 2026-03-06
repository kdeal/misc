FROM golang:trixie AS go-base

FROM ubuntu:noble

ENV USER_PATH=""

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    USER_PATH=~/.cargo/bin:/usr/local/cargo/bin:$USER_PATH \
    RUST_VERSION=1.94.0

RUN set -eux; \
    apt-get update; \
    apt-get -y install --no-install-recommends \
     build-essential \
     curl \
     git \
     man-db \
     nodejs npm \
     openssh-server \
     podman \
     python3-pip \
     python3-virtualenv \
     unminimize \
     rsync \
     unzip \
     vim \
     wget; \
    yes | unminimize; \
    rm -rf /var/lib/apt/lists/*; \
    sed -i 's/#PasswordAuthentication yes/PasswordAuthentication no/g' /etc/ssh/sshd_config; \
    # Remove default user to free up 1000 uid in case I want to use it
    userdel ubuntu; \
    #
    # Install rust from https://github.com/rust-lang/docker-rust/blob/master/stable/bullseye/Dockerfile
    #
    arch="$(dpkg --print-architecture)"; \
    case "$arch" in \
        'amd64') \
            rustArch='x86_64-unknown-linux-gnu'; \
            rustupSha256='20a06e644b0d9bd2fbdbfd52d42540bdde820ea7df86e92e533c073da0cdd43c'; \
            ;; \
        'armhf') \
            rustArch='armv7-unknown-linux-gnueabihf'; \
            rustupSha256='3b8daab6cc3135f2cd4b12919559e6adaee73a2fbefb830fadf0405c20231d61'; \
            ;; \
        'arm64') \
            rustArch='aarch64-unknown-linux-gnu'; \
            rustupSha256='e3853c5a252fca15252d07cb23a1bdd9377a8c6f3efa01531109281ae47f841c'; \
            ;; \
        'i386') \
            rustArch='i686-unknown-linux-gnu'; \
            rustupSha256='a5db2c4b29d23e9b318b955dd0337d6b52e93933608469085c924e0d05b1df1f'; \
            ;; \
        *) \
            echo >&2 "unsupported architecture: $arch"; \
            exit 1; \
            ;; \
    esac; \
    \
    url="https://static.rust-lang.org/rustup/archive/1.28.2/${rustArch}/rustup-init"; \
    wget --progress=dot:giga "$url"; \
    echo "${rustupSha256} *rustup-init" | sha256sum -c -; \
    \
    chmod +x rustup-init; \
    ./rustup-init -y --no-modify-path --profile minimal --default-toolchain $RUST_VERSION --default-host ${rustArch}; \
    rm rustup-init; \
    chmod -R a+w $RUSTUP_HOME $CARGO_HOME; \
    \
    rustup --version; \
    cargo --version; \
    rustc --version;

# Setup Go
ENV GOLANG_VERSION=1.26.0
ENV GOTOOLCHAIN=local
ENV PATH=~/go/bin:/usr/local/go/bin:$PATH
ENV USER_PATH=~/go/bin:/usr/local/go/bin:$USER_PATH
COPY --from=go-base /usr/local/go/ /usr/local/go/

# Adjust global path
RUN sed "s%PATH=\"%PATH=\"${USER_PATH}%" -i /etc/environment

COPY entrypoint.sh /tmp/

EXPOSE 22
CMD ["/tmp/entrypoint.sh"]
