package com.tordo.corvus.application;

import com.tordo.corvus.domain.IngestionEventEntity;
import com.tordo.corvus.domain.ImportBatchEntity;
import com.tordo.corvus.dto.KpiSummaryResponse;
import jakarta.enterprise.context.ApplicationScoped;
import java.time.LocalDate;
import java.util.List;

@ApplicationScoped
public class AnalyticsService {
    public KpiSummaryResponse summary(String tenantId, String companyId, LocalDate from, LocalDate to) {
        long receivedEvents = IngestionEventEntity.count("tenantId = ?1 and companyId = ?2", tenantId, companyId);
        long importBatches = ImportBatchEntity.count("tenantId = ?1 and companyId = ?2", tenantId, companyId);

        return new KpiSummaryResponse(companyId, from, to, List.of(
                new KpiSummaryResponse.KpiValue("EVENTS_RECEIVED", "Eventos recibidos", "count", Long.toString(receivedEvents), "OBSERVED"),
                new KpiSummaryResponse.KpiValue("IMPORT_BATCHES", "Lotes historicos", "count", Long.toString(importBatches), "OBSERVED"),
                new KpiSummaryResponse.KpiValue("FINANCIAL_MODEL", "Modelo financiero", "state", "BOOTSTRAP", "PENDING_DATA")));
    }
}
