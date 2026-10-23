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

- [ ] T001 Crear rama `fix/errors-and-tests` desde `main` (HEAD `b2f17fa`) con `git checkout -b fix/errors-and-tests`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: No hay infraestructura compartida pendiente — el proyecto, el CI y la constitución 2.0.0 ya están en `main`. Esta fase queda vacía a propósito.

**Checkpoint**: Foundation ready — user story implementation can now begin

---

## Phase 3: User Story 1 - Pipeline de `update` robusto y con exit codes correctos (Priority: P1) 🎯 MVP

**Goal**: Corregir los 8 bugs P0: errores silenciados, sección SHA256 sin delimitar, HTTP sin `error_for_status`/timeouts, exit codes siempre 0, archivos corruptos en caché, panic latente, EOF no verificado.

**Independent Test**: `cargo test` en verde con los tests de regresión de A2/A8; ejecutar `nya update` con un `sources` apuntando a un mirror caído y verificar exit ≠ 0 y mensajes de error en stderr.

### Tests for User Story 1 (integration with fixes — constitución III exige mismo commit)

- [ ] T002 [US1] En `src/package_info.rs`: añadir tests de regresión de A2 en el bloque `#[cfg(test)]` existente — (a) entrada presente solo en sección `MD5Sum` → `Err`; (b) línea con tamaño no numérico tras cabecera `SHA256:` → se ignora, no aborta; (c) path `notmain/binary-amd64/Packages` no coincide con `main/binary-amd64/Packages`; (d) hash de longitud/hex inválidos → `Err`. Estos tests deben FALLAR contra el código actual.
- [ ] T003 [US1] En `src/parser.rs`: añadir tests de regresión de A8 en un nuevo bloque `#[cfg(test)]` — (a) línea malformada tras el último bloque → `Err`; (b) fichero que termina en comentario `#` sin newline final → `Ok`. El test (a) debe FALLAR contra el código actual.

### Implementation for User Story 1

- [ ] T004 [P] [US1] En `src/download.rs:12`: añadir `.error_for_status()?` a la petición de `reqwest` para que un 404/500 se trate como `Err` en vez de éxito.
- [ ] T005 [P] [US1] En `src/types/mod.rs:86`: reemplazar `self.repository.component().unwrap()` por `self.repository.component().ok_or("...")?` en `target_package_path` (cambiar firma a `Result<String, Box<dyn Error + Send + Sync>>` y ajustar el call site en `src/commands/update/mod.rs:105` para propagar con `?`).
- [ ] T006 [US1] En `src/commands/update/mod.rs:10`: reemplazar `Client::new()` por `Client::builder().timeout(Duration::from_secs(30)).connect_timeout(Duration::from_secs(10)).build()?` (importar `std::time::Duration`); `execute_pipeline` debe aceptar/devolver lo necesario para propagar el error de construcción.
- [ ] T007 [US1] En `src/commands/update/mod.rs:18`: loguear los errores de descarga de `InRelease` de la primera etapa con `eprintln!` antes de descartarlos (mismo patrón que el `eprintln!` de las líneas 43–45), en vez de `filter_map(|result| async move { result.ok() })` silencioso.
- [ ] T008 [US1] En `src/commands/update/mod.rs:53` y `src/main.rs:31`: cambiar `pub async fn update(config: &Configuration)` a `pub async fn update(config: &Configuration) -> Result<(), Box<dyn std::error::Error>>`; hacer que `execute_pipeline` cuente fallos y, si algún repositorio falla, emita un resumen final con `eprintln!` y devuelva `Err`; propagar el error de lectura/parseo de `sources` (hoy solo hace `eprintln!`); en `main.rs`, propagar con `?` para obtener exit ≠ 0.
- [ ] T009 [US1] En `src/commands/update/mod.rs:164-170`: borrar el archivo corrupto (`std::fs::remove_file`) antes del `continue` cuando `downloaded_hash != package_info.hash()`.
- [ ] T010 [US1] En `src/package_info.rs:9-45`: implementar A2 — delimitar la sección SHA256 (cortar el análisis en el siguiente campo nuevo: línea no indentada no vacía), saltar entradas con tamaño no numérico en vez de abortar con `?`, validar `hash.len() == 64` y caracteres hex, y usar igualdad exacta de `path` (quitar `ends_with`). Los tests T002 deben pasar.
- [ ] T011 [US1] En `src/parser.rs:113`: implementar A8 — EOF estricto: tras `parse_file`, consumir comentarios (`#`) y espacios finales y exigir resto vacío; si queda contenido no consumible, devolver `Err` de `io::ErrorKind::InvalidData`. El test T003(a) debe pasar y T003(b) debe seguir pasando.

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently — `cargo test` en verde, exit codes correctos, sin archivos corruptos.

