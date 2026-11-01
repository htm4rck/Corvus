package com.tordo.corvus.api;

import jakarta.ws.rs.GET;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.core.MediaType;
import java.util.List;
import java.util.Map;

@Path("/api/v1/capabilities")
@Produces(MediaType.APPLICATION_JSON)
public class CapabilityResource {
    @GET
    public Map<String, Object> capabilities() {
        return Map.of(
                "component", "Corvus Intelligence Platform",
                "status", "BOOTSTRAP",
                "zones", List.of("RAW", "STANDARDIZED", "CURATED", "DOCUMENT"),
                "domains", List.of("sales", "purchases", "expenses", "inventory", "banking", "receivables", "payables"),
                "contracts", List.of("/api/v1/events", "/api/v1/import-batches", "/analytics/v1/companies/{companyId}/kpis"));
    }
}
