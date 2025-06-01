.ONESHELL: cmake

cmake:
	@mkdir -p build
	@cd build
	@cmake ..
	@cmake --build . --target run

EXE = DirkEngine

CXXFLAGS = -std=c++17 -O2
DEPFLAGS = -MMD -MP -I ${VULKAN_SDK}/include -I include
LDFLAGS = -lvulkan -lglfw

OUT = out
BIN = $(OUT)/bin
OBJ = $(OUT)/obj
SRC = src

SOURCES := $(wildcard $(SRC)/*.cpp)
OBJECTS := $(patsubst $(SRC)/%.cpp, $(OBJ)/%.o, $(wildcard $(SRC)/*.cpp))
DEPENDS := $(OBJECTS:.o=.d)

.PHONY: all
all: $(BIN)/$(EXE)

$(BIN)/$(EXE): out $(OBJECTS)
	@g++ $(LDFLAGS) $(OBJECTS) -o $@

$(OBJ)/%.o:	$(SRC)/%.cpp
	@g++ $(DEPFLAGS) $(CXXFLAGS) -c -o $@ $<

out:
	@mkdir -p $(OBJ)
	@mkdir -p $(BIN)

.PHONY: rebuild
rebuild: clean $(BIN)/$(EXE)

.PHONY: run
run: $(BIN)/$(EXE)
	@./$(BIN)/$(EXE)

.PHONY: clean
clean:
	@rm -r $(OUT)

-include $(DEPENDS)