---

## Phase 4: User Story 2 - Tests unitarios de parsing, URLs, arquitectura e integridad (Priority: P2)

**Goal**: Cumplir el Principio III de la constitución 2.0.0: tests en línea con fixtures para `parser.rs`, `url.rs`, `configuration.rs`, `integrity.rs` y `types/repository.rs`.

**Independent Test**: `cargo test` en verde; cada módulo tiene al menos un test de camino feliz y un test de error.

### Tests for User Story 2

- [ ] T012 [P] [US2] En `src/fixtures/`: crear la fixture `sources` (texto deb822) con 2 repositorios (el primero con 2 URIs para multi-mirror), comentarios `#`, claves desconocidas (`Types`, `Architectures`) y los campos `URIs`/`Suites`/`Components`/`Signed-By` completos; incluirla vía `include_str!` en los tests de T016.
- [ ] T013 [P] [US2] En `src/url.rs`: añadir bloque `#[cfg(test)]` con tabla de casos puros para `format_url` — URL normal con path → nombre plano correcto; URL sin host (`file:///x`) → `None`; URL inválida → `None`; trailing slash; documentar la colisión `/a/b` vs `/a_b` (mismo nombre de fichero) como test que documenta la limitación actual.
- [ ] T014 [P] [US2] En `src/integrity.rs`: añadir tests al bloque `#[cfg(test)]` existente — `calculate_file_hash` sobre `src/fixtures/InRelease` con valor SHA256 esperado hardcodeado (obtener con `sha256sum src/fixtures/InRelease` al implementar); fichero inexistente → `Err`; `validate_signature_file` con `signed_path` inexistente → `Err`; `validate_signature_file` con `key_path` inexistente → `Err`; clave válida (`key.pub.asc`) + texto sin firmar (string arbitrario) → `Err`.
- [ ] T015 [P] [US2] En `src/types/repository.rs`: añadir bloque `#[cfg(test)]` — `Repository::new()` (vacío) → `is_valid() == false`; repo con solo `uris` → `is_valid() == false`; repo completo → `is_valid() == true`; `suite()` devuelve el primero de `suites`; `component()` devuelve el primero de `components`.
- [ ] T016 [US2] En `src/configuration.rs`: extraer `pub(crate) fn to_debian_arch(arch: &str) -> Result<&'static str, String>` pura (sin panic) con `x86_64→amd64`, `aarch64→arm64`, otro → `Err`; hacer que `architecture()` (línea 46) la envuelva; añadir bloque `#[cfg(test)]` con los 3 casos.
- [ ] T017 [US2] En `src/parser.rs`: añadir tests de B1 usando la fixture `src/fixtures/sources` de T012 (vía `include_str!`) — parseo válido → 2 repos con campos correctos; comentarios y claves desconocidas ignorados; campo ausente (`Components` vacío) → `Err`; versión CRLF (reemplazar `\n` por `\r\n` en runtime) → parsea igual; EOF estricto (ya cubierto por T003, verificar interacción con la fixture). Depends on T011 (A8) y T012 (fixture).
- [ ] T018 [US2] En `src/package_info.rs`: añadir tests de B4 al bloque existente — sin sección `SHA256` en el contenido → `Err`; entrada ausente (sección presente pero sin el path objetivo) → `Err`; hash con caracteres no-hex → `Err`. Depends on T010 (A2).

**Checkpoint**: All user stories should now be independently functional — US1 con bugs corregidos, US2 con cobertura de tests del Principio III.

---

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Gates de calidad y limpieza final

