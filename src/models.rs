use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube_custom_resources_rs::monitoring_coreos_com::v1::prometheuses::PrometheusSpec;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct KamutConfig {
    pub name: String,
    pub kind: Option<String>,
    pub image: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub resources: Option<Resources>,

    // Deployment specific fields
    #[serde(rename = "replicaCount")]
    pub replica_count: Option<i32>,

    // Prometheus specific fields
    pub replicas: Option<i32>,
    pub retention: Option<String>,

    // Sub-configs for different types
    pub deployment: Option<DeploymentConfig>,
    pub prometheus: Option<PrometheusConfig>,
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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Prometheus {
    pub metadata: ObjectMeta,
    pub spec: PrometheusSpec,
}
