BUILD=build
RELEASE=release

.PHONY: run
run: config
	@cmake --build $(BUILD) --config=Debug
	@$(BUILD)/DirkEngine

.PHONY: release
release: config
	@cmake --build $(BUILD) --config=Release
	@echo Creating release...
	@rm -rf $(RELEASE)
	@mkdir $(RELEASE)
	@cp $(BUILD)/DirkEngine $(RELEASE)
	@cp -r ressources $(RELEASE)
	@echo Release created!

.PHONY: config
config:
	@cmake -S . -B $(BUILD) -DCMAKE_EXPORT_COMPILE_COMMANDS=ON
	@ln -fs $(BUILD)/compile_commands.json .
