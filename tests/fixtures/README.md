# Fixtures

- `demo/`: bóveda con texto, lista, cita, tabla, dibujo y un bloque de plugin.
- `canvas-500/`: una nota con 500 bloques (uno de cada diez anclado) para probar
  layout, paneo y zoom.

Se regeneran con:

```bash
cargo run -p folio-tools -- gen-demo --vault tests/fixtures/demo
cargo run -p folio-tools -- gen-canvas --blocks 500 --out tests/fixtures/canvas-500
```

No contienen datos personales. El índice SQLite no se versiona.
