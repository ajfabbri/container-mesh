SHELL := /bin/bash
BIN_DIR = bin/

CARGO = cargo
FLAVOR ?= debug
ARCH = x86_64-unknown-linux-gnu

# If FLAVOR is release, set FLAVOR_FLAG to "--release", else make it empty
FLAVOR_FLAG = $(ifeq ($(release,$(FLAVOR)),--release,)

# Define the default target
all: build

# Build rust coordinator and peer
rust:
	{ \
    set -e ;\
	echo "FLAVOR: $(FLAVOR)" ;\
	$(CARGO) build --target $(ARCH) $(FLAVOR_FLAG) ;\
	mkdir -p $(BIN_DIR); \
	cp target/$(ARCH)/$(FLAVOR)/cmesh-peer $(BIN_DIR) ;\
	cp target/$(ARCH)/$(FLAVOR)/cmesh-coordinator $(BIN_DIR) ;\
    sopath=$$(find target/$(ARCH)/$(FLAVOR) -name libdittoffi.so | head -n 1) ;\
    if [[ -z $$sopath ]]; then \
        echo "Error: could not find libdittoffi.so" ;\
        false ;\
    fi ;\
    cp $$sopath $(BIN_DIR) ;\
	}

rust-clean:
	$(CARGO) clean

typescript:
	{ \
	pushd ts ;\
	npm ci ;\
	npm run build ;\
	npm pack ;\
	mkdir -p $(BIN_DIR) ;\
	popd ;\
	cp ts/cmesh_peer-*.tgz $(BIN_DIR) ;\
	}

typescript-clean:
	{ \
	cd ts ;\
	rm -rf node_modules/* dist/* cmesh-peer-*.tgz ;\
	}

# build typescript and rust
build: rust typescript

# Nuke all build and dependency artifacts
clean: rust-clean typescript-clean

