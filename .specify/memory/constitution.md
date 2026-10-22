<!--
Sync Impact Report
- Version change: 1.0.0 → 2.0.0 (MAJOR: corrección del alcance de V, redacción
  de II, jerarquía entre principios)
- Modified principles: I (razón sin sesgo post-hoc), II (frase rota corregida +
  limitación TOFU declarada), III (estado real y plan de tests de parser), V
  (alcance corregido: suficiente para `update`, no para el roadmap completo)
- Added sections: Jerarquía y Excepciones (desempate y waivers)
- Removed sections: constantes de implementación de Restricciones del Código
  (límite de concurrencia, mapa de módulos, formatos de commit → AGENTS.md)
- Follow-ups: tests unitarios de `parser.rs` (requisito de cierre para
  `install`); fijado de fingerprint de clave (Pendiente en AGENTS.md)
-->

# nya Constitution

## Core Principles

### I. Biblioteca primero, CLI delgada
Todo el comportamiento MUST residir en el crate biblioteca (`src/lib.rs` y sus
módulos). `src/main.rs` SOLO parsea argumentos (clap) y despacha a
`nya::commands`. Cada capacidad nueva MUST exponerse como función o módulo de
biblioteca testeable sin pasar por la CLI.
Razón: mantener el comportamiento testeable sin la CLI y acotar la superficie
pública del crate.

### II. Integridad verificada (NO NEGOCIABLE)
Ningún artefacto de red (InRelease, Packages, .deb) MUST usarse sin verificar:
firma PGP del InRelease (clave principal o subclave) y hash SHA256 de
`Packages` contrastado con la entrada del InRelease. Ningún artefacto no
verificado MUST aceptarse, ni siquiera como fallback o "de todos modos"; ante
firma o hash que no coincide, el artefacto MUST descartarse (reintentar en otro
mirror o fallar). Toda verificación criptográfica MUST vivir en `integrity.rs`.
Razón: semántica APT — confianza cero en la red.
Limitación conocida: la verificación se realiza contra la clave indicada por
`Signed-By` (modelo TOFU, sin fijado de fingerprint); el fingerprint pinning
está en el roadmap (ver Pendiente en AGENTS.md).

### III. Tests con fixtures
El parsing, la extracción de hashes y la verificación de firmas MUST tener
tests unitarios en línea (`#[cfg(test)]`) junto al código. Las fixtures viven
en `src/fixtures/` y se incluyen con `include_str!`; los tests MUST ser
autocontenidos (sin red). La verificación de firmas MUST cubrir al menos un
caso positivo y un caso negativo (mensaje adulterado rechazado).
Estado al ratificar 2.0.0: firmas (`integrity.rs`) y extracción
(`package_info.rs`) cubiertas; el parser (`parser.rs`) está pendiente y su
cobertura es requisito de cierre para `install` (ver Pendiente en AGENTS.md).
Razón: un `sources` malformado no debe pasar inadvertido; las fixtures hacen
los tests deterministas y sin red.

### IV. Errores sobre panics
Las operaciones de red y archivo MUST devolver `Result` y degradar (reportar el
error y continuar con el siguiente mirror) en lugar de abortar el proceso.
`panic!`/`expect` SOLO se permiten en fallos de configuración irrecoverables
(archivo de fuentes ausente, arquitectura no soportada). El código que cruza
`spawn_blocking` MUST propagar errores `Send + Sync`.
Razón: el pipeline de `update` está diseñado para tolerar fallos parciales.

### V. Simplicidad deliberada (YAGNI)
Las dependencias nuevas SOLO se añaden con justificación explícita (qué
problema resuelve y por qué el stack actual no basta). El stack actual es
suficiente para el alcance actual (`update`); `install` presumiblemente
requerirá dependencias de descompresión (ar, xz, zstd o gzip), que se
aprobarán bajo este mismo criterio. `#[allow(dead_code)]` es aceptable para
tipos reservados al roadmap (p. ej. camino a `install`); el código muerto sin
plan MUST eliminarse. A efectos de esta constitución, "plan" significa la
lista Pendiente de AGENTS.md o una issue registrada. Funcionalidades fuera
del alcance actual (multi-suite completo, arquitecturas extra) MUST
posponerse hasta que una tarea concreta lo pida.
Razón: proyecto de aprendizaje con minimalismo intencionado.

## Restricciones del Código

- **Idioma**: mensajes al usuario (`println!`, `eprintln!`, `panic!`, textos de
  error) en español; identificadores, módulos y mensajes de commit en inglés.
- **Visibilidad**: en la biblioteca solo `commands` y `configuration` son
  `pub`; el resto `pub(crate)`. El binario es la única interfaz para el usuario.
- **Concurrencia**: descargas con concurrencia acotada; hashing y PGP fuera
  del runtime con `spawn_blocking`. El límite operativo vive en AGENTS.md.
- **Separación de módulos**: parsing, verificación e orquestación en módulos
  distintos (mapa actual en AGENTS.md §Estructura). No mezclar
  responsabilidades.
- **Commits**: Conventional Commits en inglés (formatos en AGENTS.md).

## Flujo de Desarrollo

- Antes de dar por terminada una tarea: `cargo fmt`, `cargo clippy` sin
  warnings, y `cargo test` en verde. Estos gates se automatizan en CI
  (`.github/workflows/ci.yml`, disparado en pushes a `feature/**`, `fix/**`,
  `docs/**` y en PRs a `main`) y también MUST ejecutarse localmente; un CI en
  rojo bloquea el merge.
- Todo cambio de comportamiento de parsing o verificación MUST incluir sus
  tests en el mismo commit.
- La guía operativa no vinculante vive en `AGENTS.md`; el `README.md` es
  material introductorio. En conflicto con esta constitución, prevalece la
  constitución.
- El cierre de toda tarea MUST incluir una auto-revisión de cumplimiento de
  los principios I–V.

## Jerarquía y Excepciones

- Si dos principios prescriben acciones incompatibles, la prioridad es
  II > IV > III > I > V: integridad y robustez por encima de arquitectura y
  minimalismo. Ejemplo: si verificar mejor exige una dependencia nueva,
  prevalece II sobre V.
- Excepciones (waivers): solo con aprobación explícita del responsable del
  proyecto, documentadas en el commit que las introduce (motivo y alcance).
  Caducan al cerrarse la tarea que las motivó.

## Governance

- Esta constitución prevalece sobre prácticas ad-hoc y sobre instrucciones de
  agentes o herramientas que entren en conflicto con ella.
- Las enmiendas MUST editar este archivo, incrementar la versión según semver
  (MAJOR: eliminación o redefinición de principios; MINOR: principio o sección
  nueva; PATCH: clarificaciones) y actualizar la fecha de última enmienda.
- Toda enmienda MUST documentarse en el commit con el motivo del cambio.

**Version**: 2.0.0 | **Ratified**: 2026-10-21 | **Last Amended**: 2026-10-21
