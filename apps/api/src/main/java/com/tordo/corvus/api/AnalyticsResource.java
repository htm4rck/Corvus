package com.tordo.corvus.api;

import com.tordo.corvus.application.AnalyticsService;
import jakarta.ws.rs.DefaultValue;
import jakarta.ws.rs.GET;
import jakarta.ws.rs.HeaderParam;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.PathParam;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.QueryParam;
import jakarta.ws.rs.core.MediaType;
import java.time.LocalDate;

@Path("/analytics/v1/companies/{companyId}/kpis")
@Produces(MediaType.APPLICATION_JSON)
public class AnalyticsResource {
    private final AnalyticsService analyticsService;

    public AnalyticsResource(AnalyticsService analyticsService) {
        this.analyticsService = analyticsService;
    }

    @GET
    public Object kpis(
            @HeaderParam("X-Tenant-Id") @DefaultValue("default") String tenantId,
            @PathParam("companyId") String companyId,
            @QueryParam("from") String from,
            @QueryParam("to") String to) {
        LocalDate dateTo = to == null || to.isBlank() ? LocalDate.now() : LocalDate.parse(to);
        LocalDate dateFrom = from == null || from.isBlank() ? dateTo.minusMonths(12) : LocalDate.parse(from);
        return analyticsService.summary(tenantId, companyId, dateFrom, dateTo);
    }
}
