#!/bin/bash

password=$(pwgen -sn1 32)
echo $password > password.txt

rm -f *.crt *.key *.req *.srl

openssl req -x509 -newkey rsa:4096 -keyout ca.key -out ca.crt -passout pass:$password -days 365 -subj '/CN=ContinuousC CA/emailAddress=mnow@si-int.eu' -extensions exts -config <(cat /etc/ssl/openssl.cnf <(printf '\n[exts]\nbasicConstraints=critical,CA:TRUE\nsubjectKeyIdentifier=hash\nauthorityKeyIdentifier=keyid:always,issuer\nkeyUsage=cRLSign,keyCertSign'))

openssl req -newkey rsa:4096 -keyout server.key -nodes -out server.req -subj '/CN=mndev02/emailAddress=mdp@si-int.eu'
openssl x509 -req -in server.req -CA ca.crt -CAkey ca.key -CAcreateserial -out server.crt -passin pass:$password -days 365 -extfile <(printf '\n[default]\nsubjectAltName=DNS:mndev02,DNS:mndev02.sit.be,IP:127.0.0.1,IP:192.168.10.30\nbasicConstraints=CA:FALSE\nkeyUsage=nonRepudiation,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n')

openssl req -newkey rsa:4096 -keyout agent.key -nodes -out agent.req -days 365 -subj '/O=si/CN=test-agent/emailAddress=mdp@si-int.eu'
openssl x509 -req -in agent.req -CA ca.crt -CAkey ca.key -CAcreateserial -out agent.crt -passin pass:$password -days 365 -extfile <(printf '\n[default]\nbasicConstraints=CA:FALSE\nkeyUsage=nonRepudiation,digitalSignature,keyEncipherment\nextendedKeyUsage=clientAuth\n')

openssl req -newkey rsa:4096 -keyout agent2.key -nodes -out agent2.req -days 365 -subj '/O=si/CN=test-agent-ssh/emailAddress=mdp@si-int.eu'
openssl x509 -req -in agent2.req -CA ca.crt -CAkey ca.key -CAcreateserial -out agent2.crt -passin pass:$password -days 365 -extfile <(printf '\n[default]\nbasicConstraints=CA:FALSE\nkeyUsage=nonRepudiation,digitalSignature,keyEncipherment\nextendedKeyUsage=clientAuth\n')

openssl req -newkey rsa:4096 -keyout backend.key -nodes -out backend.req -days 365 -subj '/O=si/CN=backend/emailAddress=mdp@si-int.eu'
openssl x509 -req -in backend.req -CA ca.crt -CAkey ca.key -CAcreateserial -out backend.crt -passin pass:$password -days 365 -extfile <(printf '\n[default]\nbasicConstraints=CA:FALSE\nkeyUsage=nonRepudiation,digitalSignature,keyEncipherment\nextendedKeyUsage=clientAuth\n')

