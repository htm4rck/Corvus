# Publicación y DEV

La rama `develop` ejecuta pruebas de UI, build Angular, Cargo y Clippy antes de publicar `ghcr.io/htm4rck/corvus:<commit>` y activar únicamente `corvus-rust-dev` mediante Tordo.Platform.

La dependencia Rust corresponde a la [distribución privada SDK 0.1.0](https://github.com/tordo-io/tordo-sdk/releases/tag/rust-v0.1.0), fijada al commit `8cd3fe09ac9f70b6f93f4a481f1f5351b35c7adc`. La distribución es código fuente verificable; no es un registro Cargo.

La CI requiere `SDK_READ_TOKEN` con lectura del repositorio SDK y `PLATFORM_DEPLOY_TOKEN` para solicitar el despliegue. No se copian credenciales al contexto Docker. El contexto de construcción es el directorio padre con `Corvus` y `Tordo.SDK` como hermanos; `Dockerfile.dockerignore` excluye runtime privado, artefactos y dependencias locales.

Tordo.Platform conserva variables, red, aislamiento y límites del servicio DEV existente, y sustituye los montajes de binario/UI locales por los contenidos de la imagen. Comprueba `/health` antes de continuar y restaura el contenedor anterior si falla. El despliegue no reprovisiona bases ni ejecuta los scripts del fixture. Los cambios de esquema futuros requieren una migración explícita antes de activar una versión que los necesite.

La validación de UI y dominio no equivale a una homologación completa del producto ni a una prueba de aceptación con datos reales. Producción no forma parte de este flujo.
