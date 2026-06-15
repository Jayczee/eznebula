FROM eclipse-temurin:17-jre

# Install nebula binary dependencies (none needed, statically linked)
COPY binaries/linux-amd64/nebula /usr/local/bin/nebula
COPY binaries/linux-amd64/nebula-cert /usr/local/bin/nebula-cert
RUN chmod +x /usr/local/bin/nebula /usr/local/bin/nebula-cert

# Copy the Spring Boot JAR
COPY eznebula-backend/target/*.jar /app/eznebula.jar

# Environment variables (with defaults)
ENV EZNEBULA_PORT=8080
ENV EZNEBULA_LIGHTHOUSE_PORT=4242
ENV EZNEBULA_LIGHTHOUSE_IP=0.0.0.0
ENV EZNEBULA_HOME=/data/eznebula-data
ENV JAVA_OPTS="-Xms128m -Xmx256m"

WORKDIR /app

EXPOSE 8080
EXPOSE 4242/udp

ENTRYPOINT ["sh", "-c", "exec java $JAVA_OPTS \
    -Dserver.port=$EZNEBULA_PORT \
    -Deznebula.lighthouse.public-ip=$EZNEBULA_LIGHTHOUSE_IP \
    -Deznebula.lighthouse.port=$EZNEBULA_LIGHTHOUSE_PORT \
    -Duser.home=$EZNEBULA_HOME \
    -jar /app/eznebula.jar"]
