.DEFAULT_GOAL := help

.PHONY: help install dev api frontend build test lint fmt-check

help: ## Show available commands
	@awk 'BEGIN { FS = ":.*## " } /^[a-zA-Z0-9_-]+:.*## / { printf "%-12s %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

install: ## Install root and frontend dependencies
	npm install
	npm --prefix frontend install

dev: ## Start the API and frontend together
	npm run dev

api: ## Start the Rust API
	npm run dev:api

frontend: ## Start the Vite frontend
	npm run dev:frontend

build: ## Build the frontend
	npm run build:frontend

test: ## Run the Rust workspace tests
	npm run test:rust

lint: ## Lint the frontend
	npm --prefix frontend run lint

fmt-check: ## Check Rust formatting
	cargo fmt --manifest-path rust/Cargo.toml --all -- --check
