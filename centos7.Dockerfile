FROM rust-centos7
# RUN yum install -y openssl-devel file make pciutils-devel perl-ExtUtils-Embed rpm-devel libkrb5-dev python-devel && yum clean all
RUN yum install -y unixODBC-devel libsodium-devel
RUN echo /usr/local/lib > /etc/ld.so.conf.d/local.conf
COPY external/netsnmp_si-centos7*.tar.gz /
RUN tar -C/ -xf /netsnmp_si-centos7-dev.tar.gz && tar -C/ -xf /netsnmp_si-centos7.tar.gz
