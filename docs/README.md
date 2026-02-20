# Codebase documentation

Visual codebase map for **mkt-noise-analysis**.

## Viewing

Open `index.html` in a web browser:

```bash
# From project root
start docs/index.html     # Windows
open docs/index.html     # macOS
xdg-open docs/index.html # Linux
```

Or serve locally:

```bash
cd docs && python -m http.server 8000
# Then open http://localhost:8000
```

## Contents

- **index.html** — Visual codebase map: module dependency flow, file tree, module overview, Cargo dependencies, data flow diagram
- **file-reference.html** — File-by-file reference with descriptions

## Requirements

- Modern browser (Chrome, Firefox, Edge, Safari)
- Internet connection for Mermaid.js CDN (for diagram rendering)
