.ONESHELL: build run
.PHONY: build run

build:
	@mkdir -p build
	@cd build
	@cmake ..
	@cmake --build .

run:
	@mkdir -p build
	@cd build
	@cmake ..
	@cmake --build . --target run
