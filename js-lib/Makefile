.PHONY: build publish

build:
	wasm-pack build --release --target nodejs
	jq '. + {"publishConfig": {"registry": "http://sigit01:4873/"}}' pkg/package.json > pkg/package2.json
	mv -f pkg/package2.json pkg/package.json

publish:
	cd pkg/; npm publish
