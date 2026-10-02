FROM rust:1.98-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        clang \
        mold \
        python3-dev \
        python3-venv \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY . .

RUN python3 -m venv /opt/build-venv \
    && /opt/build-venv/bin/pip install --no-cache-dir 'maturin>=1,<2' \
    && /opt/build-venv/bin/maturin build --release --interpreter python3 --out /wheels

FROM python:3.11-slim-bookworm

COPY --from=builder /wheels /wheels
RUN pip install --no-cache-dir /wheels/*.whl \
    && rm -rf /wheels

WORKDIR /workspace
ENTRYPOINT ["cytoscnpy"]
CMD ["--help"]
