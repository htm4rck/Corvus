package com.tordo.corvus.api;

import com.tordo.corvus.application.IngestionService;
import com.tordo.corvus.dto.IngestionEventRequest;
import com.tordo.corvus.dto.IngestionEventResponse;
import jakarta.validation.Valid;
import jakarta.ws.rs.Consumes;
import jakarta.ws.rs.POST;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;

@Path("/api/v1/events")
@Consumes(MediaType.APPLICATION_JSON)
@Produces(MediaType.APPLICATION_JSON)
public class EventResource {
    private final IngestionService ingestionService;

    public EventResource(IngestionService ingestionService) {
        this.ingestionService = ingestionService;
    }

    @POST
    public Response receive(@Valid IngestionEventRequest request) {
        IngestionEventResponse response = ingestionService.receive(request);
        return Response.status(Response.Status.ACCEPTED).entity(response).build();
    }
}