- [ ] T019 Ejecutar localmente los gates del CI (AGENTS.md §Comandos): `cargo fmt --all --check`, `cargo clippy --locked --all-targets --all-features -- -D warnings`, `cargo test --locked --all-features` (timeout ≥10 min la primera compilación); corregir cualquier warning nuevo.
- [ ] T020 Revisar que no quedan mensajes de error en inglés en los archivos tocados (`src/commands/update/mod.rs` tiene `println!`/`eprintln!` en inglés en las líneas 139-145, 165-169 — traducirlos al español según la convención de idioma).

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Empty — no blocking prerequisites
- **User Story 1 (Phase 3)**: Depends on Setup (T001) — this is the MVP
- **User Story 2 (Phase 4)**: Mostly independent; T017 depends on T011 (A8) and T012; T018 depends on T010 (A2)
- **Polish (Phase 5)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Setup (T001) — No dependencies on US2
- **User Story 2 (P2)**: Can start after Setup; T017 and T018 require US1 fixes (T011, T010) to be meaningful; T013-T016 are fully independent

### Within User Story 1

- Tests de regresión (T002, T003) ANTES de las implementaciones (TDD mínima exigida por constitución III para A2/A8)
- T004 (download.rs) y T005 (types/mod.rs) son paralelizables (archivos distintos)
- T006-T009 tocan `src/commands/update/mod.rs` → secuenciales entre sí
- T010 (package_info.rs) y T011 (parser.rs) son paralelizables entre sí y con T006-T009

### Within User Story 2

- T012-T016 paralelizables (archivos distintos; T016 refactoriza configuration.rs)
- T017 y T018 al final de la story (dependen de fixes de US1)

### Parallel Opportunities

- T004 [P] + T005 [P] (download.rs vs types/mod.rs)
- T010 [P] + T011 [P] (package_info.rs vs parser.rs, tras sus tests T002/T003)
- T012 [P] + T013 [P] + T014 [P] + T015 [P] + T016 [P] (5 archivos distintos en paralelo)

---

## Parallel Example: User Story 1

```bash
# Tras T002 y T003 (tests que deben fallar), lanzar en paralelo:
Task: "Añadir error_for_status en src/download.rs:12"
Task: "Reemplazar component().unwrap() por ok_or en src/types/mod.rs:86"

# Luego, en paralelo:
Task: "Implementar delimitación de sección SHA256 en src/package_info.rs (T010)"
Task: "Implementar EOF estricto en src/parser.rs (T011)"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Completar Phase 1: Setup (T001)
2. Completar Phase 3: User Story 1 (T002 → T011)
3. **STOP and VALIDATE**: `cargo test` en verde; ejecutar `nya update` con mirror caído y verificar exit ≠ 0
4. Commit por grupo lógico (ver plan: commits 1-4)

### Incremental Delivery

1. Setup → rama lista
2. User Story 1 → bugs corregidos, exit codes correctos (MVP)
3. User Story 2 → cobertura de tests del Principio III
4. Polish → gates en verde, mensajes en español

### Suggested Commit Mapping (del plan aprobado)

1. `fix(download): reject HTTP error statuses and add timeouts` (T004+T006)
2. `fix(update): log discarded InRelease errors, propagate Result, cleanup on mismatch` (T007+T008+T009+T005)
3. `fix(package_info): bound SHA256 section parsing (+tests)` (T002+T010)
4. `fix(parser): require EOF after last block (+tests)` (T003+T011)
5. `test(parser): cover valid/invalid sources parsing` (T012+T017)
6. `test(url): table-driven format_url cases` (T013)
7. `refactor(configuration): extract pure arch mapping (+tests)` (T016)
8. `test(integrity): hash and validation error cases` (T014)
9. `test(types): repository getters and validity` (T015)
10. `test(package_info): error paths` (T018)

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Cada user story es independiente y testeable por separado
- Constitución III: T002/T003 (tests de A2/A8) van en el MISMO commit que sus fixes (T010/T011)
- Sin dependencias nuevas (Principio V)
- No hacer push ni PR sin petición explícita del usuario (AGENTS.md §Flujo de trabajo con git)
