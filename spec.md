# KAMUT.YAML Specification

## Overview

This document describes the specification for `kamut.yaml` files, which are simplified configuration files for Kubernetes resources. The goal of KAMUT is to provide a more straightforward way to define Kubernetes resources that will be transformed into standard Kubernetes manifests.

## File Structure

- YAML format with `.kamut.yaml` extension
- Can contain multiple resources separated by `---`
- Each resource must have `name` and `kind` fields

## Common Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | String | Yes | Name of the resource |
| `kind` | String | Yes | Type of resource ("Deployment", "Prometheus", or "KubeScrapeConfig") |
| `namespace` | String | No | Kubernetes namespace for the resource |
| `resources` | Object | No | Resource requests and limits |
| `resources.requests.memory` | String | No | Memory request (e.g., "400Mi") |
| `resources.requests.cpu` | String | No | CPU request (e.g., "500m") |
| `resources.limits.memory` | String | No | Memory limit (e.g., "2Gi") |
| `resources.limits.cpu` | String | No | CPU limit (e.g., "1000m") |
| `node_selector` | Object | No | Key-value pairs for node selection |

## Kind-Specific Fields

### Deployment

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `image` | String | Yes | Container image to use |
| `env` | Object | No | Map of environment variables |
| `replicas` | Integer | No | Number of replicas |

### Prometheus

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `image` | String | Yes | Prometheus container image |
| `replicas` | Integer | No | Number of Prometheus instances |
| `retention` | String | No | Data retention period (default: "15d") |
| `storage` | Object | No | Persistent storage configuration |
| `storage.size` | String | No | Storage size (e.g., "100Gi") |
| `storage.className` | String | No | Storage class name (e.g., "gp3-prom") |
| `ingress` | Object | No | Ingress configuration |
| `ingress.host` | String | No | Hostname for the ingress |
| `service_account` | Object | No | Service account configuration |
| `service_account.create` | Boolean | No | Whether to create a service account (default: true) |
| `service_account.cluster_role` | Boolean | No | Whether to create cluster role/binding (default: true) |
| `service_account.annotations` | Object | No | Service account annotations |

### KubeScrapeConfig

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `role` | String | Yes | Kubernetes discovery role (pod, endpoints, service, etc.) |
| `scrapeInterval` | String | No | Scraping interval (default: "30s") |
| `scrapeTimeout` | String | No | Scraping timeout (default: "10s") |
| `metricsPath` | String | No | Path to metrics endpoint |
| `labels` | Object | No | Labels to select pods to scrape |
| `port` | String/Integer | No | Port number or name to scrape metrics from |

## Examples

### Deployment Example

```yaml
name: app-server
kind: Deployment
image: hello:v0.1.0
env:
  DATABASE_URL: IN_VAULT
  LOG_LEVEL: INFO
resources:
  requests:
    cpu: 100m
    memory: 100Mi
  limits:
    cpu: 300m
    memory: 300Mi
replicas: 2
node_selector:
  group: frontend
```

### Prometheus Example

```yaml
name: example2
kind: Prometheus
namespace: monitoring
image: prom/prometheus:v2.7.1
retention: 15d
replicas: 1
resources:
  requests:
    memory: 400Mi
    cpu: 500m
  limits:
    memory: 2Gi
    cpu: 1000m
storage:
  size: 200Gi
  className: gp3-prom
node_selector:
  group: monitoring
ingress:
  host: "example.com"
```

### KubeScrapeConfig Example

```yaml
name: hello-sc
namespace: hello
kind: KubeScrapeConfig
role: pod
scrapeInterval: 30s
scrapeTimeout: 10s
metricsPath: /metrics
labels:
  app: hello
port: 4040
```

## Multiple Resources Example

You can define multiple resources in a single file by separating them with `---`:

```yaml
name: app-server
kind: Deployment
image: hello:v0.1.0
replicas: 2
---
name: app-monitoring
kind: KubeScrapeConfig
role: pod
labels:
  app: hello
port: 8080
```