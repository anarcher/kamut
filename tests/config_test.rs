use kamut::config::{
    find_config_files, generate_deployment_manifest, generate_prometheus_ingress,
    generate_prometheus_manifest, process_file,
};
use kamut::models::{Ingress, KamutConfig, Resources, ResourceSpec, Storage};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_find_config_files() {
    // Create a temporary directory
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();

    // Create some test files
    let file1_path = temp_path.join("test1.kamut.yaml");
    let file2_path = temp_path.join("test2.kamut.yaml");
    let file3_path = temp_path.join("test3.yaml"); // Not a kamut file

    File::create(&file1_path).unwrap();
    File::create(&file2_path).unwrap();
    File::create(&file3_path).unwrap();

    // Test finding kamut files
    let pattern = format!("{}/*.kamut.yaml", temp_path.display());
    let files = find_config_files(&pattern).unwrap();

    assert_eq!(files.len(), 2);
    assert!(files.iter().any(|f| f == &file1_path));
    assert!(files.iter().any(|f| f == &file2_path));
    assert!(!files.iter().any(|f| f == &file3_path));
}

#[test]
fn test_generate_deployment_manifest() {
    // Create a test KamutConfig for a Deployment
    let mut env = HashMap::new();
    env.insert("KEY1".to_string(), "VALUE1".to_string());
    env.insert("KEY2".to_string(), "VALUE2".to_string());

    let requests = ResourceSpec {
        cpu: Some("100m".to_string()),
        memory: Some("100Mi".to_string()),
    };

    let limits = ResourceSpec {
        cpu: Some("200m".to_string()),
        memory: Some("200Mi".to_string()),
    };

    let resources = Resources {
        requests: Some(requests),
        limits: Some(limits),
    };

    let mut node_selector = HashMap::new();
    node_selector.insert("group".to_string(), "frontend".to_string());

    let config = KamutConfig {
        name: "test-deployment".to_string(),
        kind: Some("Deployment".to_string()),
        namespace: Some("default".to_string()),
        image: Some("test-image:v1.0.0".to_string()),
        env: Some(env),
        resources: Some(resources),
        replicas: Some(3),
        retention: None,
        ingress: None,
        storage: None,
        node_selector: Some(node_selector),
        service_account: None,
    };

    // Generate the manifest
    let manifest = generate_deployment_manifest(&config).unwrap();

    // Basic validation of the manifest
    assert!(manifest.contains("name: test-deployment"));
    assert!(manifest.contains("image: test-image:v1.0.0"));
    assert!(manifest.contains("replicas: 3"));
    assert!(manifest.contains("cpu: 100m"));
    assert!(manifest.contains("memory: 100Mi"));
    assert!(manifest.contains("cpu: 200m"));
    assert!(manifest.contains("memory: 200Mi"));
    assert!(manifest.contains("name: KEY1"));
    assert!(manifest.contains("value: VALUE1"));
    assert!(manifest.contains("name: KEY2"));
    assert!(manifest.contains("value: VALUE2"));
    assert!(manifest.contains("group: frontend"));
}

#[test]
fn test_generate_prometheus_manifest() {
    // Create a test KamutConfig for Prometheus
    let requests = ResourceSpec {
        cpu: Some("500m".to_string()),
        memory: Some("500Mi".to_string()),
    };

    let limits = ResourceSpec {
        cpu: Some("1000m".to_string()),
        memory: Some("1Gi".to_string()),
    };

    let resources = Resources {
        requests: Some(requests),
        limits: Some(limits),
    };

    let storage = Storage {
        size: "100Gi".to_string(),
        class_name: "standard".to_string(),
    };

    let mut node_selector = HashMap::new();
    node_selector.insert("group".to_string(), "monitoring".to_string());

    let config = KamutConfig {
        name: "test-prometheus".to_string(),
        kind: Some("Prometheus".to_string()),
        namespace: Some("monitoring".to_string()),
        image: Some("prom/prometheus:v2.7.1".to_string()),
        env: None,
        resources: Some(resources),
        replicas: Some(1),
        retention: Some("30d".to_string()),
        ingress: None,
        storage: Some(storage),
        node_selector: Some(node_selector),
        service_account: None,
    };

    // Generate the manifest
    let manifest = generate_prometheus_manifest(&config).unwrap();

    // Basic validation of the manifest
    assert!(manifest.contains("name: test-prometheus"));
    assert!(manifest.contains("image: prom/prometheus:v2.7.1"));
    assert!(manifest.contains("replicas: 1"));
    assert!(manifest.contains("retention: 30d"));
    assert!(manifest.contains("cpu: 500m"));
    assert!(manifest.contains("memory: 500Mi"));
    assert!(manifest.contains("cpu: 1000m"));
    assert!(manifest.contains("memory: 1Gi"));
    assert!(manifest.contains("storage: 100Gi"));
    assert!(manifest.contains("storageClassName: standard"));
    assert!(manifest.contains("group: monitoring"));
}

