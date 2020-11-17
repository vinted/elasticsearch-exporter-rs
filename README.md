# Elasticsearch exporter

Prometheus Elasticsearch exporter capable of working with large clusters.

## Features

 - Skips if metric zero/empty metrics
 - Metric collection is decoupled from rendering `/metrics` page

## Options

```shell
$ curl -s localhost:9222
Vinted Elasticsearch exporter
elasticsearch_url: http://127.0.0.1:9200
elasticsearch_global_timeout: 30s
elasticsearch_skip_labels
 - cat_allocation: health,status
 - cat_fielddata: id
 - cat_indices: health,status
 - cat_nodeattrs: id
 - cat_nodes: health,status,pid
 - cat_plugins: id,description
 - cat_segments: health,status,checkpoint,prirep
 - cat_shards: health,status,checkpoint,prirep
 - cat_templates: composed_of
 - cat_thread_pool: node_id,ephemeral_node_id,pid
 - cat_transforms: health,status
elasticsearch_include_labels
 - cat_aliases: index,alias
 - cat_allocation: node
 - cat_fielddata: node,field
 - cat_health: shards
 - cat_indices: index
 - cat_nodeattrs: node,attr
 - cat_nodes: index,name,node_role
 - cat_pending_tasks: index
 - cat_plugins: name
 - cat_recovery: index,shard,stage,type
 - cat_repositories: index
 - cat_segments: index,shard
 - cat_shards: index,node,shard
 - cat_templates: name,index_patterns
 - cat_thread_pool: node_name,name,type
 - cat_transforms: index
 - cluster_health: status
elasticsearch_skip_metrics
 - cat_aliases: filter,routing_index,routing_search,is_write_index
 - cat_health: epoch,timestamp
 - cat_nodeattrs: pid
 - cat_recovery: start_time,start_time_millis,stop_time,stop_time_millis
 - cat_templates: order
elasticsearch_cat_headers
 - cat_nodes: *
exporter_poll_interval: 5s
exporter_histogram_buckets: [0.02, 0.04, 0.06, 0.08, 0.1, 0.25, 0.5, 0.75, 1.0, 2.0, 4.0, 6.0, 8.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0] in seconds
exporter_skip_zero_metrics: true
```

### DEFAULT SKIP LABELS

```
cat_allocation=health,status
cat_fielddata=id
cat_indices=health,status
cat_nodeattrs=id
cat_nodes=health,status,pid
cat_plugins=id,description
cat_segments=health,status,checkpoint,prirep
cat_shards=health,status,checkpoint,prirep
cat_templates=composed_of
cat_thread_pool=node_id,ephemeral_node_id,pid
cat_transforms=health,status
```


### DEFAULT INCLUDE LABELS

```
cat_health=shards
cat_aliases=index,alias
cat_allocation=node
cat_fielddata=node,field
cat_indices=index
cat_nodeattrs=node,attr
cat_nodes=index,name,node_role
cat_pending_tasks=index
cat_plugins=name
cat_recovery=index,shard,stage,type
cat_repositories=index
cat_segments=index,shard
cat_shards=index,node,shard
cat_templates=name,index_patterns
cat_thread_pool=node_name,name,type
cat_transforms=index
```

### DEFAULT SKIP METRICS

```
cat_health=epoch,timestamp
cat_aliases=filter,routing_index,routing_search,is_write_index
cat_nodeattrs=pid
cat_recovery=start_time,start_time_millis,stop_time,stop_time_millis
cat_templates=order
```

## Development

To start:

```shell
cargo run --bin elasticsearch_exporter
```

To test:

```shell
cargo test
```

# License

BSD-3-Clause
