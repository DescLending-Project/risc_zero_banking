# CONFIG
IMAGE_NAME = rbbozkurt/tlsn-verifier
TAG ?= latest
DOCKER_PLATFORM = linux/amd64
COMPOSE_FILE_LOCAL = docker-compose.yml
ENV_FILE = .env

# Build the Docker image for local use
build:
	docker buildx build --platform $(DOCKER_PLATFORM) -t $(IMAGE_NAME):$(TAG) .

# Push the built image to Docker Hub
push: build
	docker push $(IMAGE_NAME):$(TAG)

# Run locally with Docker Compose
up:
	docker compose --env-file $(ENV_FILE) -f $(COMPOSE_FILE_LOCAL) up --build

# Stop local container
down:
	docker compose -f $(COMPOSE_FILE_LOCAL) down
