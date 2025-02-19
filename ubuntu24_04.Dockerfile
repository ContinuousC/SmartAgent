FROM rust-ubuntu24_04
RUN apt install -y unixodbc-dev libsodium-dev
RUN echo /usr/local/lib > /etc/ld.so.conf.d/local.conf
COPY external/netsnmp_si-ubuntu22_04*.tar.gz /
RUN tar -C/ -xf /netsnmp_si-ubuntu22_04-dev.tar.gz && tar -C/ -xf /netsnmp_si-ubuntu22_04.tar.gz
