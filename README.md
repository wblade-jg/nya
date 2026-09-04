<div align="center">
  <img src=".github/nya-logo.png" alt="nya logo" width="300"/>
  
  # nya
  
  **Not Yet APT** - Un mini gestor de paquetes tipo APT escrito en Rust
  
  ![Rust](https://img.shields.io/badge/rust-2024-orange.svg)
  ![Status](https://img.shields.io/badge/status-learning%20project-yellow.svg)
  ![License](https://img.shields.io/badge/license-TBD-lightgrey.svg)
  
</div>

---

## 🚀 Acerca de nya

**nya** (pronunciado "nia") es un proyecto de aprendizaje que implementa un gestor de paquetes minimalista tipo APT, escrito en Rust. El nombre significa **"Not Yet APT"**, reconociendo humildemente que es una versión simplificada de APT enfocada en entender sus mecanismos internos.

Este proyecto existe como una exploración práctica de los internals de sistemas de gestión de paquetes Debian, implementando desde cero el parsing de repositorios, validación criptográfica, y gestión de descargas.

## ✨ Características

### ✅ Implementado

- **Comando `update`** - Actualiza la lista de paquetes disponibles
- **Validación de firmas GPG** - Verificación criptográfica de archivos InRelease
- **Hash validation con SHA256** - Validación de integridad de archivos descargados
- **Skip inteligente de descargas** - Evita descargas redundantes verificando hashes locales
- **Soporte multi-mirror** - Fallback automático entre múltiples espejos
- **Pipeline asíncrono** - Descarga y procesamiento concurrente con Tokio
- **Parser de repositorios APT** - Parsing de archivos sources.list y metadata de repositorios

### 🚧 En Desarrollo

- [ ] Comando `install` - Instalación de paquetes con resolución básica de dependencias

## 🛠️ Uso

> **Nota:** nya está en desarrollo activo y no está listo para uso en producción. Los comandos pueden cambiar sin previo aviso.

### Comandos actuales

```bash
# Actualizar lista de repositorios
nya update

# Instalar un paquete (en desarrollo)
nya install <nombre_paquete>
```

### Configuración

nya lee la configuración de repositorios desde un único archivo siguiendo el formato DEB822. Por defecto busca el archivo `sources` en el directorio actual, pero puedes especificar otra ruta usando la variable de entorno `REPOSITORIES_FILE`. Los archivos descargados se almacenan en `/etc/nya` (configurable con `CACHE_DIR`).

## 🏗️ Arquitectura

El proyecto está estructurado en módulos principales:

- **`commands/`** - Implementación de comandos CLI (update, install)
- **`types/`** - Tipos de datos para repositorios y metadata
- **`integrity.rs`** - Validación de firmas GPG con pgp
- **`download.rs`** - Sistema de descarga asíncrona
- **`parser.rs`** - Parser de archivos APT usando nom
- **`configuration.rs`** - Gestión de configuración del sistema

## 🧪 Estado del Desarrollo

**Última actualización:** Septiembre 2024

El proyecto actualmente se encuentra en fase de implementación de funcionalidades core:

- ✅ Pipeline de actualización de repositorios funcional
- ✅ Validación criptográfica completa
- ✅ Optimizaciones de ownership y manejo eficiente de memoria
- 🚧 Sistema de instalación en desarrollo
- 🚧 Resolución de dependencias en diseño

## 🤝 Contribución

¡Las contribuciones son bienvenidas! Si estás interesado en contribuir a nya:

1. **Revisa el código** - Familiarízate con la arquitectura actual
2. **Abre un issue** - Discute nuevas características o reporta bugs
3. **Crea un PR** - Contribuye con código, documentación o tests

## 📝 Licencia

El proyecto aún no tiene una licencia definida. Se definirá próximamente.

## 🙏 Agradecimientos

Este proyecto existe como una herramienta de aprendizaje y exploración de:
- Los internals del sistema APT de Debian
- Programación de sistemas en Rust
- Criptografía aplicada (GPG, SHA256)
- Diseño de herramientas CLI modernas
