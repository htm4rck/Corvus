package com.tordo.corvus.application;

import com.tordo.corvus.domain.ImportBatchEntity;
import com.tordo.corvus.dto.CreateImportBatchRequest;
import com.tordo.corvus.dto.ImportBatchResponse;
import com.tordo.corvus.dto.QualitySummaryResponse;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.transaction.Transactional;
import jakarta.ws.rs.NotFoundException;
import java.util.UUID;

@ApplicationScoped
public class ImportBatchService {
    @Transactional
    public ImportBatchResponse create(CreateImportBatchRequest request) {
        ImportBatchEntity entity = new ImportBatchEntity();
        entity.tenantId = request.tenantId();
        entity.companyId = request.companyId();
        entity.domain = request.domain();
        entity.sourceSystem = request.sourceSystem();
        entity.periodFrom = request.periodFrom();
        entity.periodTo = request.periodTo();
        entity.aggregationLevel = request.aggregationLevel();
        entity.originalFile = request.originalFile();
        entity.checksum = request.checksum();
        entity.schemaVersion = request.schemaVersion() == null || request.schemaVersion().isBlank()
                ? "1.0"
                : request.schemaVersion();
        entity.uploadedBy = request.uploadedBy();
        entity.status = "CREATED";
        entity.persist();
        return toResponse(entity);
    }

    public ImportBatchResponse get(UUID id) {
        return toResponse(find(id));
    }

    @Transactional
    public QualitySummaryResponse validate(UUID id) {
        ImportBatchEntity entity = find(id);
        entity.status = "VALIDATED";
        entity.validRecords = entity.receivedRecords;
        entity.rejectedRecords = 0;
        return new QualitySummaryResponse(entity.status, entity.receivedRecords, entity.validRecords, entity.rejectedRecords, "CONFIRM_IMPORT");
    }

    @Transactional
    public ImportBatchResponse confirm(UUID id) {
        ImportBatchEntity entity = find(id);
        entity.status = "CONFIRMED";
        return toResponse(entity);
    }

    private ImportBatchEntity find(UUID id) {
        ImportBatchEntity entity = ImportBatchEntity.findById(id);
        if (entity == null) {
            throw new NotFoundException("Import batch not found: " + id);
        }
        return entity;
    }

    private ImportBatchResponse toResponse(ImportBatchEntity entity) {
        return new ImportBatchResponse(
                entity.id,
                entity.tenantId,
                entity.companyId,
                entity.domain,
                entity.sourceSystem,
                entity.periodFrom,
                entity.periodTo,
                entity.aggregationLevel,
                entity.originalFile,
                entity.checksum,
                entity.schemaVersion,
                entity.receivedRecords,
                entity.validRecords,
                entity.rejectedRecords,
                entity.status,
                entity.uploadedBy,
                entity.uploadedAt,
                entity.updatedAt);
    }
}
