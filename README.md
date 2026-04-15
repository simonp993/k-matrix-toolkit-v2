# K-Matrix Toolkit v2

Web-based automotive communication matrix (K-Matrix) search tool.

**Stack:** Rust (Axum) backend + Next.js frontend with Porsche Design System.

## Quick Start

```bash
docker compose up
# Open http://localhost:3000
```

## Documentation

- [Functional Specification](docs/01_functional_spec.md)
- [Technical Specification](docs/02_technical_spec.md)

## Structure

```
├── old_source/     # Original Python/PyQt6 tool (reference)
├── backend/        # Rust backend (Axum + calamine + can-dbc)
└── frontend/       # Next.js + Porsche Design System
```

## Old Source

The `old_source/` directory contains the original Python/PyQt6 desktop application for reference. The K-Matrizen example data is in the sibling directory of this repository.
