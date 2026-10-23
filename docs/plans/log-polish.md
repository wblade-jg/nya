# Plan: pulido de logs y captura de errores de `nya update`

Fecha: 2026-10-22
Rama: `fix/errors-and-tests` (tras los commits de la US1)
Estado: plan aprobado por el usuario (alcance: contexto + causas por mirror,
indicatif incluido), pendiente de implementar.

## Diagnóstico (por qué la salida actual es inútil)

Con el fallo real (falta `/etc/apt/keyrings/docker.asc`):

```
Descargado InRelease: /etc/nya/download.docker.com_linux_ubuntu_dists_noble_InRelease  ← stdout, path plano feo
Error en el pipeline: Error procesando InRelease: No such file or directory (os error 2)  ← ¿qué fichero?
Actualización fallida: 1 de 1 repositorios con error  ← resumen DUPLICADO
Error: "1 de 1 repositorios fallaron durante la actualización"  ← Debug de Rust con comillas
```

Problemas concretos:

1. **Mismo error impreso 3 veces** (pipeline, resumen, `main` via `Error: {:?}` de `fn main() -> Result`).
2. **Errores sin contexto**: `No such file or directory` no dice *qué* fichero (`integrity.rs` no envuelve `File::open`/`read_to_string`).
3. **Mirrors mudos**: `download_inrelease`/`download_packages_file` prueban mirrors en bucle y solo dicen "de ningún mirror" — la causa de cada intento (404, timeout) se descarta.
4. **Parser sin ruta**: `parser.rs` no incluye *qué* fichero `sources` falló ni línea.
5. **stdout/stderr sin criterio** y énfasis en paths internos que al usuario no le aportan.
6. **Sin feedback de descarga**: Packages de varios MB en streaming silencioso.
7. **Mezcla de idiomas** (pendiente T020): `"Packages file already exists..."`, `"No matching signature found"`, `"Parse error: ..."`.

## Diseño de salida

**Regla**: progreso/éxito → **stdout**; fallos → **stderr**; el resumen se imprime
**una sola vez** (desde `main`).

```
$ nya update
==> 1 repositorio en sources
==> download.docker.com/linux/ubuntu (noble)
  ✓ InRelease verificado
  ⠋ Packages  [██████████░░░░] 62% (2.6/4.2 MB)
  ✓ Packages verificado (SHA256)
==> Hecho: 1/1 correcto
```

```
$ nya update                          # fallo de clave (repro del usuario)
==> download.docker.com/linux/ubuntu (noble)
  ✗ No se pudo abrir la clave pública /etc/apt/keyrings/docker.asc: No existe el fichero (os error 2)
nya: 1 de 1 repositorios fallaron durante la actualización     ← exit 1
```

```
  ✗ InRelease no descargable de ningún mirror:                # causas por mirror
    - http://m1...: HTTP 404 Not Found
    - http://m2...: timeout de 30s agotado
```

**Decisiones**:

- **Sin crate `log`/`env_logger`** (YAGNI): macros `println!`/`eprintln!` con
  formato consistente bastan; un sistema de niveles no lo pide nadie.
- **`main` captura el `Err`** explícitamente (`eprintln!("nya: {e}")` + `exit(1)`)
  en vez del `Error: {:?}` por defecto → elimina la 3ª copia y las comillas.
- **Contexto en el origen**: cada capa (`integrity`, `parser`, mirrors) envuelve
  su error con qué fichero/URL/paso; las capas superiores solo propagan (sin
  re-formatear `"Error procesando InRelease: {heredado}"`).

## Fases

### Fase 0 — Commit de la US1 (acordado)

1. Commit los cambios pendientes de US1 en `fix/errors-and-tests`, en 3 commits
   sin partir archivos (`git add -p` evitable):
   - `fix(update): propagate errors, HTTP timeouts and cleanup on mismatch` —
     `download.rs`, `types/mod.rs`, `update/mod.rs`, `main.rs`
   - `fix(package_info): bound SHA256 section parsing (+tests)` — `package_info.rs`
   - `fix(parser): require EOF after last block (+tests)` — `parser.rs`

   > Desviación del mapeo de 4 commits del plan original: A3+A4+A1/A5/A6/A7
   > comparten `update/mod.rs`, separarlos exige `add -p`; 3 commits mantienen
   > la lógica sin trocear archivos.

