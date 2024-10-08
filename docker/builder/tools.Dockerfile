### Tools Image ###
FROM debian-base AS tools

RUN echo "deb http://deb.debian.org/debian bullseye main" > /etc/apt/sources.list.d/bullseye.list && \
    echo "Package: *\nPin: release n=bullseye\nPin-Priority: 50" > /etc/apt/preferences.d/bullseye

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get --no-install-recommends --allow-downgrades -y \
    install \
    wget \
    curl \
    perl-base=5.32.1-4+deb11u1 \
    libtinfo6=6.2+20201114-2+deb11u1 \
    git \
    libssl1.1 \
    ca-certificates \
    socat \
    python3-botocore/bullseye \
    awscli/bullseye

RUN ln -s /usr/bin/python3 /usr/local/bin/python
COPY --link docker/tools/boto.cfg /etc/boto.cfg

RUN wget https://storage.googleapis.com/pub/gsutil.tar.gz -O- | tar --gzip --directory /opt --extract && ln -s /opt/gsutil/gsutil /usr/local/bin
RUN cd /usr/local/bin && wget "https://storage.googleapis.com/kubernetes-release/release/v1.18.6/bin/linux/amd64/kubectl" -O kubectl && chmod +x kubectl

COPY --link --from=tools-builder /diem/dist/diem-db-bootstrapper /usr/local/bin/diem-db-bootstrapper
COPY --link --from=tools-builder /diem/dist/diem-db-tool /usr/local/bin/diem-db-tool
COPY --link --from=tools-builder /diem/dist/diem /usr/local/bin/diem
COPY --link --from=tools-builder /diem/dist/diem-openapi-spec-generator /usr/local/bin/diem-openapi-spec-generator
COPY --link --from=tools-builder /diem/dist/diem-fn-check-client /usr/local/bin/diem-fn-check-client
COPY --link --from=tools-builder /diem/dist/diem-transaction-emitter /usr/local/bin/diem-transaction-emitter

### Get Diem Move releases for genesis ceremony
RUN mkdir -p /diem-framework/move
COPY --link --from=tools-builder /diem/dist/head.mrb /diem-framework/move/head.mrb

# add build info
ARG BUILD_DATE
ENV BUILD_DATE ${BUILD_DATE}
ARG GIT_TAG
ENV GIT_TAG ${GIT_TAG}
ARG GIT_BRANCH
ENV GIT_BRANCH ${GIT_BRANCH}
ARG GIT_SHA
ENV GIT_SHA ${GIT_SHA}
