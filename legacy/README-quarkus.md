# Corvus Intelligence Platform

Corvus es el componente de inteligencia de datos para el ecosistema Tordo. Su primera versión expone una API Quarkus para recibir eventos operativos, registrar lotes históricos y habilitar KPIs iniciales sobre una base PostgreSQL compartida.

## Stack

| Capa | Tecnología |
| --- | --- |
| Backend | Java 21, Quarkus 3.17 |
| Persistencia | PostgreSQL, Hibernate ORM Panache, Flyway |
| Observabilidad | SmallRye Health, Micrometer Prometheus, OpenAPI |
| Infra local | Docker Compose, `tordo_network` |
| Integración prevista | RabbitMQ, MinIO, Kong, Keycloak |

## Endpoints iniciales

```http
GET  /health
GET  /metrics
GET  /openapi
GET  /api/v1/capabilities
POST /api/v1/events
POST /api/v1/import-batches
GET  /api/v1/import-batches/{id}
POST /api/v1/import-batches/{id}/validate
POST /api/v1/import-batches/{id}/confirm
GET  /analytics/v1/companies/{companyId}/kpis
```

## Ejecución local

Corvus espera que `tordo.platform` haya creado `tordo_network` y que PostgreSQL esté disponible como `tordo_postgres`.

```bash
cd Corvus/infra
docker compose up --build
```

Para desarrollo directo:

```bash
cd Corvus/apps/api
./gradlew quarkusDev
```

## Contrato de evento

```json
{
  "eventId": "evt-001",
  "eventType": "sale.completed",
  "eventVersion": "1.0",
  "occurredAt": "2026-07-21T10:30:00-05:00",
  "tenantId": "tenant-001",
  "companyId": "company-001",
  "sourceSystem": "amazilia",
  "entityId": "sale-12345",
  "data": {
    "totalAmount": 125.5,
    "currency": "PEN"
  }
}
```

Corvus guarda el evento original como JSONB antes de cualquier normalización. El identificador `eventId` es único por `tenantId`.

## Siguiente implementación recomendada

1. Agregar almacenamiento Raw en MinIO para archivos históricos y documentos.
2. Implementar `POST /api/v1/import-batches/{id}/files` con multipart upload, checksum y antivirus.
3. Publicar consumidores RabbitMQ para eventos `sale.completed`, `payment.confirmed` e `inventory.receipt.completed`.
4. Crear tablas standardized y curated para ventas diarias, gastos, compras, bancos e inventario.
5. Agregar cierre diario y cierre mensual versionado.
