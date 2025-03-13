use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube_custom_resources_rs::monitoring_coreos_com::v1::prometheuses::PrometheusSpec;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
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
