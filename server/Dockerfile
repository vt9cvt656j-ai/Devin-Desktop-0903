# --- build stage ---
FROM rust:1-bookworm AS builder
WORKDIR /app
COPY Cargo.toml ./
COPY Cargo.lock* ./
COPY src ./src
COPY migrations ./migrations
COPY static ./static
RUN cargo build --release

# --- runtime stage (rustls everywhere, Python only for media helpers) ---
FROM python:3.12-slim-bookworm
ENV PIP_NO_CACHE_DIR=1 \
    PYTHONDONTWRITEBYTECODE=1 \
    HF_HOME=/var/cache/michael/huggingface
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates ffmpeg \
    && rm -rf /var/lib/apt/lists/*
# Keep the runtime capable of local MusicGen, but store its large model weights in
# a persistent volume on first use instead of baking 2+ GB into every image layer.
RUN pip install edge-tts gradio_client \
    && pip install torch --index-url https://download.pytorch.org/whl/cpu \
    && pip install transformers scipy numpy
WORKDIR /app
COPY --from=builder /app/target/release/michael-backend /usr/local/bin/michael-backend
COPY migrations ./migrations
COPY knowledge ./knowledge
COPY prompts ./prompts
COPY trellis_gen.py ./trellis_gen.py
COPY music_gen.py ./music_gen.py
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
  CMD ["python", "-c", "import urllib.request; urllib.request.urlopen('http://127.0.0.1:8080/health', timeout=3)"]
CMD ["michael-backend"]
