# Operar ingestas por dominio y empresa

Fecha: 2026-10-30. Estado: DEV, snapshots manuales. Modelo canónico: `Tordo.SDK/rust/docs/analytics-product-model.md`.

## Definiciones y capas

Cargar el espacio autorizado en https://dev.tordo.io/corvus/ o http://127.0.0.1:8000/corvus/. En Ingestas se crea una definición con nombre, dominio y origen autorizado. El perfil disponible es `sales_documents`, comprobantes de venta con contexto de pedidos. No hay empresa por defecto ni componentes de cliente.

La empresa se obtiene del scope autenticado. Otra empresa usa el mismo dominio y puede crear varios pipelines. Los orígenes son referencias aprobadas del servidor y sólo se listan los de la empresa seleccionada.

Seleccionar una definición, fechas e identidad de ejecución. Ejecutar ingesta toma un corte consistente del origen. Bronce conserva el contrato exportado antes de normalizar/validar; plata contiene los hechos homologados; oro contiene sus modelos materializados. El historial distingue bronce capturado, error de calidad y snapshot publicado. Un fallo no reemplaza la última plata/oro correcta.

Repetir `(empresa,pipeline,runKey)` y ventana devuelve el snapshot publicado, o reintenta un run fallido con otro intento. Cambiar la ventana bajo esa identidad falla. Nueva identidad toma otro corte. No se acumulan importes de diferentes snapshots. Los vigentes y checkpoints son por pipeline.

## Endpoints

| Método | Ruta | Propósito |
|---|---|---|
| GET | /api/v1/ingestion-domains | Perfiles de dominio disponibles |
| GET | /api/v1/ingestion-sources | Referencias de origen autorizadas del espacio |
| GET/POST | /api/v1/pipelines | Listar/crear definiciones |
| POST | /api/v1/pipelines/{id}/ingestions | Ejecutar con runKey/from/to |
| GET | /api/v1/pipelines/{id}/runs | Estado y captura bronce por intento |
| GET | /api/v1/pipelines/{id}/dataset | Manifiesto vigente y capas |
| GET | /api/v1/models | Modelos oro y esquema/filtros disponibles |
| POST | /api/v1/sales/query | Consultar modelo autorizado con pipelineId/snapshotId explícitos |

Las rutas anteriores de eventos/lotes se mantienen. Las rutas comerciales antiguas sin pipeline ID son de compatibilidad y requieren que la selección sea inequívoca; una conexión Reports nueva debe ligar un pipeline explícito.

## Origen y contratos

`CORVUS_ANALYTICS_SOURCES` es un registro privado de referencias con `tenantId`, `url` de lector y `deploymentEnvironment`. El lector `PostgresSalesExportReader` consume el contrato `analytics_export` de cuatro vistas: cabeceras/líneas de comprobante y cabeceras/líneas de pedido. Si un origen no tiene pedidos, entrega esas vistas vacías; no se inventan pedidos POS.

`source/operation_sales_export.sql` es el adaptador de exportación para una base Operation por empresa. No contiene un código de empresa. Excluye datos personales y usa evidencia de origen para clasificar negocio/prueba/desconocido. El perfil actual de lectura está habilitado sólo en DEV. Integrar otro esquema/ERP requiere otro adaptador que entregue el contrato, no un componente por cliente.

La primera carga no es CDC: máximo5000 comprobantes/pedidos,50000 líneas de cada clase y366 días. La lectura usa REPEATABLE READ READ ONLY, presupuesto por sentencia y deadline total25s. Si excede límites o el origen falla, no anuncia completitud ni adelanta checkpoint.

Los sobres normalizados anteriores se conservan en `corvus.legacy_sales_raw`; no se presentan como capturas bronce originales. La promoción actual escribe `corvus_silver` y `corvus_gold` junto con manifiesto/head/checkpoint/auditoría. Bronce se guarda en una transacción anterior y permanece ante un rollback de promoción.

## Instalación y actualización DEV

Requiere los checkouts hermanos de SDK/Corvus/Reports y Docker. Respaldar los catálogos antes de migrar. Compilar los dos backends y sus UI; después:

```sh
cargo build --locked
npm --prefix apps/ui run build -- --base-href /corvus/
python3 scripts/run_api.py . migrate
python3 scripts/run_api.py ../Tordo.Reports migrate
python3 scripts/deploy_dev_containers.py
python3 scripts/apply_dev_routes.py
```

Los binarios/UI se montan de sólo lectura; es necesario recrear las APIs para cargar nuevos artefactos. El script de rutas espera que cada ruta resuelva al servicio correcto tras la renovación de DNS del gateway. No cambia PROD.

Preparar la fuente y las referencias es una operación del operador. Los scripts `provision_acme_source.py`, `prepare_composable_dev.py` y `Tordo.Reports/scripts/install_acme_pilot.py` son preparación/importación del ejemplo existente y sus fixtures, no implementaciones del producto por empresa. Los secretos permanecen en `.runtime`, fuera de Git.

Para una nueva empresa: aprobar su lector/exportación, configurar su referencia, crear el pipeline con el mismo dominio, ejecutar una carga y ligar la conexión Reports a ese pipeline mediante `configure_serving_binding.py`. No se cambia código de UI, dominio ni queries por añadir una empresa.

## Verificación

```sh
python3 scripts/verify_composable.py
python3 scripts/verify_dev.py
node scripts/verify_composable_ui.mjs
```

El verificador compuesto usa el ejemplo real de Acme y un segundo espacio de QA con vistas de laboratorio. Prueba dos pipelines con la misma identidad de run, corte/normalización/oro, conservación de bronce ante mala calidad, rollback real de promoción y CAS. También configura consultas/gráficos/tableros/publicaciones independientes en Reports y verifica sus exportaciones y permisos.

Usa triggers temporales de fault injection en los catálogos de laboratorio, restaurados con `finally`, y reinicia únicamente Corvus DEV para comprobar ausencia de fallback. No ejecutarlo en producción. El verificador de navegador requiere Playwright/Chromium locales y axe-core en `.runtime/browser/node_modules`; recorre el editor genérico y produce capturas escritorio/móvil.

Los endpoints de lectura/exportación autorizan por empresa. El login interactivo completo, ACL por recurso, programación, delta/CDC, nuevos perfiles y workers siguen pendientes. Las capas actuales viven en PostgreSQL; no se anuncia Parquet/objeto storage por usar los nombres bronce/plata/oro.
