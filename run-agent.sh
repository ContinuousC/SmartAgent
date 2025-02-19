#!/bin/sh

instance=$(whoami)-dev
cargo build --release && sudo ./target/release/agent --broker mndev02 --ca-cert /usr/share/smartm/certs/$instance/ca.crt --key /usr/share/smartm/certs/$instance/agent.key --cert /usr/share/smartm/certs/$instance/agent.crt
