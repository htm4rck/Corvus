# Corvus

Backend Rust y UI Angular básica para la plataforma de datos analíticos de Tordo. Reimplementación iniciada por decisión del propietario el 2026-10-30. Los modelos, puertos y casos de uso viven en Tordo.SDK/rust/corvus-core siguiendo la arquitectura Business/SDK.Business.

La primera fase entrega eventos durables con replay compatible, lotes con validación estructural real, publicación transaccional de snapshots, alcance por empresa y una UI de operación inicial. Añade el piloto comercial Acme con curado de comprobantes/pedidos y serving de tres consultas registradas. Ingesta incremental, catálogo universal, linaje y workers siguen pendientes.

El backend Quarkus previo y su README se conservan íntegros en legacy/. AnpyeReports permanece en su repositorio; Tordo.Reports es el nuevo producto complementario de BI.

## Ejecutar y verificar

Requiere Tordo.SDK y Tordo.Reports como checkouts hermanos, Rust 1.85.1, Node 24, Docker y la red DEV de Tordo.Platform. Los lockfiles están versionados.

```sh
cargo build --locked
cargo clippy --locked --all-targets -- -D warnings
cd apps/ui
npm ci
npm run build -- --base-href /corvus/
npm test -- --watch=false
cd ../..
python3 scripts/bootstrap_dev.py
python3 scripts/bootstrap_mysql_dev.py
python3 scripts/apply_dev_routes.py
python3 scripts/verify_dev.py
```

También se debe compilar Tordo.Reports y su UI antes del bootstrap compartido. Los scripts sólo provisionan las nuevas bases/servicios analytics DEV. El token para las operaciones del fixture está en .runtime/stack.json, archivo privado excluido de Git.

- UI DEV: https://dev.tordo.io/corvus/
- UI local mediante gateway: http://127.0.0.1:8000/corvus/
- Salud: /corvus/health mediante gateway, o http://127.0.0.1:8086/health

Ver [arquitectura, contratos, continuidad y límites](docs/architecture.md). No hay migración de datos históricos ni despliegue de producción en esta primera fase.

## Piloto Acme disponible en DEV

[Acme: ingesta Birdport/Amazilia y tres reportes](docs/acme-sales-ingestion-design.md), con reconocimiento de datos DEV y requisitos de homologación a la arquitectura Business. Estado: ejecutado y verificado. Ver [operación y aceptación](docs/acme-pilot-runbook.md).


## Ingesta por dominio, sin componentes por empresa

El producto organiza definiciones reutilizables de ingesta por dominio y scope de empresa. `sales_documents` publica bronce, plata y oro en esquemas separados. Acme es configuración/datos del ejemplo, no un componente. El modelo vigente está en `Tordo.SDK/rust/docs/analytics-product-model.md`; ver [operación por dominio](docs/runbook.md).

## Publicación

Ver [CI e imágenes para DEV](docs/RELEASE_DEV.md). La publicación utiliza la distribución Rust del SDK fijada a un commit; no depende de cambios locales sin publicar.
