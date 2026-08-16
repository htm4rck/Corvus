package com.tordo.corvus.dto;

import jakarta.validation.constraints.NotBlank;
import jakarta.validation.constraints.NotNull;
import java.time.OffsetDateTime;
import java.util.Map;

public record IngestionEventRequest(
        @NotBlank String eventId,
        @NotBlank String eventType,
        @NotBlank String eventVersion,
        @NotNull OffsetDateTime occurredAt,
        @NotBlank String tenantId,
        @NotBlank String companyId,
        @NotBlank String sourceSystem,
        String entityId,
        Map<String, Object> data) {
}
