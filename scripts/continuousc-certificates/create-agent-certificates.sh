#!/bin/bash


org=$1
agent=$2

if ! [[ -e "ca" ]]; then
    echo "Please create the CA first! (create-ca.sh)" >&2
    exit 1
fi

if [[ -z "$org" || -z "$agent" ]]; then
    echo "Usage: $0 ORGANIZATION AGENT" >&2
    exit 1
fi

if ! [[ -e "$org" ]]; then
    echo "Please create the backend certificates first! (using create-backend-certificates $org)" >&2
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

function create_client_cert {
    create_cert "$1" "$2" "$3" "basicConstraints=CA:FALSE\nkeyUsage=nonRepudiation,digitalSignature,keyEncipherment\nextendedKeyUsage=clientAuth\n"
}

create_client_cert "$org/backend-ca" "$org/agent-$agent" "/O=$org/CN=$agent"
