use byte_unit::Byte;
use serde_json::Value;
use std::convert::TryFrom;
use std::time::Duration;

use super::{MetricError, RawMetric};

/// Parsed metric types
#[derive(Debug, PartialEq)]
pub enum MetricType {
    /// Time is parsed as duration in milliseconds
    /// duration is casted to float seconds
    Time(Duration), // Milliseconds
    /// Bytes
    Bytes(i64),
    /// Integer gauges
    Gauge(i64),
    /// Float gauges
    GaugeF(f64),
    /// Switch metrics having value of true/false
    Switch(u8),

    /// Labels e.g.: index, node, ip, etc.
    Label(String), // Everything not number

    /// Null value
    Null,
}

impl<'s> TryFrom<RawMetric<'s>> for MetricType {
    type Error = MetricError;

    fn try_from(metric: RawMetric) -> Result<Self, MetricError> {
        let value: &Value = metric.1;

        let unknown = || MetricError::unknown(metric.0.to_owned(), Some(value.clone()));

        let parse_i64 = || -> Result<i64, MetricError> {
            if value.is_number() {
                Ok(value.as_i64().unwrap_or(0))
            } else {
                value
                    .as_str()
                    .map(|n| n.parse::<i64>())
                    .ok_or_else(unknown)?
                    .map_err(|e| MetricError::from_parse_int(e, Some(value.clone())))
            }
        };

        let parse_f64 = || -> Result<f64, MetricError> {
            if value.is_f64() {
                Ok(value.as_f64().unwrap_or(0.0))
            } else {
                value
                    .as_str()
                    // .replace is handling string percent notation, e.g.: "3.44%"
                    .map(|n| n.replace("%", "").parse::<f64>())
                    .ok_or_else(unknown)?
                    .map_err(|e| MetricError::from_parse_float(e, Some(value.clone())))
            }
        };

        if value.is_boolean() {
            return Ok(MetricType::Switch(if value.as_bool().unwrap_or(false) {
                1
            } else {
                0
            }));
        }

        if value.is_null() {
            return Ok(MetricType::Null);
        }

        // "get.total": "0", INT
        // "disk.total": "475894423552", BYTES
        match metric.0 {
            "indices" | "avail" | "used" | "memory" | "store" | "bytes" => {
                // /_nodes/stats returns size with size postfix: kb, b, gb
                // in case parsing to integer fails fallback and try to
                // parse byte unit
                return match parse_i64() {
                    Ok(int) => Ok(MetricType::Bytes(int)),
                    Err(e) => {
                        if let Some(byte_str) = value.as_str() {
                            return Ok(MetricType::Bytes(
                                // FIX: Possible accuracy loss (Prometheus accepts up to 64 bits)
                                Byte::from_str(byte_str).map(|b| b.get_bytes()).or(Err(e))? as i64,
                            ));
                        }

                        Err(e)
                    }
                };
            }
            // elasticsearch_nodes_stats_fs_io_stats_total_write_kilobytes
            "kilobytes" => {
                return match parse_i64() {
                    Ok(int) => Ok(MetricType::Bytes(int * 1024)),
                    Err(e) => {
                        if let Some(byte_str) = value.as_str() {
                            return Ok(MetricType::Bytes(
                                // FIX: Possible accuracy loss (Prometheus accepts up to 64 bits)
                                Byte::from_str(byte_str).map(|b| b.get_bytes()).or(Err(e))? as i64
                                    * 1024,
                            ));
                        }

                        Err(e)
                    }
                };
            }
            // Skip these metrics as highly variable or redundant
            "installed" | "jdk" | "pid" | "date" | "epoch" | "timestamp" | "uptime" => {
                return Ok(MetricType::Null)
            }

            "time" | "millis" | "alive" => {
                return Ok(MetricType::Time(Duration::from_millis(
                    parse_i64().unwrap_or(0) as u64,
                )))
            }
            _ => {
                if value.is_number() {
                    if value.is_i64() {
                        return Ok(MetricType::Gauge(parse_i64()?));
                    } else {
                        return Ok(MetricType::GaugeF(parse_f64()?));
                    }
                }
            }
        }

        // TODO: rethink list matching, label could be matched by default with
        // attempt to parse number before return as default type label
        match metric.0 {
            // timed_out
            "tripped" | "enabled" | "out" | "value" | "committed" | "searchable" | "compound"
            | "throttled" => Ok(MetricType::Switch(if value.as_bool().unwrap_or(false) {
                1
            } else {
                0
            })),

            // Special cases
            // _cat/health: elasticsearch_cat_health_node_data{cluster="testing"}
            // _cat/shards: "path.data": "/var/lib/elasticsearch/m1/nodes/0"
            "data" => match parse_i64() {
                Ok(number) => Ok(MetricType::Gauge(number)),
                Err(_) => Ok(MetricType::Label(
                    value.as_str().ok_or_else(unknown)?.to_owned(),
                )),
            },

            // elasticsearch_cat_thread_pool_size - int
            // elasticsearch_nodes_stats_indices_query_cache_cache_size should be MetricType::Bytes though
            // pool_size is an int - MetricType::Gauge
            "size" => {
                // parse byte unit
                return match parse_i64() {
                    Ok(int) => Ok(MetricType::Gauge(int)),
                    Err(e) => {
                        if let Some(byte_str) = value.as_str() {
                            return Ok(MetricType::Gauge(
                                // FIX: Possible accuracy loss (Prometheus accepts up to 64 bits)
                                Byte::from_str(byte_str).map(|b| b.get_bytes()).or(Err(e))? as i64,
                            ));
                        }

                        Err(e)
                    }
                };
            }

            "overhead" | "processors" | "primaries" | "min" | "max" | "successful" | "nodes"
            | "fetch" | "order" | "largest" | "rejected" | "completed" | "queue" | "active"
            | "core" | "tasks" | "relo" | "unassign" | "init" | "files" | "ops" | "recovered"
            | "generation" | "contexts" | "listeners" | "pri" | "rep" | "docs" | "count"
            | "compilations" | "deleted" | "shards" | "checkpoint" | "cpu" | "triggered"
            | "evictions" | "failed" | "total" | "current" | "operations" => {
                Ok(MetricType::Gauge(parse_i64()?))
            }

            "avg" | "1m" | "5m" | "15m" | "number" | "percent" => {
                Ok(MetricType::GaugeF(parse_f64()?))
            }

            "types" | "usage" | "mount" | "group" | "rank" | "path" | "roles" | "context"
            | "cluster" | "repository" | "snapshot" | "stage" | "uuid" | "component" | "master"
            | "role" | "alias" | "filter" | "search" | "flavor" | "string" | "address"
            | "health" | "build" | "node" | "state" | "patterns" | "of" | "segment" | "host"
            | "ip" | "prirep" | "id" | "status" | "at" | "for" | "details" | "reason" | "port"
            | "attr" | "field" | "shard" | "index" | "name" | "type" | "version"
            | "description" => Ok(MetricType::Label(
                value.as_str().ok_or_else(unknown)?.to_owned(),
            )),
            _ => {
                if cfg!(debug_assertions) {
                    println!("Catchall metric: {:?}", metric);

                    let parsed = parse_i64().unwrap_or(-1).to_string();

                    if &parsed != "-1" && parsed.len() == value.as_str().unwrap_or("").len() {
                        println!("Unhandled metic value {:?}", metric);
                    }
                }

                Ok(MetricType::Label(
                    value.as_str().ok_or_else(unknown)?.to_owned(),
                ))
            }
        }
    }
}
