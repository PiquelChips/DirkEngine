BUILD=build

.PHONY: build
build: config clangd
	@cmake --build $(BUILD)

.PHONY: run
run: build
	@./$(BUILD)/bin/DirkEngine


.PHONY: config
config:
	@cmake -S . -B $(BUILD) -DCMAKE_EXPORT_COMPILE_COMMANDS=ON

.PHONY: clangd
clangd:
	@ln -fs $(BUILD)/compile_commands.json .
