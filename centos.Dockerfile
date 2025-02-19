ARG TARGET
FROM ${TARGET}
RUN yum install -y make git rpmdevtools && yum clean all
CMD rpmdev-setuptree && cd /root/source && rpmbuild -ba agent.spec && mkdir -p Build/rpms Build/srpms && cp -a /root/rpmbuild/RPMS/x86_64/*.rpm Build/rpms && cp -a /root/rpmbuild/SRPMS/*.rpm Build/srpms
