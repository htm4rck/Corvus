package com.tordo.corvus.api;

import com.tordo.corvus.application.ImportBatchService;
import com.tordo.corvus.dto.CreateImportBatchRequest;
import jakarta.validation.Valid;
import jakarta.ws.rs.Consumes;
import jakarta.ws.rs.GET;
import jakarta.ws.rs.POST;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.PathParam;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import java.util.UUID;

@Path("/api/v1/import-batches")
@Consumes(MediaType.APPLICATION_JSON)
@Produces(MediaType.APPLICATION_JSON)
public class ImportBatchResource {
    private final ImportBatchService importBatchService;

    public ImportBatchResource(ImportBatchService importBatchService) {
        this.importBatchService = importBatchService;
    }

    @POST
    public Response create(@Valid CreateImportBatchRequest request) {
        return Response.status(Response.Status.CREATED).entity(importBatchService.create(request)).build();
    }

    @GET
    @Path("/{id}")
    public Response get(@PathParam("id") UUID id) {
        return Response.ok(importBatchService.get(id)).build();
    }

    @POST
    @Path("/{id}/validate")
    public Response validate(@PathParam("id") UUID id) {
        return Response.ok(importBatchService.validate(id)).build();
    }

    @POST
    @Path("/{id}/confirm")
    public Response confirm(@PathParam("id") UUID id) {
        return Response.ok(importBatchService.confirm(id)).build();
    }
}
