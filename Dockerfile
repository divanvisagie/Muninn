# Use the official Rust image as the base image
FROM rust:latest

# Set the working directory inside the container
WORKDIR /app

# Copy the Cargo.toml and Cargo.lock files to the container
COPY Cargo.toml Cargo.lock ./

# Copy the source code to the container
COPY src ./src

# Build the application
RUN cargo build --release

EXPOSE 8080

# Set the entrypoint command to run the application
CMD ["cargo", "run", "--release"]
