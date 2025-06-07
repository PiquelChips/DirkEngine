BUILD=build
RELEASE=release

.PHONY: run
run: config
	@cmake --build $(BUILD) --config=Debug --target=run

.PHONY: build
build: config
	@cmake --build $(BUILD) --config=Debug

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
	@cmake -S . -B $(BUILD) -DCMAKE_EXPORT_COMPILE_COMMANDS=ON -DGLFW_BUILD_X11=OFF
	@ln -fs $(BUILD)/compile_commands.json .

.PHONY: clean
clean:
	@git clean -dfx
