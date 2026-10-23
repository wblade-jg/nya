# Feature Specification: Corrección de errores y cobertura de tests

**Feature Branch**: `001-errors-and-tests`

**Created**: 2026-10-22

**Status**: Draft

**Input**: Plan aprobado en `docs/plans/errors-and-tests.md` (alcance Fase A + Fase B).

## User Scenarios & Testing

### User Story 1 - Pipeline de `update` robusto y con exit codes correctos (Priority: P1)

Como usuario de la CLI (`nya update`), quiero que el pipeline reporte todos
los fallos (descargas caídas, hashes incorrectos, HTTP 4xx/5xx, parses
malformados), que no deje archivos corruptos en la caché y que el proceso
salga con código ≠ 0 si cualquier repositorio falla, para poder detectar
fallos en CI/scripts y confiar en la caché local.

**Why this priority**: Son los bugs P0 detectados en la revisión; afectan a
la corrección funcional y a la seguridad de la verificación (sección SHA256
sin delimitar). Sin esto, `update` silencia errores y siempre sale con 0.

**Independent Test**: `cargo test` en verde con los tests de regresión de
A2/A8; ejecutar `nya update` con un `sources` apuntando a un mirror caído y
verificar exit ≠ 0 y mensajes de error en stderr.

**Acceptance Scenarios**:

1. **Given** un `sources` con un repositorio cuyo mirror devuelve 404, **When** se ejecuta `nya update`, **Then** el error HTTP se loguea y el proceso sale con código ≠ 0.
2. **Given** un InRelease cuya sección SHA256 no contiene la entrada objetivo (solo está en MD5Sum), **When** se extrae la info del paquete, **Then** se devuelve `Err` (nunca un hash de otra sección).
3. **Given** un fichero `sources` con una línea malformada tras el último bloque, **When** se parsea, **Then** se devuelve `Err` (no se pierde en silencio).
4. **Given** un Packages descargado cuyo hash no coincide, **When** se detecta el mismatch, **Then** el archivo corrupto se borra de la caché antes de reintentar en el siguiente mirror.

---

### User Story 2 - Tests unitarios de parsing, URLs, arquitectura e integridad (Priority: P2)

Como mantenedor del proyecto, quiero que `parser.rs`, `url.rs`,
`configuration.rs`, `integrity.rs` y `types/repository.rs` tengan tests
unitarios en línea con fixtures, para cumplir el Principio III de la
constitución y poder refactorizar sin miedo a romper el parsing deb822.

**Why this priority**: Es el cumplimiento del Principio III (constitución
2.0.0 registra los tests de parser como pendientes). Prioridad P2 porque no
arregla bugs activos, sino que previene regresiones futuras.

**Independent Test**: `cargo test` en verde con los tests de B1–B6; cada
módulo tiene al menos un test de camino feliz y un test de error.

**Acceptance Scenarios**:

1. **Given** la fixture `src/fixtures/sources` con 2 repos, comentarios y claves desconocidas, **When** se parsea con `parse_file`, **Then** se devuelven 2 repos con los campos correctos.
2. **Given** una URL sin host (`file:///x`), **When** se llama a `format_url`, **Then** se devuelve `None`.
3. **Given** la arquitectura `"riscv64"`, **When** se llama a `to_debian_arch`, **Then** se devuelve `Err` (sin panic).
4. **Given** un fichero inexistente, **When** se llama a `calculate_file_hash`, **Then** se devuelve `Err`.

---

### Edge Cases

- Qué pasa cuando el `sources` está vacío (0 repositorios): `update` debe salir con 0 (nada que hacer, no es un error).
- Qué pasa cuando `CACHE_DIR` no es escribible: hoy `expect` en `configuration.rs` (fuera de alcance, Fase C).
- Fichero `sources` que termina en comentario `#` sin newline final: debe seguir parseando OK (comportamiento preservado por A8).
- Hash con caracteres no-hex en el InRelease: debe devolver `Err`, no aceptarlo.

## Requirements

### Functional Requirements

- **FR1**: `update` MUST devolver `Result` y el proceso MUST salir con código ≠ 0 si algún repositorio falla.
- **FR2**: Los errores de descarga de InRelease de la primera etapa del pipeline MUST loguearse con `eprintln!` antes de descartarse.
- **FR3**: La extracción de hash SHA256 del InRelease MUST delimitarse a la sección (cortar en el siguiente campo), ignorar tamaños no numéricos, validar longitud 64 y hex, y usar igualdad exacta de path.
- **FR4**: La descarga HTTP MUST tratar 4xx/5xx como error (`error_for_status`) y usar timeouts (30s total, 10s connect).
- **FR5**: Un archivo con hash mismatch MUST borrarse de la caché antes de reintentar.
- **FR6**: El parser MUST exigir EOF (resto vacío tras consumir comentarios/espacios finales).
- **FR7**: `parser.rs` MUST tener tests con fixture `src/fixtures/sources` (camino feliz, claves ignoradas, campos ausentes, CRLF, EOF).
- **FR8**: `url.rs`, `configuration.rs` (mapeo de arquitectura puro), `integrity.rs` (hash + errores de validación) y `types/repository.rs` MUST tener tests unitarios en línea.

### Key Entities (from existing code)

- **Repository** (`src/types/repository.rs`): uris, suites, components, signed_by; `is_valid()`, `suite()`, `component()`.
- **PackageFileInfo** (`src/types/mod.rs`): hash, size, path extraídos del InRelease.
- **HashType** (`src/types/mod.rs`): MD5/SHA1/SHA256/SHA512 con `as_str()` y `size()`.
