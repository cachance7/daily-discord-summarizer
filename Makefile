# Define variables
CARGO := cargo
PROJECT_NAME := daily-discord-summarizer
EXECUTABLE_NAME := catchup-bot
INSTALL_PATH := /usr/local/bin
CONFIG_FILE := config.toml
CONFIG_INSTALL_PATH := /etc/$(EXECUTABLE_NAME)

# Default target
all: build

# Build the project
build:
	$(CARGO) build

# Run the project
run: build
	$(CARGO) run

# Test the project
test:
	$(CARGO) test

# Clean the project
clean:
	$(CARGO) clean

# Release build
release:
	$(CARGO) build --release

# Install the binary and config file
install: release
	install -Dm755 target/release/$(PROJECT_NAME) $(INSTALL_PATH)/$(EXECUTABLE_NAME)
	install -Dm644 $(CONFIG_FILE) $(CONFIG_INSTALL_PATH)/$(CONFIG_FILE)

# Format the code
format:
	$(CARGO) fmt

# Lint the code
lint:
	$(CARGO) clippy

# Help message
help:
	@echo "Makefile for $(PROJECT_NAME)"
	@echo ""
	@echo "Usage:"
	@echo "  make [target]"
	@echo ""
	@echo "Targets:"
	@echo "  all       - Build the project (default)"
	@echo "  build     - Build the project"
	@echo "  run       - Run the project"
	@echo "  test      - Test the project"
	@echo "  clean     - Clean the project"
	@echo "  release   - Build the project in release mode"
	@echo "  install   - Install the binary as $(EXECUTABLE_NAME) to $(INSTALL_PATH) and config file to $(CONFIG_INSTALL_PATH)"
	@echo "  format    - Format the code"
	@echo "  lint      - Lint the code"
	@echo "  help      - Display this help message"

# Phony targets
.PHONY: all build run test clean release install format lint help
