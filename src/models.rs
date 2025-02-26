use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct KamutConfig {
    pub name: String,
    pub kind: String,
    pub image: String,
    pub env: Option<std::collections::HashMap<String, String>>,
    pub resources: Option<Resources>,
    #[serde(rename = "replicaCount")]
    pub replica_count: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct Resources {
    pub requests: Option<ResourceSpec>,
    pub limits: Option<ResourceSpec>,
}

#[derive(Debug, Deserialize)]
pub struct ResourceSpec {
    pub cpu: Option<String>,
    pub memory: Option<String>,
}
