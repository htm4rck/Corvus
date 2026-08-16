package com.tordo.corvus.domain;

import io.quarkus.hibernate.orm.panache.PanacheEntityBase;
import jakarta.persistence.Column;
import jakarta.persistence.Entity;
import jakarta.persistence.Id;
import jakarta.persistence.PrePersist;
import jakarta.persistence.Table;
import java.time.OffsetDateTime;
import java.util.UUID;

@Entity
@Table(name = "ingestion_events", schema = "corvus")
public class IngestionEventEntity extends PanacheEntityBase {
    @Id
    public UUID id;

    @Column(name = "tenant_id", nullable = false)
    public String tenantId;

    @Column(name = "company_id", nullable = false)
    public String companyId;

    @Column(name = "event_id", nullable = false)
    public String eventId;

    @Column(name = "event_type", nullable = false)
    public String eventType;

    @Column(name = "event_version", nullable = false)
    public String eventVersion;

    @Column(name = "source_system", nullable = false)
    public String sourceSystem;

    @Column(name = "entity_id")
    public String entityId;

    @Column(name = "occurred_at", nullable = false)
    public OffsetDateTime occurredAt;

    @Column(name = "ingested_at", nullable = false)
    public OffsetDateTime ingestedAt;

    @Column(name = "payload", nullable = false, columnDefinition = "jsonb")
    public String payload;

    @Column(name = "quality_status", nullable = false)
    public String qualityStatus;

    @PrePersist
    void prePersist() {
        if (id == null) {
            id = UUID.randomUUID();
        }
        if (ingestedAt == null) {
            ingestedAt = OffsetDateTime.now();
        }
        if (qualityStatus == null) {
            qualityStatus = "RECEIVED";
        }
    }
}
