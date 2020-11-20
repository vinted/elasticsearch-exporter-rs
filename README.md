# Elasticsearch exporter [![Build Status](https://travis-ci.org/vinted/elasticsearch-exporter-rs.svg?branch=master)](https://travis-ci.org/vinted/elasticsearch-exporter-rs)

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![BSD-3 licensed][bsd-badge]][bsd-url]

[crates-badge]: https://img.shields.io/crates/v/elasticsearch_exporter.svg
[crates-url]: https://crates.io/crates/elasticsearch_exporter
[docs-badge]: https://docs.rs/elasticsearch_exporter/badge.svg
[docs-url]: https://docs.rs/elasticsearch_exporter
[bsd-badge]: https://img.shields.io/badge/license-bsd-3.svg
[bsd-url]: LICENSE

Prometheus Elasticsearch exporter capable of working with large clusters.
**Caution you may overload Prometheus server by enabling all metrics**, exporter is capable to export over near 1 million metrics.
To avoid overloading Prometheus server run multiple Elasticsearch exporters that target just few specific metrics.

```bash
$ curl -s http://127.0.0.1:9222/metrics | wc
 940272 1887011 153668390
```

## Features

 - Metric collection is decoupled from serving `/metrics` page
 - Skips zero/empty metrics (controlled with flag `exporter_skip_zero_metrics`)
 - Elasticsearch "millis" converted to seconds
 - Gauges everywhere
 - Histograms for time
 - All time based metrics are converted as f64 seconds, keywords `millis` replaced with `seconds`
 - Added `_bytes` and `_seconds` postfix
 - Preserves metrics tree namespace up to last leaf
   - elasticsearch_cat_indices_pri_warmer_total_time_seconds_bucket
   - elasticsearch_cat_health_unassign
   - elasticsearch_nodes_info_jvm_mem_heap_max_in_bytes
 - Custom namespace labels `vin_cluster_version` for convenient comparison of metrics between cluster versions
 - Automatic cluster metadata updates every 5 minutes
 - /_nodes/* API additional labels injection by mapping node ID to fetched cluster metadata
   - name (map from node ID to name) namespaced -> `name`
   - version (Elasticsearch node version) namespaced to `vin_cluster_version`
   - IP namespaced -> `ip`

## Options

 - Configurable labels "skip" and/or "include" (flags: `exporter_include_labels`, `exporter_skip_labels`)
 - Configurable skip metrics (controlled with (flag `exporter_skip_metrics`)
 - Configurable global timeout (flag `elasticsearch_global_timeout`)
 - Configurable global polling interval (flag `exporter_poll_default_interval`)
 - Configurable per metric polling interval (flag `exporter_poll_intervals`)
 - Configurable histogram buckets (flag `exporter_histogram_buckets`)
 - Configurable metrics collection (flag `exporter_metrics_enabled`)
 - Configurable metadata collection (flag `exporter_metadata_refresh_interval`)

```shell
$ curl -s http://127.0.0.1:9222
Vinted Elasticsearch exporter
elasticsearch_url: http://127.0.0.1:9200
elasticsearch_global_timeout: 30s
exporter_skip_labels:
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
 - cluster_stats: segment,patterns
exporter_include_labels:
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
 - nodes_info: name
 - nodes_stats: name,vin_cluster_version
 - nodes_usage: name
exporter_skip_metrics:
 - cat_aliases: filter,routing_index,routing_search,is_write_index
 - cat_nodeattrs: pid
 - cat_recovery: start_time,start_time_millis,stop_time,stop_time_millis
 - cat_templates: order
 - nodes_usage: _nodes_total,_nodes_successful,since
exporter_poll_default_interval: 5s
exporter_poll_intervals:
 - cluster_health: 5s
exporter_histogram_buckets: [0.02, 0.04, 0.06, 0.08, 0.1, 0.25, 0.5, 0.75, 1.0, 2.0, 4.0, 6.0, 8.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0] in seconds
exporter_skip_zero_metrics: true
exporter_metrics_enabled:
 - cat_health: true
 - cat_indices: true
 - nodes_info: true
 - nodes_stats: true
exporter_metadata_refresh_interval: 300s
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
