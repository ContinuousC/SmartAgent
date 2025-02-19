FROM rust-rhel8
#RUN yum install -y clang openssl-devel file make krb5-devel python2 python2-devel && yum clean all
RUN yum install -y unixODBC-devel libsodium-devel
RUN echo /usr/local/lib > /etc/ld.so.conf.d/local.conf
COPY external/netsnmp_si-rhel8*.tar.gz /
RUN tar -C/ -xf /netsnmp_si-rhel8-dev.tar.gz && tar -C/ -xf /netsnmp_si-rhel8.tar.gz
