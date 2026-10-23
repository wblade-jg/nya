# Tasks: Corrección de errores y cobertura de tests

**Input**: Design documents from `/specs/001-errors-and-tests/`

**Prerequisites**: plan.md (required), spec.md (required), research.md (N/A), data-model.md (N/A), contracts/ (N/A)

**Tests**: Sí — pedidos explícitamente en el plan. Van integrados en las tareas de corrección cuando la constitución lo exige (A2/A8: tests en el mismo commit) y como tareas dedicadas en US2.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- Single project: `src/`, `specs/` at repository root (Rust crate `nya`).

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Rama de trabajo conforme al flujo del repo (AGENTS.md §Flujo de trabajo con git)

- [x] T001 Crear rama `fix/errors-and-tests` desde `main` (HEAD `b2f17fa`) con `git checkout -b fix/errors-and-tests`

> **Nota (sync 2026-10-22)**: la US1 (T001–T011) fue implementada y mergeada en
> `main` vía la rama previa; esta rama recreada continúa con la US2 y polish.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: No hay infraestructura compartida pendiente — el proyecto, el CI y la constitución 2.0.0 ya están en `main`. Esta fase queda vacía a propósito.

**Checkpoint**: Foundation ready — user story implementation can now begin

---

## Phase 3: User Story 1 - Pipeline de `update` robusto y con exit codes correctos (Priority: P1) 🎯 MVP

**Goal**: Corregir los 8 bugs P0: errores silenciados, sección SHA256 sin delimitar, HTTP sin `error_for_status`/timeouts, exit codes siempre 0, archivos corruptos en caché, panic latente, EOF no verificado.

**Independent Test**: `cargo test` en verde con los tests de regresión de A2/A8; ejecutar `nya update` con un `sources` apuntando a un mirror caído y verificar exit ≠ 0 y mensajes de error en stderr.

### Tests for User Story 1 (integration with fixes — constitución III exige mismo commit)

- [x] T002 [US1] En `src/package_info.rs`: añadir tests de regresión de A2 en el bloque `#[cfg(test)]` existente — (a) entrada presente solo en sección `MD5Sum` → `Err`; (b) línea con tamaño no numérico tras cabecera `SHA256:` → se ignora, no aborta; (c) path `notmain/binary-amd64/Packages` no coincide con `main/binary-amd64/Packages`; (d) hash de longitud/hex inválidos → `Err`.
- [x] T003 [US1] En `src/parser.rs`: añadir tests de regresión de A8 en un nuevo bloque `#[cfg(test)]` — (a) línea malformada tras el último bloque → `Err`; (b) fichero que termina en comentario `#` sin newline final → `Ok`.

### Implementation for User Story 1

- [x] T004 [P] [US1] En `src/download.rs`: añadir `.error_for_status()?` a la petición de `reqwest` para que un 404/500 se trate como `Err` en vez de éxito.
- [x] T005 [P] [US1] En `src/types/mod.rs`: reemplazar `self.repository.component().unwrap()` por `self.repository.component().ok_or(...)?` en `target_package_path`.
- [x] T006 [US1] En `src/commands/update/mod.rs`: reemplazar `Client::new()` por `Client::builder()` con timeout 30s total / 10s connect.
- [x] T007 [US1] ~~Loguear con `eprintln!` los errores de InRelease de la primera etapa~~ — **superseded by log-polish**: el rediseño de errores unificados con causas por mirror (`mirror_error`) superó esta formulación.
- [x] T008 [US1] `update()` → `Result`; contar fallos por repositorio, resumen final y exit ≠ 0 vía `?` en `main.rs`; propagar errores de lectura/parseo de `sources`.
- [x] T009 [US1] Borrar el archivo corrupto (`std::fs::remove_file`) cuando `downloaded_hash != package_info.hash()`.
- [x] T010 [US1] Implementar A2 en `package_info.rs` — delimitar sección SHA256, saltar tamaños no numéricos, validar len 64/hex, igualdad exacta de path.
- [x] T011 [US1] Implementar A8 en `parser.rs` — EOF estricto con `io::ErrorKind::InvalidData` si queda contenido sin consumir.

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently — `cargo test` en verde, exit codes correctos, sin archivos corruptos.

