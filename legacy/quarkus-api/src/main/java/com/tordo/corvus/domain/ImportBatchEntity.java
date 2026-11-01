package com.tordo.corvus.domain;

import io.quarkus.hibernate.orm.panache.PanacheEntityBase;
import jakarta.persistence.Column;
import jakarta.persistence.Entity;
import jakarta.persistence.Id;
import jakarta.persistence.PrePersist;
import jakarta.persistence.PreUpdate;
import jakarta.persistence.Table;
import java.time.LocalDate;
import java.time.OffsetDateTime;
import java.util.UUID;

@Entity
@Table(name = "import_batches", schema = "corvus")
public class ImportBatchEntity extends PanacheEntityBase {
    @Id
    public UUID id;

    @Column(name = "tenant_id", nullable = false)
    public String tenantId;

    @Column(name = "company_id", nullable = false)
    public String companyId;

    @Column(name = "domain", nullable = false)
    public String domain;

    @Column(name = "source_system", nullable = false)
    public String sourceSystem;

    @Column(name = "period_from")
    public LocalDate periodFrom;

    @Column(name = "period_to")
    public LocalDate periodTo;

    @Column(name = "aggregation_level", nullable = false)
    public String aggregationLevel;

    @Column(name = "original_file")
    public String originalFile;

    @Column(name = "checksum")
    public String checksum;

    @Column(name = "schema_version", nullable = false)
    public String schemaVersion;

    @Column(name = "received_records", nullable = false)
    public int receivedRecords;

    @Column(name = "valid_records", nullable = false)
    public int validRecords;

    @Column(name = "rejected_records", nullable = false)
    public int rejectedRecords;

    @Column(name = "status", nullable = false)
    public String status;

    @Column(name = "uploaded_by")
    public String uploadedBy;

    @Column(name = "uploaded_at", nullable = false)
    public OffsetDateTime uploadedAt;

    @Column(name = "updated_at", nullable = false)
    public OffsetDateTime updatedAt;

    @PrePersist
    void prePersist() {
        if (id == null) {
            id = UUID.randomUUID();
        }
        OffsetDateTime now = OffsetDateTime.now();
        if (uploadedAt == null) {
            uploadedAt = now;
        }
        updatedAt = now;
        if (status == null) {
            status = "CREATED";
        }
    }

    @PreUpdate
    void preUpdate() {
        updatedAt = OffsetDateTime.now();
    }
}