2. Opcional: commit docs (`specs/`, `docs/plans/` están sin trackear) como
   `docs(spec): add 001-errors-and-tests`.
3. `git push -u origin fix/errors-and-tests` solo si lo pide el usuario.

### Fase 1 — Contexto de errores (qué y por qué falló)

- **`src/integrity.rs`**: envolver `File::open(key_path)`, `read_to_string(signed_path)`,
  `from_reader_single`, parse del cleartext y verificación → mensajes tipo
  `"No se pudo abrir la clave pública {path}: {e}"`, `"La firma no coincide con la
  clave {path}"` (también traduce `"No matching signature found"`).
- **`src/parser.rs`**: `read_repositories_from_file` incluye `filepath` y, si es
  posible con `nom`, snippet de línea/columna en el `Parse error` (mantener `InvalidData`).
- **`src/commands/update/mod.rs`**:
  - `download_inrelease` y `download_packages_file` acumulan `Vec<String>` de
    causas por mirror (`"{url}: {e}"`) y las devuelven multilínea en el `Err` final.
  - Etiqueta de repositorio `"{primera URI} ({suite})"` propagada en los errores
    de cada etapa, para que la línea `✗` sea identificable con pipelines concurrentes.
  - Quitar re-formateo redundante (`"Error procesando InRelease: {}"` etc.);
    propagar tal cual.

### Fase 2 — Salida unificada

- **`src/commands/update/mod.rs`**: imprime solo líneas de progreso/éxito
  (`✓`, cabeceras `==>`); los fallos por repositorio se imprimen una vez con `✗` +
  causa; **eliminar** el `eprintln!("Actualización fallida: ...")` (el `Err`
  devuelto ya lleva el resumen).
- **`src/main.rs`**: extraer `async fn run()`, capturar `Err` →
  `eprintln!("nya: {e}")` + `std::process::exit(1)`.
- Caso borde spec: `sources` con 0 repos → `==> Sin repositorios, nada que hacer`,
  exit 0.
- Fases 1 y 2 tocan los mismos archivos → pueden implementarse juntas, pero van
  en commits separados (contexto vs. formato de salida).

### Fase 3 — `indicatif` (barras de progreso)

- **`Cargo.toml`**: añadir `indicatif` + commitear `Cargo.lock` (el CI usa `--locked`).
- **Justificación Principio V** (va en el commit/plan): feedback de descargas
  multi-MB ya en streaming; el estándar del ecosistema; se auto-oculta en
  no-TTY (CI sin barras); alternativa manual (`\r`) reinventaría detección TTY
  y MultiProgress.
- **Diseño**: un `MultiProgress` compartido; **una barra por repositorio** cuyo
  mensaje evoluciona por etapa (`descargando InRelease` → `verificando firma` →
  `Packages %` con `content_length()`); `download_file` recibe `&ProgressBar`;
  `finish_and_clear()` + línea `✓` al terminar. Spinner para la verificación de
  firma (`spawn_blocking`). Barras creadas tras saber la URL/suite (etapa 1),
  así no hay filas huérfanas si un mirror falla.

### Fase 4 — Español restante (T020)

- `update/mod.rs`: `"Packages file already exists..."`, `"re-downloading"`,
  `"Hash mismatch..."`.
- `integrity.rs`/`parser.rs`: cubiertos en Fase 1.

### Fase 5 — Verificación (lo mismo que el CI + manual)

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
```

- Escenarios manuales con `CACHE_DIR`/`REPOSITORIES_FILE` en `/tmp/opencode/nya-test/`:
  1. clave ausente (repro) → mensaje nombra el fichero, exit 1, **una** línea de resumen;
  2. mirror caído → lista de causas por mirror, exit 1;
  3. `sources` malformado → error con ruta del fichero;
  4. 0 repositorios → exit 0;
  5. red real (si hay clave Docker) → barras visibles en TTY.

## Notas

- Tests unitarios de esta fase: casi todos los caminos son de red/archivo →
  verificación manual (el mock HTTP es Fase D, fuera de alcance). Se mantienen
  verdes los 9 tests existentes.
- Trasmergear eventualmente, marcar T019/T020 de
  `specs/001-errors-and-tests/tasks.md`; las causas por mirror eran "Fase C" en
  el plan original → este trabajo las adelanta.
