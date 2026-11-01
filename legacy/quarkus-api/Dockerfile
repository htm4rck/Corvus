FROM eclipse-temurin:21-jdk-alpine AS build
WORKDIR /app

COPY gradle/ gradle/
COPY gradlew settings.gradle build.gradle gradle.properties ./
RUN sed -i 's/\r$//' gradlew && chmod +x gradlew
RUN ./gradlew quarkusGoOffline --no-daemon || true

COPY src/ src/
RUN ./gradlew quarkusBuild --no-daemon

FROM eclipse-temurin:21-jre-alpine
WORKDIR /app
COPY --from=build /app/build/quarkus-app/ ./
EXPOSE 8086
ENTRYPOINT ["java", "-jar", "quarkus-run.jar"]
