ARG TARGET=x86_64-unknown-linux-musl
# Stage 1: Build the binary
FROM rust:latest AS builder

# Create a new directory for the project
WORKDIR /usr/src/rocks_works

ARG TARGET
RUN rustup target add "$TARGET"

# Copy the source code into the container
COPY . .

# Build the binary
RUN cargo build --release --target "$TARGET"

# Stage 2: Create a smaller image for running the binary
FROM nginx:alpine

ARG TARGET
# Install openssl for generating self-signed certificates
RUN apk add --no-cache openssl

# Generate a self-signed certificate
RUN openssl req -x509 -newkey rsa:4048 -days 365 -nodes -keyout /etc/ssl/private/nginx-selfsigned.key -out /etc/ssl/certs/nginx-selfsigned.crt -subj "/CN=localhost"

# Copy the default nginx configuration
COPY config/nginx.conf /etc/nginx/nginx.conf

# Copy the static HTML content into the container
COPY public /usr/share/nginx/html

# Copy the binary from the builder stage
COPY --from=builder /usr/src/rocks_works/target/$TARGET/release/rocks_svr /usr/local/bin/rocks_svr

# Copy the script to start both services
COPY config/start.sh /start.sh
RUN chmod +x /start.sh

# forward request and error logs to docker log collector
RUN ln -sf /dev/stdout /var/log/nginx/access.log \
    && ln -sf /dev/stderr /var/log/nginx/error.log

# Expose the port the application runs on
EXPOSE 34434
EXPOSE 443

# Set the entry point to run the binary
ENTRYPOINT /start.sh
