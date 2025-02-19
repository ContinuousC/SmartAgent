.PHONY: install install-agent-api install-broker install-tools

install: install-agent-api install-broker install-tools

install-agent-api:
	cargo build --release --bin agent-api
	sudo install -m755 target/release/agent-api /usr/local/bin/

install-broker:
	cargo build --release --bin broker
	sudo install -m755 target/release/broker /usr/local/bin/broker
	sudo systemctl restart broker

# install-agent:
# 	sudo install -m755 target/release/agent-api /usr/local/bin/
# 	sudo systemctl restart continuousc-agent
# 	sudo systemctl restart continuousc-agent-ssh

install-tools:
	cargo build --release --bin type_check
	cargo build --release --bin parse_expr
	cargo build --release --bin parse_unit
	cargo build --release --package py-lib
	cargo build --release --package py3-lib
	sudo install -m755 target/release/type_check /usr/bin/smartm_type_check
	sudo install -m755 target/release/parse_expr /usr/bin/smartm_parse_expr
	sudo install -m755 target/release/parse_unit /usr/bin/smartm_parse_unit
	sudo install -m755 target/release/libpy_lib.so /usr/lib/python2.7/smart_agent.so
	sudo install -m755 target/release/libpy3_lib.so /usr/lib/python3/dist-packages/smart_agent.so
