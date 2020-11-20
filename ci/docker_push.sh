#!/bin/bash

TAG=ernestasvinted/elasticsearch_exporter

docker build -t $TAG .
echo "$DOCKER_PASSWORD" | docker login -u "$DOCKER_USERNAME" --password-stdin
docker push $TAG
