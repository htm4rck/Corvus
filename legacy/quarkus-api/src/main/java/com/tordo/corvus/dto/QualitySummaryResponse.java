package com.tordo.corvus.dto;

public record QualitySummaryResponse(
        String status,
        int receivedRecords,
        int validRecords,
        int rejectedRecords,
        String nextAction) {
}
