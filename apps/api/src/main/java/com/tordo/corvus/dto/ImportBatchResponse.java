package com.tordo.corvus.dto;

import java.time.LocalDate;
import java.time.OffsetDateTime;
import java.util.UUID;

public record ImportBatchResponse(
        UUID id,
        String tenantId,
        String companyId,
        String domain,
        String sourceSystem,
        LocalDate periodFrom,
        LocalDate periodTo,
        String aggregationLevel,
        String originalFile,
        String checksum,
        String schemaVersion,
        int receivedRecords,
        int validRecords,
        int rejectedRecords,
        String status,
        String uploadedBy,
        OffsetDateTime uploadedAt,
        OffsetDateTime updatedAt) {
}
