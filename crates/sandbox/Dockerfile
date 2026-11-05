# Hermes Agent Sandbox Base Image
# Provides a minimal, secure environment for running crabjar agent workloads
# with terminal support and proper isolation.

FROM alpine:3.21

# Install essential dependencies for agent runtime
RUN apk --no-cache add \
    bash \
    coreutils \
    procps \
    util-linux \
    openssh-client \
    git \
    curl \
    wget \
    jq \
    ca-certificates \
    tzdata \
    && rm -rf /var/cache/apk/*

# Create dedicated agent user with proper home directory
RUN addgroup -g 1000 agent && \
    adduser -u 1000 -g agent -s /bin/bash -D agent && \
    mkdir -p /home/agent/workspace && \
    chown -R agent:agent /home/agent

# Set working directory
WORKDIR /home/agent/workspace

# Default environment variables for agent sandboxes
ENV HOME=/home/agent \
    PATH=/usr/local/bin:/usr/bin:/bin:/usr/local/sbin:/usr/sbin:/sbin \
    SHELL=/bin/bash \
    LANG=en_US.UTF-8 \
    LC_ALL=en_US.UTF-8

# Expose standard ports for agent services (optional)
EXPOSE 8080

# Run as non-root agent user by default
USER agent:agent

# Default command provides interactive shell
CMD ["/bin/bash", "-l"]