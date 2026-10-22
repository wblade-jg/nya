<div align="center">
  <img src="docs/nya-logo.png" alt="nya logo" width="300"/>

  # nya

  **Not Yet APT** — Un mini gestor de paquetes tipo APT escrito en Rust

  [![CI](https://github.com/wblade-jg/nya/actions/workflows/ci.yml/badge.svg)](https://github.com/wblade-jg/nya/actions/workflows/ci.yml)
  ![Rust](https://img.shields.io/badge/rust-2024-orange.svg)
  ![Status](https://img.shields.io/badge/status-learning%20project-yellow.svg)
  [![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
</div>

**nya** (pronunciado "nia") es un proyecto de aprendizaje que implementa un gestor de paquetes minimalista tipo APT, escrito en Rust. El nombre significa **"Not Yet APT"**, reconociendo humildemente que es una versión simplificada de APT enfocada en entender sus mecanismos internos.

Este proyecto existe como una exploración práctica de los internals de sistemas de gestión de paquetes Debian: parsing de repositorios, validación criptográfica de índices y descarga verificada de paquetes.

## Características

- Parser deb822 para archivos `sources`, escrito con [nom](https://crates.io/crates/nom)
- Descarga de `InRelease` con reintento automático sobre múltiples mirrors
- Verificación de firmas PGP de `InRelease` con [pgp](https://crates.io/crates/pgp), contra la clave principal o sus subclaves
- Extracción de la entrada SHA256 de `Packages` desde el `Release` firmado
- Descarga verificada de `Packages` con caché local: se omite si el hash ya coincide
- Pipeline concurrente sobre [tokio](https://crates.io/crates/tokio) (runtime multi-thread)

## Compilación

```bash
cargo build --release
```

> [!NOTE]
> Por defecto, `nya` guarda sus índices en `/etc/nya`, que requiere permisos de escritura. Ejecuta con `sudo` o apunta `CACHE_DIR` a un directorio de tu usuario antes de continuar.

## Uso

```bash
# Actualizar los índices de todos los repositorios configurados
nya update

# Información de un paquete (en desarrollo)
nya install <paquete>
```

También puedes ejecutarlo directamente con Cargo:

```bash
cargo run -- update
```

Los repositorios se definen en un archivo con formato deb822 (`sources` por defecto):

```text
Types: deb
URIs: https://download.docker.com/linux/ubuntu
Suites: noble
Components: stable
Architectures: amd64
Signed-By: /etc/apt/keyrings/docker.asc
```

> [!NOTE]
> `nya install` es un stub: por ahora solo imprime el nombre del paquete. El parser de `Packages`, la resolución de dependencias y la instalación de `.deb` están en el roadmap.

## Configuración

| Variable | Por defecto | Descripción |
|---|---|---|
| `REPOSITORIES_FILE` | `sources` | Ruta al archivo de repositorios |
| `CACHE_DIR` | `/etc/nya` | Directorio donde se guardan los índices descargados |

Arquitecturas soportadas: `amd64` y `arm64` (se detectan automáticamente desde el host).

## Desarrollo

```bash
cargo test                       # tests unitarios (fixtures en src/fixtures/)
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

El CI de GitHub Actions ejecuta exactamente estos mismos gates en cada PR hacia `main`; un CI en rojo bloquea el merge.
