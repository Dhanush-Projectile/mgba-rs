# Default recipe
default:
    @just --list

# Install system dependencies (Ubuntu/Debian)
install-deps:
    sudo dnf update && sudo dnf install -y build-essential cmake zlib-devel git

# Initialize git submodules for mGBA
init-submodules:
    git submodule update --init --recursive

# Run cargo check
check: init-submodules
    cargo check

#clean

clean: 
	cargo clean
