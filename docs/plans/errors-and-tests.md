# Plan: corrección de errores y cobertura de tests

Fecha: 2026-10-22
Rama prevista: `fix/errors-and-tests` (desde `main`, en `b2f17fa`)
Estado: plan aprobado, pendiente de implementar.

## Contexto

La revisión completa del código (fases 1–4) detectó ~20 hallazgos y listó 26
aspectos que deberían testearse; solo 3 están cubiertos (firma positiva,
firma adulterada, extracción parcial de `Packages`). La constitución 2.0.0
(Principio III) registra los tests de `parser.rs` como pendientes y requisito
de cierre para `install`.

El PR #1 ya está mergeado en `main`: la constitución v2.0.0, `AGENTS.md`, el
CI y el scaffold de Spec Kit están trackeados; no queda gobernanza pendiente
de commitear. Los números de línea de este plan corresponden al HEAD de
`main` (`b2f17fa`, tras `cargo fmt` + lints de clippy).

Decisiones acordadas:

- Alcance de esta tanda: **Fase A (errores P0) + Fase B (tests P1)**.
  Las fases C (robustez P2) y D (mocks de red) quedan para más adelante.
- Código de salida de `update`: **estricto** — si cualquier repositorio
  falla, el proceso sale con código ≠ 0 (resumen final de fallos).
- Git: crear `fix/errors-and-tests` desde `main` y trabajar allí. No
  push/PR salvo petición explícita del usuario.

## Fase A — Correcciones de errores (P0)

| # | Cambio | Ubicación |
|---|---|---|
| A1 | Loguear los errores de descarga de `InRelease` que hoy se descartan con `result.ok()` (mismo patrón que el `eprintln!` de las líneas 43–45) | `src/commands/update/mod.rs:18` |
| A2 | **Delimitar la sección SHA256**: cortar el análisis en el siguiente campo nuevo (línea no indentada no vacía), saltar entradas con tamaño no numérico en vez de abortar con `?`, validar `hash.len() == 64` y caracteres hex, igualdad exacta de `path` (quitar `ends_with`, falso positivo `notmain/…`) | `src/package_info.rs:9-45` |
| A3 | `error_for_status()` en la petición de descarga (un 404/500 no puede tratarse como éxito) | `src/download.rs:12` |
| A4 | Cliente HTTP con timeouts: `Client::builder().timeout(30s).connect_timeout(10s).build()?` en vez de `Client::new()` | `src/commands/update/mod.rs:10` |
| A5 | `update() -> Result<(), Box<dyn Error>>` y `?` en `main` para obtener código de salida ≠ 0. `execute_pipeline` cuenta fallos; si algún repositorio falla: resumen final con `eprintln!` y `Err`. El error de lectura/parseo de `sources` también propaga | `src/commands/update/mod.rs:53`, `src/main.rs:31` |
| A6 | Borrar el archivo corrupto tras un hash mismatch antes del `continue` (hoy queda en disco) | `src/commands/update/mod.rs:164-170` |
| A7 | `component().unwrap()` → `ok_or(...)?` (eliminar panic latente si un `Repository` a mano tiene `components` vacío) | `src/types/mod.rs:86` |
| A8 | **EOF estricto en el parser**: `read_repositories_from_file` hoy descarta el resto del parseo, de modo que un bloque malformado al final se pierde en silencio. Exigir resto vacío tras consumir comentarios y espacios finales (para no romper ficheros que terminan en `#`) | `src/parser.rs:113` |

Nota de gobernanza (constitución III): los tests de regresión de **A2** y
**A8** deben ir en el mismo commit que su cambio de comportamiento.

Tests de regresión de A2 (en `package_info.rs`):

- entrada presente solo tras un campo posterior (p. ej. `MD5Sum`) → `Err`;
- línea con tamaño no numérico tras la cabecera `SHA256:` → se ignora, no
  aborta;
- path `notmain/binary-amd64/Packages` no debe coincidir con
  `main/binary-amd64/Packages`;
- hash de longitud/hex inválidos → `Err`.

Tests de regresión de A8 (en `parser.rs`):

- línea malformada tras el último bloque → `Err` (antes: silencio);
- fichero que termina en comentario `#` → `Ok` (comportamiento preservado).

## Fase B — Tests (P1, cumple Principio III)

| # | Módulo | Casos |
|---|---|---|
| B1 | `parser.rs` + fixture nueva `src/fixtures/sources` (vía `include_str!`) | parseo válido de 2 repos con multi-mirror; comentarios `#` y claves desconocidas (`Types`, `Architectures`) ignorados; campos ausentes (`URIs`/`Suites`/`Components`) → `Err`; CRLF; EOF estricto (A8) |
| B2 | `url.rs` | tabla de casos puros: URL normal → nombre plano; sin host (`file:///x`) → `None`; trailing slash; colisión `/a/b` vs `/a_b` (documentar limitación) |
| B3 | `configuration.rs` | extraer `to_debian_arch(&str) -> Result<&'static str, …>` pura (sin panic) y testear `x86_64→amd64`, `aarch64→arm64`, desconocida → `Err`; `architecture()` pasa a envolverla |
| B4 | `package_info.rs` | sin sección `SHA256` → `Err`; entrada ausente → `Err`; hash no-hex → `Err` |
| B5 | `integrity.rs` | `calculate_file_hash` sobre `src/fixtures/InRelease` con valor esperado hardcodeado (obtenido con `sha256sum` al implementar); fichero inexistente → `Err`; `validate_signature_file` con rutas inexistentes → `Err`; clave válida + texto sin firmar → `Err` |
| B6 | `types/repository.rs` | `is_valid` (vacío, parcial, completo); `suite()`/`component()` devuelven el primero; `target_package_path` |

Follow-ups explícitamente fuera de alcance (anotar en Pendiente si se
confirman):

- e2e de `process_inrelease`: requiere re-firmar la fixture `InRelease` con
  una clave privada de test (hasta ahora solo está la pública);
- segunda clave PGP en fixtures para cubrir "firma válida pero de otro
  remitente" → `Err`.

## Commits propuestos (código)

Se presentarán al usuario antes de ejecutarse:

1. `fix(download): reject HTTP error statuses and add timeouts` (A3+A4)
2. `fix(update): log discarded InRelease errors, propagate Result, cleanup on mismatch` (A1+A5+A6+A7)
3. `fix(package_info): bound SHA256 section parsing (+tests)` (A2 + regresiones)
4. `fix(parser): require EOF after last block (+tests)` (A8 + regresiones)
5. `test(parser): cover valid/invalid sources parsing` (B1)
6. `test(url): table-driven format_url cases` (B2)
7. `refactor(configuration): extract pure arch mapping (+tests)` (B3)
8. `test(package_info): error paths` (B4, si no quedó en el commit 3)
9. `test(integrity): hash and validation error cases` (B5)
10. `test(types): repository getters and validity` (B6)

## Verificación final

Ejecutar localmente lo mismo que el CI, con timeout alto (la primera
compilación supera los 2 minutos):

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
```

Sin `git push` ni `gh pr create` salvo petición explícita del usuario.

## Fuera de alcance (recordatorio)

- Fase C — robustez P2: descarga atómica `.tmp`+rename, CLI parseada antes
  de `Configuration` (`--help` sin `sources`), log de la causa por mirror
  fallido, validación única de arquitectura al inicio de `update()`,
  mensajes en inglés → español.
- Fase D — red: dev-dep de mock HTTP (`wiremock`/`httpmock`) para
  `download_file` y el skip/failover de mirrors; requiere justificación
  explícita bajo Principio V.
