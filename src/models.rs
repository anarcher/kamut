use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube_custom_resources_rs::monitoring_coreos_com::v1::prometheuses::PrometheusSpec;
// ScrapeConfig is used directly in config.rs

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct KamutConfig {
    pub name: String,
    pub kind: Option<String>,
    pub namespace: Option<String>,
    pub image: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub resources: Option<Resources>,
    pub storage: Option<Storage>,
    pub node_selector: Option<HashMap<String, String>>,

    // Prometheus specific fields
    pub replicas: Option<i32>,
    pub retention: Option<String>,
    pub ingress: Option<Ingress>,
    pub service_account: Option<ServiceAccount>,
    
    // ScrapeConfig specific fields
    pub role: Option<String>,
    #[serde(rename = "scrapeInterval")]
    pub scrape_interval: Option<String>,
    #[serde(rename = "scrapeTimeout")]
    pub scrape_timeout: Option<String>,
    #[serde(rename = "metricsPath")]
    pub metrics_path: Option<String>,
    pub labels: Option<HashMap<String, String>>,
    pub port: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
pub struct ServiceAccount {
    #[serde(default = "default_true")]
    pub create: bool,
    pub annotations: Option<HashMap<String, String>>,
    #[serde(default)]
    pub cluster_role: Option<bool>,
}

fn default_true() -> bool {
    true
}

impl Default for KamutConfig {
    fn default() -> Self {
        KamutConfig {
            name: "default".to_string(),
            kind: None,
            namespace: None,
            image: None,
            env: None,
            resources: None,
            storage: None,
            node_selector: None,
            replicas: None,
            retention: None,
            ingress: None,
            service_account: None,
            role: None,
            scrape_interval: None,
            scrape_timeout: None,
            metrics_path: None,
            labels: None,
            port: None,
        }
    }
}

impl Default for ServiceAccount {
    fn default() -> Self {
        ServiceAccount {
            create: true,
            annotations: None,
            cluster_role: Some(true),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
pub struct Ingress {
    pub host: String,
}

#[derive(Debug, Deserialize)]
pub struct DeploymentConfig {
    pub name: Option<String>,
    pub image: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub resources: Option<Resources>,
    #[serde(rename = "replicaCount")]
    pub replica_count: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct PrometheusConfig {
    pub name: Option<String>,
    pub image: Option<String>,
    pub replicas: Option<i32>,
    pub retention: Option<String>,
    pub resources: Option<Resources>,
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
pub struct Resources {
    pub requests: Option<ResourceSpec>,
    pub limits: Option<ResourceSpec>,
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
pub struct ResourceSpec {
    pub cpu: Option<String>,
    pub memory: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
pub struct Storage {
    pub size: String,
    #[serde(rename = "className")]
    pub class_name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Prometheus {
    pub metadata: ObjectMeta,
    pub spec: PrometheusSpec,
}

// ScrapeConfig is now imported from kube_custom_resources_rs
