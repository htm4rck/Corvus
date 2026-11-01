package com.tordo.corvus.dto;

import java.time.LocalDate;
import java.util.List;

public record KpiSummaryResponse(
        String companyId,
        LocalDate from,
        LocalDate to,
        List<KpiValue> kpis) {

    public record KpiValue(String code, String name, String unit, String value, String status) {
    }
}
