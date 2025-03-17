use kamut::config::{find_config_files, process_file};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tempfile::tempdir;

// This is an integration test that simulates the main function's behavior
#[test]
fn test_generate_manifests_workflow() {
    // Create a temporary directory
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();

    // Create test kamut files
    create_test_kamut_files(temp_path);

    // Find kamut files
    let pattern = format!("{}/*.kamut.yaml", temp_path.display());
    let files = find_config_files(&pattern).unwrap();

    // Verify files were found
    assert_eq!(files.len(), 2);

    // Process each file
    for file_path in &files {
        process_file(file_path).unwrap();
    }

    // Verify output files were created
    let deployment_output = temp_path.join("deployment.yaml");
    let prometheus_output = temp_path.join("prometheus.yaml");

    assert!(deployment_output.exists());
    assert!(prometheus_output.exists());

    // Verify deployment output content
    let deployment_content = fs::read_to_string(&deployment_output).unwrap();
    assert!(deployment_content.contains("kind: Deployment"));
    assert!(deployment_content.contains("name: test-deployment"));
    assert!(deployment_content.contains("image: test-image:v1.0.0"));
    assert!(deployment_content.contains("replicas: 2"));

    // Verify prometheus output content
    let prometheus_content = fs::read_to_string(&prometheus_output).unwrap();
    assert!(prometheus_content.contains("kind: Prometheus"));
    assert!(prometheus_content.contains("name: test-prometheus"));
    assert!(prometheus_content.contains("image: prom/prometheus:v2.7.1"));
    assert!(prometheus_content.contains("retention: 15d"));
    assert!(prometheus_content.contains("kind: Ingress"));
    assert!(prometheus_content.contains("host: test.example.com"));
}

// Helper function to create test kamut files
fn create_test_kamut_files(dir: &Path) {
    // Create deployment kamut file
    let deployment_path = dir.join("deployment.kamut.yaml");
    let mut deployment_file = File::create(&deployment_path).unwrap();
    let deployment_content = r#"name: test-deployment
kind: Deployment
image: test-image:v1.0.0
replicas: 2
env:
  KEY1: VALUE1
  KEY2: VALUE2
resources:
  requests:
    cpu: 100m
    memory: 100Mi
  limits:
    cpu: 200m
    memory: 200Mi
"#;
    deployment_file.write_all(deployment_content.as_bytes()).unwrap();
    deployment_file.flush().unwrap();

    // Create prometheus kamut file
    let prometheus_path = dir.join("prometheus.kamut.yaml");
    let mut prometheus_file = File::create(&prometheus_path).unwrap();
    let prometheus_content = r#"name: test-prometheus
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
  size: 100Gi
  className: standard
ingress:
  host: "test.example.com"
"#;
    prometheus_file.write_all(prometheus_content.as_bytes()).unwrap();
    prometheus_file.flush().unwrap();
}

// Test error handling for missing kind field
#[test]
fn test_missing_kind_field() {
    // Create a temporary directory
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();

    // Create a kamut file with missing kind field
    let file_path = temp_path.join("missing-kind.kamut.yaml");
    let mut file = File::create(&file_path).unwrap();
    let content = r#"name: test-app
image: test-image:v1.0.0
replicas: 2
"#;
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();

    // Process the file and expect an error
    let result = process_file(&file_path);
    assert!(result.is_err());
    let error = result.unwrap_err().to_string();
    assert!(error.contains("'kind' field is required"));
}

// Test error handling for missing image field
#[test]
fn test_missing_image_field() {
    // Create a temporary directory
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();

    // Create a kamut file with missing image field
    let file_path = temp_path.join("missing-image.kamut.yaml");
    let mut file = File::create(&file_path).unwrap();
    let content = r#"name: test-app
kind: Deployment
replicas: 2
"#;
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();

    // Process the file
    let result = process_file(&file_path);
    
    // The process_file function should not return an error for missing image,
    // but it should print a warning and not generate a manifest
    assert!(result.is_ok());
    
    // Check that no output file was created
    let output_path = temp_path.join("missing-image.yaml");
    assert!(!output_path.exists());
}

// Test handling of multiple documents in a single file
#[test]
fn test_multiple_documents() {
    // Create a temporary directory
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();

    // Create a kamut file with multiple documents
    let file_path = temp_path.join("multi-doc.kamut.yaml");
    let mut file = File::create(&file_path).unwrap();
    let content = r#"name: app1
kind: Deployment
image: app1:v1.0.0
replicas: 2
---
name: app2
kind: Deployment
image: app2:v1.0.0
replicas: 3
---
name: monitoring
kind: Prometheus
image: prom/prometheus:v2.7.1
retention: 15d
"#;
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();

    // Process the file
    process_file(&file_path).unwrap();

    // Check that the output file was created
    let output_path = temp_path.join("multi-doc.yaml");
    assert!(output_path.exists());

    // Read the output file content
    let output_content = fs::read_to_string(&output_path).unwrap();

    // Verify that all documents were processed
    assert!(output_content.contains("name: app1"));
    assert!(output_content.contains("replicas: 2"));
    assert!(output_content.contains("name: app2"));
    assert!(output_content.contains("replicas: 3"));
    assert!(output_content.contains("name: monitoring"));
    assert!(output_content.contains("retention: 15d"));

    // Count the number of documents in the output file
    // Expected: 3 original definitions + 1 Service = 4 documents
    // Note: ServiceAccount, ClusterRole, and ClusterRoleBinding are not created by default anymore
    let doc_count = output_content.matches("---").count() + 1;
    assert_eq!(doc_count, 4);
}
