#!/bin/bash


org=$1
alt_names="DNS:mndev02,DNS:mndev02.sit.be,DNS:localhost,IP:127.0.0.1,IP:192.168.10.30"

if ! [[ -e "ca" ]]; then
    echo "Please create the CA first! (create-ca.sh)" >&2
    exit 1
fi

if [[ -z "$org" ]]; then
    echo "Usage: $0 ORGANIZATION" >&2
    exit 1
fi

if [[ -e "$org" ]]; then
    echo "The $org directory already exists; exiting!" >&2
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

function create_ca_cert {
    create_cert "$1" "$2" "$3" "basicConstraints=CA:TRUE\nkeyUsage=critical,cRLSign,digitalSignature,keyCertSign\n"
}

function create_client_cert {
    create_cert "$1" "$2" "$3" "subjectAltName=$4\nbasicConstraints=CA:FALSE\nkeyUsage=nonRepudiation,digitalSignature,keyEncipherment\nextendedKeyUsage=clientAuth\n"
}

function create_server_cert {
    create_cert "$1" "$2" "$3" "subjectAltName=$4\nbasicConstraints=CA:FALSE\nkeyUsage=nonRepudiation,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n"
}

mkdir "$org"

create_ca_cert     "ca/ca"           "$org/backend-ca"            "/O=$org/CN=ContinuousC Backend CA/emailAddress=mnow@si-int.eu"      "$alt_names"
create_server_cert "$org/backend-ca" "$org/server"                "/O=$org/CN=mndev02/emailAddress=mnow@si-int.eu"                     "$alt_names"
create_client_cert "$org/backend-ca" "$org/backend"               "/O=$org/CN=ContinuousC Backend/emailAddress=mnow@si-int.eu"         "$alt_names"
create_server_cert "$org/backend-ca" "$org/dbdaemon"              "/O=$org/CN=ContinuousC Database Daemon/emailAddress=mnow@si-int.eu" "$alt_names"
create_server_cert "$org/backend-ca" "$org/metrics-engine-server" "/O=$org/CN=ContinuousC Metrics Engine/emailAddress=mnow@si-int.eu"  "$alt_names"
create_client_cert "$org/backend-ca" "$org/metrics-engine-client" "/O=$org/CN=ContinuousC Metrics Engine/emailAddress=mnow@si-int.eu"  "$alt_names"
cp ca/ca.crt "$org"
