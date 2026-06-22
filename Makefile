.PHONY: help init dev dev-backend dev-frontend build run docker docker-stop clean check

help: ## Affiche l'aide
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

init: ## Initialise le projet (config, dépendances)
	@test -d config || cp -r config.example config
	cd backend && cargo fetch
	rm -rf frontend/node_modules
	npm install
	@echo "✓ Projet initialisé. Éditez config/*.yaml puis lancez 'make dev'"

dev-backend: check-port-8080 ## Lance le backend Rust en mode dev
	cd backend && CONFIG_DIR=../config STATIC_DIR=../frontend/build RUST_LOG=gopdash=debug cargo run

dev-frontend: ## Lance le frontend SvelteKit avec hot-reload
	npm run dev

dev: check-port-8080 ## Lance backend + frontend en parallèle (nécessite 'make init')
	@echo "Backend: http://localhost:8080 | Frontend: http://localhost:5173 (proxy /api → backend)"
	@echo "Astuce : arrêtez le container Docker (make docker-stop) si le port 8080 est occupé."
	$(MAKE) -j2 dev-backend dev-frontend

check-port-8080: ## Vérifie que le port 8080 est libre
	@pid=$$(lsof -ti :8080 2>/dev/null); \
	if [ -n "$$pid" ]; then \
		echo "❌ Port 8080 déjà utilisé :"; \
		lsof -i :8080 2>/dev/null | sed '1d' || true; \
		echo ""; \
		echo "Le frontend proxyfie /api vers :8080 — un ancien gopdash ou le container Docker provoque des erreurs 500."; \
		echo "  make docker-stop       # arrêter le container"; \
		echo "  make dev-stop          # tuer le processus local sur :8080"; \
		exit 1; \
	fi

dev-stop: ## Libère le port 8080 (processus gopdash local)
	@pid=$$(lsof -ti :8080 2>/dev/null); \
	if [ -z "$$pid" ]; then \
		echo "Port 8080 déjà libre."; \
	else \
		kill $$pid && echo "✓ Processus $$pid arrêté (port 8080 libéré)."; \
	fi

build-backend: ## Compile le backend en release
	cd backend && cargo build --release

build-frontend: ## Build le frontend statique
	npm run build

build: build-frontend build-backend ## Build complet (frontend + backend)

run: build ## Build et lance le binaire localement
	CONFIG_DIR=./config STATIC_DIR=./frontend/build ./backend/target/release/gopdash

docker: ## Build et lance le container Docker (production)
	docker compose up --build -d

docker-stop: ## Arrête les containers
	docker compose down

check: ## Vérifie backend + frontend
	cd backend && cargo check
	npm run check

clean: ## Nettoie les artefacts de build
	cd backend && cargo clean
	rm -rf frontend/build frontend/.svelte-kit frontend/node_modules node_modules

install-shadcn: ## Installe shadcn-svelte (composants UI additionnels)
	cd frontend && npx shadcn-svelte@latest init

.DEFAULT_GOAL := help
