use kamut::models::{Ingress, KamutConfig, Resources, ResourceSpec, Storage};
use std::collections::HashMap;

#[test]
fn test_kamut_config_deserialization() {
    // Test basic deployment config
    let yaml = r#"
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
    "#;

    let config: KamutConfig = serde_yaml::from_str(yaml).unwrap();
    
    assert_eq!(config.name, "app-server");
    assert_eq!(config.kind, Some("Deployment".to_string()));
    assert_eq!(config.image, Some("hello:v0.1.0".to_string()));
    
    // Check env vars
    let env = config.env.unwrap();
    assert_eq!(env.get("DATABASE_URL").unwrap(), "IN_VAULT");
    assert_eq!(env.get("LOG_LEVEL").unwrap(), "INFO");
    
    // Check resources
    let resources = config.resources.unwrap();
    let requests = resources.requests.unwrap();
    assert_eq!(requests.cpu, Some("100m".to_string()));
    assert_eq!(requests.memory, Some("100Mi".to_string()));
    
    let limits = resources.limits.unwrap();
    assert_eq!(limits.cpu, Some("300m".to_string()));
    assert_eq!(limits.memory, Some("300Mi".to_string()));
    
    // Check replicas
    assert_eq!(config.replicas, Some(2));
    
    // Check node selector
    let node_selector = config.node_selector.unwrap();
    assert_eq!(node_selector.get("group").unwrap(), "frontend");
}

#[test]
fn test_prometheus_config_deserialization() {
    // Test Prometheus config with ingress and storage
    let yaml = r#"
    name: example2
    kind: Prometheus
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
    "#;

    let config: KamutConfig = serde_yaml::from_str(yaml).unwrap();
    
    assert_eq!(config.name, "example2");
    assert_eq!(config.kind, Some("Prometheus".to_string()));
    assert_eq!(config.image, Some("prom/prometheus:v2.7.1".to_string()));
    assert_eq!(config.retention, Some("15d".to_string()));
    assert_eq!(config.replicas, Some(1));
    
    // Check resources
    let resources = config.resources.unwrap();
    let requests = resources.requests.unwrap();
    assert_eq!(requests.cpu, Some("500m".to_string()));
    assert_eq!(requests.memory, Some("400Mi".to_string()));
    
    let limits = resources.limits.unwrap();
    assert_eq!(limits.cpu, Some("1000m".to_string()));
    assert_eq!(limits.memory, Some("2Gi".to_string()));
    
    // Check storage
    let storage = config.storage.unwrap();
    assert_eq!(storage.size, "200Gi");
    assert_eq!(storage.class_name, "gp3-prom");
    
    // Check node selector
    let node_selector = config.node_selector.unwrap();
    assert_eq!(node_selector.get("group").unwrap(), "monitoring");
    
    // Check ingress
    let ingress = config.ingress.unwrap();
    assert_eq!(ingress.host, "example.com");
}

#[test]
fn test_missing_fields() {
    // Test with missing optional fields
    let yaml = r#"
    name: minimal-app
    kind: Deployment
    image: minimal:v1.0.0
    "#;

    let config: KamutConfig = serde_yaml::from_str(yaml).unwrap();
    
    assert_eq!(config.name, "minimal-app");
    assert_eq!(config.kind, Some("Deployment".to_string()));
    assert_eq!(config.image, Some("minimal:v1.0.0".to_string()));
    
    // Optional fields should be None
    assert!(config.env.is_none());
    assert!(config.resources.is_none());
    assert!(config.replicas.is_none());
    assert!(config.node_selector.is_none());
    assert!(config.retention.is_none());
    assert!(config.ingress.is_none());
    assert!(config.storage.is_none());
}

#[test]
fn test_missing_required_fields() {
    // Test with missing name (required)
    let yaml = r#"
    kind: Deployment
    image: hello:v0.1.0
    "#;

    let result: Result<KamutConfig, _> = serde_yaml::from_str(yaml);
    assert!(result.is_err());
}

#[test]
fn test_service_account_deserialization() {
    // Test ServiceAccount configuration
    let yaml = r#"
    name: prometheus
    kind: Prometheus
    image: prom/prometheus:v2.7.1
    service_account:
      create: true
      cluster_role: true
      annotations:
        eks.amazonaws.com/role-arn: "arn:aws:iam::123456789012:role/prometheus-role"
    "#;

    let config: KamutConfig = serde_yaml::from_str(yaml).unwrap();
    
    assert_eq!(config.name, "prometheus");
    assert_eq!(config.kind, Some("Prometheus".to_string()));
    
    // Check service account
    let service_account = config.service_account.unwrap();
    assert_eq!(service_account.create, true);
    assert_eq!(service_account.cluster_role, Some(true));
    
    // Check annotations
    let annotations = service_account.annotations.unwrap();
    assert_eq!(
        annotations.get("eks.amazonaws.com/role-arn").unwrap(),
        "arn:aws:iam::123456789012:role/prometheus-role"
    );
}
