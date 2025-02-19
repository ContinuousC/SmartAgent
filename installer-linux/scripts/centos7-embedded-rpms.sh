#!/bin/sh

if ! (. /etc/os-release; echo $NAME $VERSION_ID) | grep -qxF 'CentOS Linux 7'; then
    echo 'This script is intended for CentOS 7 systems; detected another OS!'
    exit 1
fi

if ! [ $(id -u) == 0 ]; then
    echo "This script must be run as root!"
    exit 1
fi


tmpdir="$(mktemp -d)"
rpmfile="smart-agent-{{RpmVersion}}.el7.x86_64.rpm"
rpmlibsfile="smart-agent-libs-{{RpmVersion}}.el7.x86_64.rpm"


echo "Downloading $rpmfile..."

base64 -d <<EOF > "$tmpdir/$rpmfile"
{{SmartAgentRpmBase64}}
EOF


echo "Downloading $rpmlibsfile..."

base64 -d <<EOF > "$tmpdir/$rpmlibsfile"
{{SmartAgentLibsRpmBase64}}
EOF


echo "Installing ContinuousC Smart Agent..."
rpm -U "$tmpdir/$rpmfile" "$tmpdir/$rpmlibsfile" || exit 1

rm -f "$tmpdir/$rpmfile" "$tmpdir/$rpmlibsfile"
rmdir "$tmpdir"


echo "Installing certificates..."

cat <<EOF > /usr/share/smart-agent/certs/ca.crt
{{CaCert}}
EOF

cat <<EOF > /usr/share/smart-agent/certs/agent.crt
{{AgentCert}}
EOF

cat <<EOF > /usr/share/smart-agent/certs/agent.key
{{AgentKey}}
EOF

chmod 600 /usr/share/smart-agent/certs/agent.key


echo "Installing the service file..."

cat <<EOF > /usr/lib/systemd/system/smart-agent.service
[Unit]
Description=ContinuousC Smart Agent

[Service]
Type=simple
User=root
Group=root
ExecStart=/usr/sbin/smart-agent --broker {{BrokerArg}}{{BrokerCompat}} {{ConnectArg}}
WorkingDirectory=/usr/share/smart-agent/
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
EOF


echo "Enabling and starting the service..."

systemctl daemon-reload || exit 1
systemctl enable smart-agent || exit 1
systemctl start smart-agent || exit 1

echo
echo "Installation complete! The agent will now try to connect to "
echo "your ContinuousC instance. It should appear as connected in "
echo "the web interface. If not, check the troubleshooting section "
echo "in the documentation."
