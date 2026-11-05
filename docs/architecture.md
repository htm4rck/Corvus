# Corvus Rust — primera fase

Decisión del propietario del 2026-10-30: rehacer el backend de Corvus en Rust y añadir una UI Angular básica. Referencia de separación: Tordo.Business/Tordo.SDK.Business. La base Quarkus anterior se conserva íntegra en `legacy/quarkus-api`; el historial Git no se reescribe.

| Propiedad | Ubicación |
|---|---|
| Modelos y políticas de evento/lote | Tordo.SDK/rust/corvus-core/src/domain.rs |
| Puertos propios | SDK corvus-core/src/application/ports.rs |
| Casos de uso y autorización semántica | SDK corvus-core/src/application/use_cases.rs |
| Entrada HTTP | apps/api/src/http.rs |
| Persistencia y transacciones | apps/api/src/persistence.rs |
| Composición/configuración de ejecución | apps/api/src/main.rs |
| Identidad y adaptadores comunes | Tordo.SDK/rust/runtime |

## Entrega construida

Se conservan rutas de eventos, creación/lectura/validación/confirmación de lotes y KPI. La unicidad `(tenant_id,event_id)` se mantiene en PostgreSQL. Repetir un evento compatible devuelve el recibo existente, incluso bajo concurrencia; contenido diferente bajo la misma identidad devuelve conflicto. Los payloads se deserializan para normalizar fechas de la representación previa.

Lotes tienen estados CREATED/VALIDATED/REJECTED/CONFIRMED. Validar exige filas reales: entre 1 y 10000 objetos con columnas homogéneas, dentro del límite HTTP de 2 MiB. Una validación fallida cuenta rechazadas; no confirma. Un lote confirmado es inmutable. Modificaciones concurrentes se resuelven mediante versión esperada y conflicto explícito.

Confirmar actualiza lote, revisión y snapshot en una transacción PostgreSQL. Un fallo de publicación revierte también el estado del lote. Los snapshots son independientes por lote y no sustituyen aún datasets curados globales/versionados. `observedThrough` permanece null: la validación estructural no demuestra completitud ni frescura de un dominio. `declaredPeriodTo` conserva la fecha declarada del lote sin presentarla como frescura acreditada.

El KPI aplica desde/hasta al conteo de eventos, con límites temporales UTC; no presenta totales financieros derivados como aceptados. El contexto autorizado viene de `business-code`; tanto `tenantId` como `companyId` deben coincidir con la empresa seleccionada en esta fase. Las rutas por objeto incluyen ese alcance; se elimina el tenant default de la API anterior.

## Migración y continuidad

La migración Rust es explícita (`cargo run -- migrate` o el binario con `migrate`), nunca DDL dentro de peticiones. Mantiene la disposición de `corvus.ingestion_events` de Flyway V1, sin borrar datos. Lotes/definiciones nuevos usan `corvus.resources`, su historial `resource_versions` y `datasets`.

Los antiguos `import_batches` se conservan, pero no se convierten automáticamente en lotes validados nuevos: el bootstrap Java validaba sin revisar filas. El corte con datos históricos exige inventario, conciliación, mapeo de empresas y reversión independiente. Los clientes deben añadir identidad y empresa autorizada; `validate` ahora exige `{rows:[...]}`. No se declara paridad automática de autenticación o payload de validación con los consumidores anteriores.

## Pendiente del alcance Corvus

Delta/CDC, correcciones/borrados incrementales, nuevos dominios, archivos/Parquet, SQL analítico general, linaje y workers recuperables. Ni MinIO ni RabbitMQ se anuncian implementados por figurar en el diseño anterior. El piloto Acme ya tiene extracción inicial manual, checkpoint de corte, conciliación, proyecciones versionadas y serving de tres consultas registradas.

### Parquet: alcance futuro acordado el 2026-11-04

