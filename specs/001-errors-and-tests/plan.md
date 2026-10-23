# Implementation Plan: Corrección de errores y cobertura de tests

**Branch**: `fix/errors-and-tests` | **Date**: 2026-10-22 | **Spec**: [spec.md](spec.md)

**Input**: Plan detallado en `docs/plans/errors-and-tests.md` (aprobado por el usuario); constitución 2.0.0 en `.specify/memory/constitution.md`.

## Summary

Corregir los 8 bugs P0 detectados en la revisión del pipeline de `update`
(errores silenciados, sección SHA256 sin delimitar, HTTP sin
`error_for_status`/timeouts, exit codes siempre 0, archivos corruptos en
caché, panic latente, EOF no verificado en el parser) y añadir los tests
unitarios P1 de `parser.rs`, `url.rs`, `configuration.rs`, `integrity.rs` y
`types/repository.rs` exigidos por el Principio III de la constitución.

## Technical Context

**Language/Version**: Rust (edición 2024)

**Primary Dependencies**: tokio (runtime), reqwest (HTTP), nom (parser), pgp (firmas), sha2 (hashes), clap (CLI), url, futures — sin dependencias nuevas.

**Storage**: N/A (archivos en caché local `CACHE_DIR`)

**Testing**: `cargo test` — tests unitarios en línea (`#[cfg(test)]`) junto al código; fixtures en `src/fixtures/` incluidas con `include_str!`; sin red.

**Target Platform**: Linux

**Project Type**: CLI + biblioteca (crate `nya`)

**Constraints**: Sin dependencias nuevas (Principio V). Toda función de red/archivo degrada con `Result`, no `panic!` (Principio IV). Verificación de hashes/firmas vive en `integrity.rs` (Principio II).

**Scale/Scope**: ~8 cambios de comportamiento en 6 archivos fuente + ~6 grupos de tests nuevos. Sin cambios de API pública de la biblioteca (solo `update()` pasa a devolver `Result`).

## Constitution Check

*GATE: verificado antes de implementar.*

- **I. Biblioteca primero**: los cambios de `update()`, parser, `integrity`, etc. residen en la biblioteca; `main.rs` solo propaga el `Result` con `?`. ✅
- **II. Integridad verificada (NO NEGOCIABLE)**: A2 delimita y valida el hash SHA256 (refuerza el principio); A3 hace fallar 4xx/5xx en vez de confiar en contenido no verificado. ✅
- **III. Tests con fixtures**: A2 y A8 llevan sus tests de regresión en el mismo commit (requisito); B1–B6 añaden la cobertura de parsing/extracción/firmas que faltaba. `include_str!` desde `src/fixtures/`, sin red. ✅
- **IV. Errores sobre panics**: A5 propaga `Result`; A7 elimina un `unwrap()`; A6 borra corruptos. ✅
- **V. Simplicidad deliberada**: cero dependencias nuevas; `to_debian_arch` es una extracción de lógica existente. ✅

## Project Structure

### Documentation (this feature)

```text
specs/001-errors-and-tests/
├── spec.md              # User stories P1 (pipeline robusto) y P2 (tests)
├── plan.md              # Este archivo
└── tasks.md             # Generado por /speckit.tasks
```

### Source Code (repository root)

```text
src/
├── main.rs                      # A5: propagar Result de update() con ?
├── commands/update/mod.rs       # A1, A4, A5, A6: pipeline
├── package_info.rs              # A2: delimitar sección SHA256 (+tests)
├── parser.rs                    # A8: EOF estricto (+tests)
├── download.rs                  # A3: error_for_status
├── configuration.rs             # B3: to_debian_arch pura (+tests)
├── integrity.rs                 # B5: tests de hash/validación
├── url.rs                       # B2: tests de format_url
├── types/
│   ├── mod.rs                   # A7: component().unwrap() → ok_or
│   └── repository.rs            # B6: tests de is_valid/getters
└── fixtures/
    ├── sources                  # B1: fixture nueva para parser
    ├── InRelease                # existente
    └── key.pub.asc              # existente
```

## Dependencies Between User Stories

- **US1 (P1)**: sin dependencias de US2; es el MVP (correcciones de bugs).
- **US2 (P2)**: independiente de US1 en su mayoría, excepto:
  - B1 (tests de parser) prueba el EOF estricto de A8 → debe venir después de A8 (o testear el comportamiento previo y ajustarse).
  - B4 (tests de `package_info` error paths) prueba el comportamiento corregido por A2 → después de A2.
- Orden sugerido: US1 completa → US2 (o US2 en paralelo salvo B1/B4).

## Verification Strategy

- Tests de regresión de A2/A8 escritos primero (deben fallar contra el código actual, pasar tras el fix — TDD mínima exigida por la constitución para cambios de parsing/verificación).
- Gates del CI al final: `cargo fmt --all --check`, `cargo clippy --locked --all-targets --all-features -- -D warnings`, `cargo test --locked --all-features` (timeout ≥10 min la primera vez).
