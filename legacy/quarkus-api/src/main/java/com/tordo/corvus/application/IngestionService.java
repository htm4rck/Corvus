package com.tordo.corvus.application;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.tordo.corvus.domain.IngestionEventEntity;
import com.tordo.corvus.dto.IngestionEventRequest;
import com.tordo.corvus.dto.IngestionEventResponse;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.transaction.Transactional;
import jakarta.ws.rs.BadRequestException;
import jakarta.ws.rs.WebApplicationException;
import jakarta.ws.rs.core.Response;
import java.time.OffsetDateTime;

@ApplicationScoped
public class IngestionService {
    private final ObjectMapper objectMapper;

    public IngestionService(ObjectMapper objectMapper) {
        this.objectMapper = objectMapper;
    }

    @Transactional
    public IngestionEventResponse receive(IngestionEventRequest request) {
        long duplicates = IngestionEventEntity.count("tenantId = ?1 and eventId = ?2", request.tenantId(), request.eventId());
        if (duplicates > 0) {
            throw new WebApplicationException("Event already received for this tenant: " + request.eventId(), Response.Status.CONFLICT);
        }

        IngestionEventEntity entity = new IngestionEventEntity();
        entity.tenantId = request.tenantId();
        entity.companyId = request.companyId();
        entity.eventId = request.eventId();
        entity.eventType = request.eventType();
        entity.eventVersion = request.eventVersion();
        entity.sourceSystem = request.sourceSystem();
        entity.entityId = request.entityId();
        entity.occurredAt = request.occurredAt();
        entity.ingestedAt = OffsetDateTime.now();
        entity.qualityStatus = "RECEIVED";
        entity.payload = serialize(request);
        entity.persist();

        return new IngestionEventResponse(entity.id, entity.eventId, entity.qualityStatus, entity.ingestedAt);
    }

    private String serialize(IngestionEventRequest request) {
        try {
            return objectMapper.writeValueAsString(request);
        } catch (JsonProcessingException ex) {
            throw new BadRequestException("Event payload cannot be serialized", ex);
        }
    }
}
