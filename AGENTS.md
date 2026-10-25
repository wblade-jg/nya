# AGENTS.md

## Proyecto

**nya** ("Not Yet APT") — gestor de paquetes minimalista tipo APT, proyecto de
aprendizaje en Rust (edición 2024). Objetivo: reproducir los internals de APT:
parsing de repositorios Debian (deb822), verificación criptográfica de InRelease
y descarga verificada de `Packages`.

- Binario `nya` (`src/main.rs`) + biblioteca (`src/lib.rs`).
- CLI (clap): `nya update` (implementado), `nya install <paquete>` (stub).
- Runtime tokio multi-thread; errores con `NyaError` (`thiserror`, módulo
  `src/error.rs`); `Box<dyn Error>` solo en `main.rs`.
- Gobernanza: `.specify/memory/constitution.md` (principios I–V). En conflicto
  con esta guía, prevalece la constitución. Jerarquía entre principios y
  waivers: ver constitución §Jerarquía.

## Comandos

- `cargo build` — compilar (dev)
- `cargo test` — tests unitarios (fixtures en `src/fixtures/`, vía `include_str!`)
- `cargo clippy` — lints
- `cargo fmt` — formato
- `cargo run -- update` — ejecutar la actualización de índices

Antes de cerrar una tarea, ejecutar localmente lo mismo que el CI:

- `cargo fmt --all --check`
- `cargo clippy --locked --all-targets --all-features -- -D warnings`
- `cargo test --locked --all-features`

El CI (`.github/workflows/ci.yml`) corre en pushes a `feature/**`, `fix/**` y
`docs/**`, y en PRs a `main`. Un CI en rojo bloquea el merge.

Variables de entorno: `CACHE_DIR` (default `/etc/nya`), `REPOSITORIES_FILE`
(default `sources`).

## Flujo de trabajo con git (obligatorio)

**Antes de cualquier cambio o feature, usar la skill `repo-workflow`**
(herramienta `skill`, nombre: `repo-workflow`). Resumen mínimo:

- Nunca trabajar directo en `main`: verificar el estado del repo y crear rama
  `feature/<nombre>`, `fix/<nombre>` o `docs/<nombre>` según el alcance (los
  prefijos coinciden con los triggers del CI).
- NO commitear a menos que el usuario lo pida explícitamente.
- Si el usuario acepta: commit Conventional Commit, `git push -u origin <rama>`
  y PR a `main` con `gh pr create`.
- Tras merge del PR: `git checkout main` → `git pull` → `git branch -d <rama>`.

## Estructura

| Ruta | Rol |
|---|---|
| `src/main.rs` | CLI con clap: subcomandos `update` / `install` |
| `src/lib.rs` | públicos: `commands`, `configuration`; resto `pub(crate)` |
| `src/commands/update/mod.rs` | orquestación del pipeline de `nya update` |
| `src/parser.rs` | parser nom de archivos `sources` (deb822) |
| `src/integrity.rs` | SHA256 + verificación de firmas PGP (`pgp`) |
| `src/package_info.rs` | extrae hash/tamaño/ruta de `Packages` desde InRelease |
| `src/download.rs` | descarga streaming con reqwest |
| `src/url.rs` | URL → nombre de archivo plano |
| `src/error.rs` | `NyaError` (thiserror): errores tipados del pipeline |
| `src/types/` | `Repository`, `HashType`, `PackageFileInfo`, `DownloadedInRelease` |
| `src/configuration.rs` | config por env vars; mapeo de arquitectura del host |
| `src/fixtures/` | InRelease y clave pública de prueba |
| `docs/` | assets de documentación (logo) |
| `sources` | archivo de repositorios (Kali, GitHub CLI, ChatGPT, Docker) |
| `.github/workflows/ci.yml` | CI: fmt, clippy, test |
| `.opencode/skills/repo-workflow/` | skill de flujo git/PR |

## Pipeline de `update`

1. Leer `sources` → `Vec<Repository>` (parser nom).
2. Descargar `dists/<suite>/InRelease` probando mirrors en orden
   (`buffer_unordered(10)`; un fallo de repositorio no aborta el resto).
3. En `spawn_blocking`: verificar firma PGP (clave principal o subclave) y
   extraer la entrada SHA256 de `<component>/binary-<arch>/Packages`.
4. Descargar `Packages` contra todos los mirrors: si ya existe y el hash
   coincide se omite; si no coincide, se vuelve a descargar.
5. Los errores se reportan con `eprintln!` y el pipeline continúa.

## Convenciones

- **Idioma**: el código va en inglés — identificadores (variables, funciones,
  tipos, módulos), comentarios y commits. Solo van en español los mensajes al
  usuario (`println!`, `eprintln!`, `panic!`), los textos de error
  (`NyaError`) y los logs; el resto, en inglés.
- **Commits**: Conventional Commits en inglés: `feat(scope):`, `refactor:`,
  `fix:`, `chore:`, `docs:` (scope opcional, p. ej. `feat(update):`).
- **Visibilidad**: solo `commands` y `configuration` son `pub`; resto
  `pub(crate)`.
- **Tests**: unitarios en línea (`#[cfg(test)]`) junto al código; fixtures en
  `src/fixtures/` incluidas con `include_str!` (no usar `tests/`).
- **`#[allow(dead_code)]`** en tipos reservados al roadmap (`HashType::MD5`,
  tamaños de hash): no eliminarlos, se usarán al implementar `install`.
- El parser ignora claves desconocidas (`Types`, `Architectures`) y comentarios
  `#`; `Repository::new` exige `URIs`, `Suites`, `Components` y `Signed-By`
  no vacíos.

## Pendiente (contexto para nuevas tareas)

- `nya install`: parsear `Packages`, resolver dependencias,
  descargar/descomprimir e instalar `.deb`.
- Multi-suite/multi-componente (hoy solo usa el primero: `suites.first()`).
- Arquitecturas más allá de amd64/arm64 (hoy `NyaError::UnsupportedArch`).
- `Release` firmado (no cleartext) como fallback; validación de confianza de
  la clave (hoy TOFU: se verifica la clave indicada en `Signed-By` sin fijar
  fingerprint).
- Keyrings con varias claves: `integrity::validate_signature_file` solo lee
  la primera (`SignedPublicKey::from_reader_single`); ante rotación de
  claves falla con `SignatureMismatch`. Caso real: GitHub CLI (`sources`),
  `nya update` → 3/4 correctos.

## Reglas para agentes

- Usar la skill `repo-workflow` antes de tocar el repo (ver sección Flujo de
  trabajo con git).
- No romper la API `pub(crate)` sin motivo; la CLI es la única superficie
  pública.
- Mantener la separación: parsing → `parser.rs`, verificación →
  `integrity.rs`, orquestación → `commands/`.
- No añadir dependencias sin comentarlo: el proyecto es deliberadamente
  mínimo.
- Toda función de red o archivo nuevo debe degradar con `Result`, no con
  `panic!`.
- Ante dudas de gobernanza, consultar `.specify/memory/constitution.md`.