---

## Phase 4: User Story 2 - Tests unitarios de parsing, URLs, arquitectura e integridad (Priority: P2)

**Goal**: Cumplir el Principio III de la constitución 2.0.0: tests en línea con fixtures para `parser.rs`, `url.rs`, `configuration.rs` (refactor `to_debian_arch`) e `integrity.rs` (alcance pragmático; `types/repository.rs` descartada por trivial).

**Independent Test**: `cargo test` en verde; cada módulo tiene al menos un test de camino feliz y un test de error.

### Tests for User Story 2

> **Alcance pragmático (sync 2026-10-22)**: recortada T015 (getters triviales
> de `Repository`, sin valor de regresión); T016 reducida al refactor +
> tests mínimos; T014 sin el caso `key_path` inexistente (ya cubierto por
> `missing_key_file_error_names_the_path`); añadidos T021–T023 por análisis
> de código (aserciones de contenido y casos límite descubiertos).

- [x] T012 [P] [US2] En `src/fixtures/`: crear la fixture `sources` (texto deb822) con 2 repositorios (el primero con 2 URIs para multi-mirror), comentarios `#`, claves desconocidas (`Types`, `Architectures`) y los campos `URIs`/`Suites`/`Components`/`Signed-By` completos; termina con newline (requerido por el EOF estricto de A8).
- [x] T013 [P] [US2] En `src/url.rs`: añadir bloque `#[cfg(test)]` con tabla de casos puros para `format_url` — URL normal con path → nombre plano correcto; URL sin host (`file:///x`) → `None`; URL inválida → `None`; trailing slash; puerto descartado (`http://127.0.0.1:8080/x` → mismo nombre que sin puerto); barras duplicadas → `__`; documentar la colisión `/a/b` vs `/a_b` (mismo nombre de fichero) como test que documenta la limitación actual.
- [x] T014 [P] [US2] En `src/integrity.rs`: añadir tests al bloque `#[cfg(test)]` existente — `calculate_file_hash` sobre `src/fixtures/InRelease` con valor SHA256 esperado hardcodeado (obtener con `sha256sum src/fixtures/InRelease` al implementar); fichero >8192 bytes (loop multi-chunk) vía fichero temporal; fichero inexistente → `Err` con la ruta; `validate_signature_file` con `signed_path` inexistente → `Err` con la ruta; clave corrupta (texto basura) → `Err`; clave válida (`key.pub.asc`) + texto sin firmar → `Err`. ~~`key_path` inexistente~~ omitido: ya cubierto.
- [x] T016 [US2] En `src/configuration.rs`: extraer `pub(crate) fn to_debian_arch(arch: &str) -> Result<&'static str, String>` pura (sin panic) y hacer que `architecture()` la envuelva (elimina el `panic!` → Principio IV); añadir bloque `#[cfg(test)]` mínimo — tabla `x86_64→amd64`, `aarch64→arm64` y arquitectura no soportada → `Err` sin panic.
- [x] T017 [US2] En `src/parser.rs`: añadir tests usando la fixture `src/fixtures/sources` de T012 (vía `include_str!`) — parseo válido → 2 repos con **aserciones de contenido** de todos los campos (uris, suites, components, signed_by); comentarios y claves desconocidas ignorados (implícito); campo `Components` vacío/ausente → `Err`; versión CRLF (reemplazar `\n` por `\r\n` en runtime) → parsea igual; **folding** (test documental del comportamiento actual): continuación indentada de `URIs:` se descarta silenciosamente → `Ok` con solo la primera URI. Depends on T011 (A8) y T012 (fixture).
- [x] T018 [US2] En `src/package_info.rs`: añadir tests de B4 al bloque existente — sin sección `SHA256` en el contenido → `Err` (asertar mensaje); hash con caracteres no-hex → `Err` (ya cubierto por T002, verificar). Depends on T010 (A2).
- [x] T021 [P] [US2] En `src/package_info.rs`: reescribir `verify_package_file_info` — asserts completos de `hash()` (valor entero), `size()` y `path()`; hoy solo asserta `hash.len() == 64`.
- [x] T022 [P] [US2] En `src/package_info.rs`: test — tamaño no numérico en la línea del path **objetivo** → `Err` (rama de tamaño de la línea objetivo; los tests existentes solo cubren skip de líneas no-objetivo).
- [x] T023 [P] [US2] En `src/package_info.rs`: test documental (comportamiento actual) — hash 64-hex en MAYÚSCULAS → `Ok`; documenta la futura falsa discrepancia con `calculate_file_hash` (minúsculas) en la comparación de `update`.