#[test]
fn test_generate_prometheus_ingress() {
    // Create a test KamutConfig and Ingress for Prometheus
    let ingress_config = Ingress {
        host: "test.example.com".to_string(),
    };

    let config = KamutConfig {
        name: "test-prometheus".to_string(),
        kind: Some("Prometheus".to_string()),
        namespace: Some("monitoring".to_string()),
        image: Some("prom/prometheus:v2.7.1".to_string()),
        env: None,
        resources: None,
        replicas: None,
        retention: None,
        ingress: Some(ingress_config.clone()),
        storage: None,
        node_selector: None,
        service_account: None,
    };

    // Generate the ingress manifest
    let manifest = generate_prometheus_ingress(&config, &ingress_config).unwrap();

    // Basic validation of the manifest
    assert!(manifest.contains("name: test-prometheus-ingress"));
    assert!(manifest.contains("host: test.example.com"));
    assert!(manifest.contains("app: test-prometheus"));
    assert!(manifest.contains("path: /"));
    assert!(manifest.contains("pathType: Prefix"));
    assert!(manifest.contains("name: test-prometheus"));
    assert!(manifest.contains("number: 9090"));
}

#[test]
fn test_process_file() {
    // Create a temporary directory
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();

    // Create a test kamut file
    let kamut_file_path = temp_path.join("test.kamut.yaml");
    let mut kamut_file = File::create(&kamut_file_path).unwrap();

    // Write test content to the file
    let content = r#"name: test-app
kind: Deployment
namespace: default
image: test-image:v1.0.0
replicas: 2
---
name: test-prometheus
kind: Prometheus
namespace: monitoring
image: prom/prometheus:v2.7.1
retention: 15d
ingress:
  host: "test.example.com"
service_account:
  annotations:
    eks.amazonaws.com/role-arn: "arn:aws:iam::123456789012:role/prometheus-role"
"#;

    kamut_file.write_all(content.as_bytes()).unwrap();
    kamut_file.flush().unwrap();

    // Process the file
    process_file(&kamut_file_path).unwrap();

    // Check that the output file was created
    let output_file_path = temp_path.join("test.yaml");
    assert!(output_file_path.exists());

    // Read the output file content
    let output_content = fs::read_to_string(&output_file_path).unwrap();

    // Basic validation of the output content
    assert!(output_content.contains("name: test-app"));
    assert!(output_content.contains("kind: Deployment"));
    assert!(output_content.contains("image: test-image:v1.0.0"));
    assert!(output_content.contains("replicas: 2"));

    assert!(output_content.contains("name: test-prometheus"));
    assert!(output_content.contains("kind: Prometheus"));
    assert!(output_content.contains("image: prom/prometheus:v2.7.1"));
    assert!(output_content.contains("retention: 15d"));

    assert!(output_content.contains("name: test-prometheus-ingress"));
    assert!(output_content.contains("host: test.example.com"));
    
    // Check for ServiceAccount, ClusterRole, and ClusterRoleBinding
    assert!(output_content.contains("kind: ServiceAccount"));
    assert!(output_content.contains("name: test-prometheus-sa"));
    assert!(output_content.contains("eks.amazonaws.com/role-arn"));
    assert!(output_content.contains("arn:aws:iam::123456789012:role/prometheus-role"));
    
    assert!(output_content.contains("kind: ClusterRole"));
    assert!(output_content.contains("name: test-prometheus-role"));
    assert!(output_content.contains("nodes/proxy"));
    assert!(output_content.contains("/metrics"));
    
    assert!(output_content.contains("kind: ClusterRoleBinding"));
    assert!(output_content.contains("name: test-prometheus-role-binding"));
    assert!(output_content.contains("kind: ServiceAccount"));
    assert!(output_content.contains("name: test-prometheus-sa"));
}
