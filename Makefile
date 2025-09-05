# ==== CONFIGURATION ====
BACKEND_DIR := backend
FRONTEND_DIR := frontend
PROTO_DIR := proto
OUT_DIR := src/generated

RUST_BIN := $(BACKEND_DIR)/target/debug/backend
NEXT_CMD := npm --prefix $(FRONTEND_DIR)

PROTOC := protoc
FRONTEND_OUT := frontend/src/proto

PROTOC_GEN_CONNECT := ./node_modules/.bin/protoc-gen-connect-es
PROTOC_GEN_TS := ./node_modules/.bin/protoc-gen-es

# ==== DEFAULT ====
.PHONY: all
all: build-backend build-frontend

# ==== PROTOC ====

.PHONY: proto
proto:
	@echo "Generating TS/Connect code..."
	$(PROTOC) \
		--proto_path=$(PROTO_DIR) \
		--proto_path=$(PROTO_DIR)/third_party \
		--plugin=protoc-gen-connect-es=$(PROTOC_GEN_CONNECT) \
		--plugin=protoc-gen-es=$(PROTOC_GEN_TS) \
		--connect-es_out=$(FRONTEND_OUT) \
		--es_out=$(FRONTEND_OUT) \
		$(PROTO_DIR)/*.proto

# ==== BACKEND ====
.PHONY: build-backend
build-backend:
	@echo "Building Rust backend..."
	cargo build --manifest-path $(BACKEND_DIR)/Cargo.toml

.PHONY: run-backend
run-backend:
	@echo "Running Rust backend..."
	cargo run --manifest-path $(BACKEND_DIR)/Cargo.toml

# ==== FRONTEND ====
.PHONY: build-frontend
build-frontend:
	@echo "Building Next.js frontend..."
	$(NEXT_CMD) run build

.PHONY: run-frontend
run-frontend:
	@echo "Running Next.js frontend..."
	$(NEXT_CMD) run dev

# ==== CLEAN ====
.PHONY: clean
clean:
	@echo "Cleaning backend and frontend..."
	cargo clean --manifest-path $(BACKEND_DIR)/Cargo.toml
	rm -rf $(FRONTEND_DIR)/.next
	rm -rf $(FRONTEND_DIR)/src/proto/*
	rm -rf $(BACKEND_DIR)/src/proto/*

# ==== FULL REBUILD ====
.PHONY: rebuild
rebuild: clean proto all
