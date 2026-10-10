# Ingestion Worker Dockerfile
# Multi-stage: build Go binary, then copy to minimal runtime

# Build stage
FROM golang:1.23-alpine AS builder

WORKDIR /app

# Install build deps
RUN apk add --no-cache git make gcc musl-dev

# Copy go.mod/go.sum first for layer caching
COPY go.mod go.sum ./
RUN go mod download

# Copy source
COPY . .

# Build static binary
RUN CGO_ENABLED=1 GOOS=linux go build -o /worker ./cmd/worker

# Runtime stage
FROM alpine:3.20

RUN apk add --no-cache ca-certificates tzdata

WORKDIR /app

COPY --from=builder /worker /app/worker
COPY config.yaml.example /app/config.yaml.example

# Non-root user
RUN adduser -D -g '' appuser
USER appuser

EXPOSE 8080

ENTRYPOINT ["/app/worker"]
CMD ["-config", "/app/config.yaml"]