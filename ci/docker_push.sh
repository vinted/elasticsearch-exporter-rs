#!/bin/bash

DOCKER_TAG=${TRAVIS_TAG:-latest}

TAG="ernestasvinted/elasticsearch_exporter:$DOCKER_TAG"

docker build -t $TAG .
echo "$DOCKER_PASSWORD" | docker login -u "$DOCKER_USERNAME" --password-stdin
docker push $TAG
