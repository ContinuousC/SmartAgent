#!/bin/sh

if ! (. /etc/os-release; echo $NAME $VERSION_ID) | grep -qxF 'CentOS Linux 7'; then
    echo 'This script is intended for CentOS 7 systems; detected another OS!'
    exit 1
fi

if ! [ $(id -u) == 0 ]; then
    echo "This script must be run as root!"
    exit 1
fi


echo "Installing ContinuousC repository..."
cat <<EOF > /etc/yum.repos.d/continuousc.repo || exit 1
[continuousc]
name = ContinuousC Repository
baseurl = {{RepoUrl}}
sslverify = 0
enabled = 1
gpgcheck = 0
#gpgkey = 
EOF


echo "Installing ContinuousC Smart Agent..."
yum install -y smart-agent smart-agent-libs || exit 1


echo "Installing certificates..."

cat <<EOF > /usr/share/smart-agent/certs/ca.crt || exit 1
{{CaCert}}
EOF

cat <<EOF > /usr/share/smart-agent/certs/agent.crt || exit 1
{{AgentCert}}
EOF

cat <<EOF > /usr/share/smart-agent/certs/agent.key || exit 1
{{AgentKey}}
EOF

chmod 700 /usr/share/smart-agent/certs || exit 1
chmod 600 /usr/share/smart-agent/certs/agent.key || exit 1


echo "Installing the service file..."

cat <<EOF > /usr/lib/systemd/system/smart-agent.service || exit 1
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
