package com.tordo.corvus.dto;

import jakarta.validation.constraints.NotBlank;
import java.time.LocalDate;

public record CreateImportBatchRequest(
        @NotBlank String tenantId,
        @NotBlank String companyId,
        @NotBlank String domain,
        @NotBlank String sourceSystem,
        LocalDate periodFrom,
        LocalDate periodTo,
        @NotBlank String aggregationLevel,
        String originalFile,
        String checksum,
        String schemaVersion,
        String uploadedBy) {
}
