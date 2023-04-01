FROM ubuntu:focal

RUN apt-get update \
    && apt-get -y install --no-install-recommends ca-certificates apt-transport-https curl \
    && echo 'deb https://www.ui.com/downloads/unifi/debian stable ubiquiti' \
        > /etc/apt/sources.list.d/100-ubnt-unifi.list \
    && curl https://dl.ui.com/unifi/unifi-repo.gpg > /etc/apt/trusted.gpg.d/unifi-repo.gpg \
    && apt-get update \
    && apt-get -y install --no-install-recommends unifi

COPY entrypoint.sh /tmp/

EXPOSE 8080
EXPOSE 8443
EXPOSE 3478/udp

CMD ["/tmp/entrypoint.sh"]
