#!/bin/bash

GIT_TAG=$(git describe --tags `git rev-list --tags --max-count=1`)
DOCKER_TAG=${TRAVIS_TAG:-$GIT_TAG}

TAG="ernestasvinted/elasticsearch_exporter:$DOCKER_TAG"

docker build -t $TAG .
echo "$DOCKER_PASSWORD" | docker login -u "$DOCKER_USERNAME" --password-stdin
docker push $TAG