La [distribución de formatos del stack](../../Tordo.SDK/rust/docs/analytics-product-model.md#parquet-en-el-alcance-futuro-del-stack) establece las obligaciones y aceptación comunes. Corvus deberá publicar en Parquet tipado los cinco conjuntos de plata comercial, sus históricos y los modelos de oro distribuidos como datasets o exportaciones masivas. La copia analítica de Accounting se incorporará mediante un contrato autorizado de diario/líneas confirmadas, preservando autoridad, reversos y precisión; cada dominio adicional necesita su propio contrato y aceptación.

Bronce conserva el contrato de entrada JSONB y su hash, incluso cuando falle la calidad. Manifiestos, runs, permisos, checkpoint, puntero vigente y auditoría siguen bajo control PostgreSQL; las materializaciones de consulta pueden permanecer allí. Las APIs y eventos conservan JSON versionado.

La entrega requiere adaptadores Parquet, procesamiento por lotes, trabajos recuperables y publicación que verifique todos los archivos antes de promover el manifiesto y el checkpoint. Debe probar aislamiento por empresa/pipeline, recuperación ante fallos y paridad contra los modelos PostgreSQL. Los límites de la carga manual actual siguen vigentes hasta entregar y verificar esa evolución. Parquet y almacenamiento de objetos continúan pendientes; esta sección no modifica las capacidades anunciadas por la API.

## DEV y verificación

Rust 1.85.1 y lockfile fijados. Las APIs usan el SDK hermano por path; todavía no hay artefacto Cargo publicado. PostgreSQL de analytics y el MySQL opcional son contenedores nuevos de laboratorio; sus datos no pertenecen a las bases operativas del stack existente. Cada catálogo usa su base y usuario propios.

`bootstrap_dev.py` crea secretos privados, provisiona esas bases, ejecuta migraciones y arranca los binarios compilados como contenedores sin privilegios. `deploy_dev_containers.py` recrea sólo las dos APIs para cargar nuevos binarios/UI. Las imágenes runtime están fijadas por digest y los volúmenes de aplicación son de sólo lectura; el DEV usa artefactos Linux compilados en este host, no imágenes de release.

`apply_dev_routes.py` obtiene la configuración actual del Kong DEV, conserva backup privado y añade/actualiza sólo los dos servicios propios. La configuración de fuente también está en Tordo.Platform/kong/kong.dev.yml. Hasta desplegar esa fuente por el flujo normal de Platform, un reinicio del Kong que recargue su checkout anterior requiere volver a ejecutar el script. No cambia Kong PROD.

```sh
cargo build --locked
cd apps/ui && npm ci && npm run build -- --base-href /corvus/
cd ../..
python3 scripts/bootstrap_dev.py
python3 scripts/bootstrap_mysql_dev.py
python3 scripts/apply_dev_routes.py
python3 scripts/verify_dev.py
```

UI: https://dev.tordo.io/corvus/ o http://127.0.0.1:8000/corvus/. Desarrollo con recarga: `npm start -- --port 4206 --proxy-config proxy.conf.json` desde apps/ui.

La evidencia reproducible se guarda en `.runtime/verification.json` y los scripts. El cierre de esta fase no declara aceptadas REP13–REP20 completas ni todo el alcance datalake/BI.

## Homologación Business y piloto Acme

La aclaración del propietario exige las garantías de Business, además de la separación inicial. La vertical comercial implementa unidad de trabajo por empresa, auditoría atómica y contratos tipados canónicos; el resto del alcance sigue abierto: ver Tordo.SDK/rust/docs/analytics-business-architecture.md. El caso comercial diseñado está en [piloto Acme](acme-sales-ingestion-design.md); el pipeline manual y sus tres reportes están desplegados en DEV. Ver [runbook](acme-pilot-runbook.md).


## Corrección del modelo de producto

Vigente: `Tordo.SDK/rust/docs/analytics-product-model.md`. Los pipelines son recursos configurables por empresa/dominio/origen. Bronce precede a calidad y persiste ante fallos; plata y oro materializados se promueven con manifiesto, head/checkpoint por pipeline y auditoría. Las capturas normalizadas del primer piloto se preservan explícitamente como legacy. No hay componentes ni modelos nombrados por Acme en los núcleos/APIs/UI. Ver [runbook vigente](runbook.md).
