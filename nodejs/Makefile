.PHONY: publish

agent_connector.js: agent_connector.tmpl.js $(shell find ../Rust/agent-api ../Rust/broker-api -name '*.rs')
	cd ../Rust && cargo run --bin agent-api ../Node/agent_connector.tmpl.js

publish: agent_connector.js
	npm publish
