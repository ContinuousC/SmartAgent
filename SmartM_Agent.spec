Summary:    SmartAgent
Name:       smart-agent
Version:    %(echo $GITVERSION | sed -e 's/_/./g' -e 's/-/_/g')
Release:    1%{?dist}
License:    UNSPECIFIED
Group:      Applications/System
Requires:   %{name}-libs%{?_isa} = %{version}-%{release}

%global netsnmp_soname 40

%description
ContinuousC Smart Agent.

%package libs
Summary: Shared libraries required by Smart Agent.
Provides: libnetsnmp_si.so.%{netsnmp_soname}()(64bit)

%description libs
Shared libraries required by Smart Agent.


%prep
# no preparation

%build
#make

%install
make -C /root/source osinstall-$TARGET build_type=$BUILD_TYPE \
     pkgroot=%{buildroot} prefix=%{buildroot}/usr \
     libdir=%{buildroot}/usr/lib64

%files
%attr(0755,root,root) /usr/sbin/smart-agent
%attr(0755,root,root) %dir /usr/share/smart-agent
%attr(0700,root,root) %dir /usr/share/smart-agent/certs
#%attr(0600,root,root) /usr/share/smart-agent/certs/ca.crt
#%attr(0600,root,root) /usr/share/smart-agent/certs/agent.crt
#%attr(0600,root,root) /usr/share/smart-agent/certs/agent.key
%attr(0755,root,root) %dir /etc/smart-agent
#%attr(0644,root,root) %config(noreplace) /etc/smart-agent/config.yaml
%attr(0755,root,root) %dir /var/lib/smart-agent
#%attr(0644,root,root) /usr/lib/systemd/system/smart-agent.service
#%attr(0644,root,root) /usr/lib/systemd/system-preset/01-smart-agent.preset

%files libs
/usr/lib64/libnetsnmp_si.so.%{netsnmp_soname}*

%post
%systemd_post %{pkgname}.service

%preun
%systemd_preun %{pkgname}.service

%postun
%systemd_postun %{pkgname}.service

%changelog
# let's skip this for now
