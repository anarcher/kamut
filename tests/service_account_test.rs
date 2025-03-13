use kamut::config::generate_prometheus_service_account;
use kamut::models::{KamutConfig, ServiceAccount};
use std::collections::HashMap;

#[test]
fn test_generate_prometheus_service_account() {
    // Create a test ServiceAccount configuration
    let mut annotations = HashMap::new();
    annotations.insert(
        "eks.amazonaws.com/role-arn".to_string(),
        "arn:aws:iam::123456789012:role/prometheus-role".to_string(),
    );

    let service_account = ServiceAccount {
        create: true,
        annotations: Some(annotations),
        cluster_role: Some(true),
    };

    // Create a test KamutConfig
    let config = KamutConfig {
        name: "test-prometheus".to_string(),
        kind: Some("Prometheus".to_string()),
        image: Some("prom/prometheus:v2.7.1".to_string()),
        env: None,
        resources: None,
        replicas: None,
        retention: None,
        ingress: None,
        storage: None,
        node_selector: None,
        service_account: Some(service_account),
    };

    // Generate the ServiceAccount manifests
    let manifests = generate_prometheus_service_account(&config).unwrap();

    // Verify that three manifests were generated (ServiceAccount, ClusterRole, ClusterRoleBinding)
    assert_eq!(manifests.len(), 3);

    // Verify ServiceAccount manifest
    let sa_manifest = &manifests[0];
    assert!(sa_manifest.contains("kind: ServiceAccount"));
    assert!(sa_manifest.contains("name: prometheus-test-prometheus"));
    assert!(sa_manifest.contains("eks.amazonaws.com/role-arn"));
    assert!(sa_manifest.contains("arn:aws:iam::123456789012:role/prometheus-role"));
    assert!(sa_manifest.contains("automountServiceAccountToken: true"));

    // Verify ClusterRole manifest
    let cr_manifest = &manifests[1];
    assert!(cr_manifest.contains("kind: ClusterRole"));
    assert!(cr_manifest.contains("name: test-prometheus-role"));
    assert!(cr_manifest.contains("nodes/proxy"));
    assert!(cr_manifest.contains("services"));
    assert!(cr_manifest.contains("endpoints"));
    assert!(cr_manifest.contains("pods"));
    assert!(cr_manifest.contains("ingresses"));
    assert!(cr_manifest.contains("/metrics"));
    assert!(cr_manifest.contains("get"));
    assert!(cr_manifest.contains("list"));
    assert!(cr_manifest.contains("watch"));

    // Verify ClusterRoleBinding manifest
    let crb_manifest = &manifests[2];
    assert!(crb_manifest.contains("kind: ClusterRoleBinding"));
    assert!(crb_manifest.contains("name: test-prometheus-role-binding"));
    assert!(crb_manifest.contains("apiGroup: rbac.authorization.k8s.io"));
    assert!(crb_manifest.contains("kind: ClusterRole"));
    assert!(crb_manifest.contains("name: test-prometheus-role"));
    assert!(crb_manifest.contains("kind: ServiceAccount"));
    assert!(crb_manifest.contains("name: test-prometheus-sa"));
    assert!(crb_manifest.contains("namespace: default"));
}

#[test]
fn test_service_account_without_cluster_role() {
    // Create a test ServiceAccount configuration without ClusterRole
    let service_account = ServiceAccount {
        create: true,
        annotations: None,
        cluster_role: Some(false),
    };

    // Create a test KamutConfig
    let config = KamutConfig {
        name: "test-prometheus".to_string(),
        kind: Some("Prometheus".to_string()),
        image: Some("prom/prometheus:v2.7.1".to_string()),
        env: None,
        resources: None,
        replicas: None,
        retention: None,
        ingress: None,
        storage: None,
        node_selector: None,
        service_account: Some(service_account),
    };

    // Generate the ServiceAccount manifests
    let manifests = generate_prometheus_service_account(&config).unwrap();

    // Verify that only one manifest was generated (ServiceAccount)
    assert_eq!(manifests.len(), 1);

    // Verify ServiceAccount manifest
    let sa_manifest = &manifests[0];
    assert!(sa_manifest.contains("kind: ServiceAccount"));
    assert!(sa_manifest.contains("name: test-prometheus-sa"));
    assert!(sa_manifest.contains("automountServiceAccountToken: true"));
}

#[test]
fn test_service_account_not_created() {
    // Create a test ServiceAccount configuration with create=false
    let service_account = ServiceAccount {
        create: false,
        annotations: None,
        cluster_role: None,
    };

    // Create a test KamutConfig
    let config = KamutConfig {
        name: "test-prometheus".to_string(),
        kind: Some("Prometheus".to_string()),
        image: Some("prom/prometheus:v2.7.1".to_string()),
        env: None,
        resources: None,
        replicas: None,
        retention: None,
        ingress: None,
        storage: None,
        node_selector: None,
        service_account: Some(service_account),
    };

    // Generate the ServiceAccount manifests
    let manifests = generate_prometheus_service_account(&config).unwrap();

    // Verify that no manifests were generated
    assert_eq!(manifests.len(), 0);
}

#[test]
fn test_no_service_account_config() {
    // Create a test KamutConfig without ServiceAccount configuration
    let config = KamutConfig {
        name: "test-prometheus".to_string(),
        kind: Some("Prometheus".to_string()),
        image: Some("prom/prometheus:v2.7.1".to_string()),
        env: None,
        resources: None,
        replicas: None,
        retention: None,
        ingress: None,
        storage: None,
        node_selector: None,
        service_account: None,
    };

    // Generate the ServiceAccount manifests
    let manifests = generate_prometheus_service_account(&config).unwrap();

    // Verify that no manifests were generated
    assert_eq!(manifests.len(), 0);
}