**Checkpoint**: All user stories should now be independently functional — US1 con bugs corregidos, US2 con cobertura de tests del Principio III.

---

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Gates de calidad y limpieza final

- [x] T019 Ejecutar localmente los gates del CI (AGENTS.md §Comandos): `cargo fmt --all --check`, `cargo clippy --locked --all-targets --all-features -- -D warnings`, `cargo test --locked --all-features` (timeout ≥10 min la primera compilación); corregir cualquier warning nuevo.
- [x] T020 ~~Revisar mensajes en inglés en archivos tocados~~ — ya cumplida: los mensajes residuales de `integrity.rs` y `parser.rs` (mencionados en `log-polish.md`) fueron traducidos en los commits mergeados en `main`; verificado por grep en el código actual (todos los `format!`/`eprintln!`/`panic!` de usuario están en español).

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Empty — no blocking prerequisites
- **User Story 1 (Phase 3)**: COMPLETA (T001–T011 mergeadas en `main`)
- **User Story 2 (Phase 4)**: T017 depende de T011 (hecha) y T012; T018 depende de T010 (hecha); T013/T014/T016/T021–T023 independientes
- **Polish (Phase 5)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: COMPLETA
- **User Story 2 (P2)**: T012 bloquea T017; el resto es paralelizable

### Within User Story 2

- T012 primero (fixture necesaria para T017)
- T013, T014, T016, T018+T021+T022+T023 paralelizables (archivos distintos; los de package_info van en bloque por ser el mismo archivo)
- T017 al final (depende de T012)

### Parallel Opportunities

- T013 [P] + T014 [P] + T016 [P] + (T018+T021+T022+T023 en package_info.rs) — 4 archivos distintos en paralelo
- T017 tras T012

---

## Implementation Strategy

### Incremental Delivery (actualizado)

1. ~~Setup → rama lista~~ (hecho; rama recreada desde `main` con US1 mergeada)
2. ~~User Story 1 → bugs corregidos, exit codes correctos~~ (hecho, mergeado en `main`)
3. User Story 2 (alcance pragmático) → cobertura de tests del Principio III
4. Polish → gates en verde, mensajes en español

### Suggested Commit Mapping (actualizado)

1. `docs(spec): sync tasks.md with implemented state` (Fase 0, puede ir primero)
2. `test(url): table-driven format_url cases` (T013)
3. `test(integrity): hash and validation error cases` (T014)
4. `refactor(configuration): extract pure arch mapping (+tests)` (T016)
5. `test(package_info): strengthen happy path and error cases` (T018+T021+T022+T023)
6. `test(parser): cover valid/invalid sources parsing with fixture` (T012+T017)
7. `fix(i18n): translate residual error messages to Spanish` (T020)

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to user story for traceability
- Constitución III: tests en línea junto al código; fixtures en `src/fixtures/` vía `include_str!`; sin red
- T015 eliminada del alcance pragmático (getters triviales de `Repository` sin valor de regresión)
- **Desviación Principio V registrada**: se añadió `indicatif = "0.18.6"` (barras de progreso) vía `docs/plans/log-polish.md` (aprobado por el usuario), contradiciendo "sin dependencias nuevas" del plan original de esta spec. Waiver implícito por aprobación.
- Tests documentales (T017 folding, T023 hash en mayúsculas) son tests NORMALES que asientan el comportamiento actual; si se arregla el bug, fallarán y obligarán a actualizarlos.
- No hacer push ni PR sin petición explícita del usuario (AGENTS.md §Flujo de trabajo con git)
