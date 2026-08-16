package com.tordo.corvus.dto;

import java.time.OffsetDateTime;
import java.util.UUID;

public record IngestionEventResponse(
        UUID id,
        String eventId,
        String status,
        OffsetDateTime ingestedAt) {
}
