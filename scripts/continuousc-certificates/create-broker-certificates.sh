#!/bin/bash


alt_names="DNS:mndev02,DNS:mndev02.sit.be,DNS:localhost,IP:127.0.0.1,IP:192.168.10.30"

if ! [[ -e "ca" ]]; then
    echo "Please create the CA first! (create-ca.sh)" >&2
    exit 1
fi

if [[ -e "broker" ]]; then
    echo "The broker directory already exists; exiting!" >&2
    exit 1
fi


function create_cert {

    issuer=$1
    name=$2
    subj=$3
    opts=$4
    
    openssl req -newkey rsa:4096 -keyout "$name.key" -nodes -out "$name.req" -subj "$subj"
    openssl x509 -req -in "$name.req" -CA "$issuer.crt" -CAkey "$issuer.key" -CAcreateserial -out "$name.crt" -days 1095 -extfile <(printf "\n[default]\n$opts")

    cat "$issuer.crt" >> "$name.crt"
    rm -f "$name.req"

}

function create_server_cert {
    create_cert "$1" "$2" "$3" "subjectAltName=$4\nbasicConstraints=CA:FALSE\nkeyUsage=nonRepudiation,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n"
}

mkdir "broker"

create_server_cert "ca/ca" "broker/broker" "/CN=ContinuousC Broker/emailAddress=mnow@si-int.eu" "$alt_names"
cp ca/ca.crt broker
