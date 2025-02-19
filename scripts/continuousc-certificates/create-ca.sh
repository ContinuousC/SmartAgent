#!/bin/bash

if [[ -e "ca" ]]; then
    echo "The CA directory already exists; exiting!" >&2
    exit 1
fi

mkdir ca
openssl req -x509 -newkey rsa:4096 -nodes -keyout ca/ca.key -out ca/ca.crt -days 1095 -subj '/CN=ContinuousC Test CA/emailAddress=mnow@si-int.eu' -addext 'subjectAltName=DNS:mndev02,DNS:mndev02.sit.be,DNS:localhost,IP:127.0.0.1,IP:192.168.10.30'
