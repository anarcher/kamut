use anyhow::{Context, Result};
use glob::glob;
use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
use k8s_openapi::api::core::v1::{
    Container, EnvVar, PodSpec, PodTemplateSpec, ResourceRequirements,
};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use kube_custom_resources_rs::monitoring_coreos_com::v1::prometheuses::{
    Prometheus, PrometheusSpec,
};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::models::KamutConfig;

pub fn find_config_files(pattern: &str) -> Result<Vec<std::path::PathBuf>> {
    let files: Vec<_> = glob(pattern)
        .context("Failed to read glob pattern")?
        .filter_map(Result::ok)
        .collect();

    Ok(files)
}

pub fn process_file(file_path: &Path) -> Result<()> {
    println!("Processing file: {}", file_path.display());

    let mut file = File::open(file_path)
        .with_context(|| format!("Failed to open file: {}", file_path.display()))?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    // Don't process combined file
    if file_path.file_name().and_then(|f| f.to_str()).unwrap_or("") == "combined-kamut.yaml" {
        println!("Skipping combined-kamut.yaml file as it's not supported");
        return Ok(());
    }

    // Handle multi-document YAML files by splitting on "---" separator
    let documents: Vec<&str> = contents.split("---").collect();
    let mut doc_count = 0;

    for doc in documents {
        // Skip empty documents
        if doc.trim().is_empty() {
            continue;
        }

        doc_count += 1;
        println!("\nProcessing document {} in {}", doc_count, file_path.display());

        // Parse the YAML to KamutConfig
        let config: KamutConfig = serde_yaml::from_str(doc)
            .with_context(|| format!("Failed to parse document {} in {}", doc_count, file_path.display()))?;

        // Process configs based on what's present in the file
        let mut processed = false;

        // If kind is explicitly specified, use that
        if config.kind.is_some() {
            match config.kind.as_ref().unwrap().as_str() {
                "Deployment" => {
                    if config.image.is_some() {
                        let manifest = generate_deployment_manifest(&config)?;
                        println!("\nKubernetes Deployment Manifest:\n{}", manifest);
                        processed = true;
                    } else {
                        println!("\nError: Deployment requires an image to be specified");
                    }
                }
                "Prometheus" => {
                    if config.image.is_some() {
                        let manifest = generate_prometheus_manifest(&config)?;
                        println!("\nKubernetes Prometheus Manifest:\n{}", manifest);
                        processed = true;
                    } else {
                        println!("\nError: Prometheus requires an image to be specified");
                    }
                }
                kind => {
                    println!("\nUnsupported kind: {}", kind);
                }
            }
        }
        
        // Auto-detect type if no kind is specified
        if !processed && config.kind.is_none() {
            // Try to infer the kind based on the fields present
            if config.replica_count.is_some() && config.image.is_some() {
                println!("Auto-detecting as Deployment");
                let manifest = generate_deployment_manifest(&config)?;
                println!("\nKubernetes Deployment Manifest:\n{}", manifest);
                processed = true;
            } else if config.replicas.is_some() && config.image.is_some() {
                println!("Auto-detecting as Prometheus");
                let manifest = generate_prometheus_manifest(&config)?;
                println!("\nKubernetes Prometheus Manifest:\n{}", manifest);
                processed = true;
            }
        }

        // If still not processed
        if !processed {
            println!("\nWarning: Could not determine resource type for document {}", doc_count);
        }
    }

    if doc_count == 0 {
        println!("No valid YAML documents found in file");
    }

    Ok(())
}

pub fn generate_deployment_manifest(config: &KamutConfig) -> Result<String> {
    // Create metadata
    let mut metadata = k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta::default();
    metadata.name = Some(config.name.clone());

    // Create labels
    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), config.name.clone());
    metadata.labels = Some(labels.clone());

    // Ensure image is available
    let image = config.image.as_ref().ok_or_else(|| {
        anyhow::anyhow!("Image is required for Deployment")
    })?;

    // Create container
    let mut container = Container {
        name: config.name.clone(),
        image: Some(image.clone()),
        ..Default::default()
    };

    // Add environment variables if available
    if let Some(env_vars) = &config.env {
        let mut env = Vec::new();
        for (key, value) in env_vars {
            env.push(EnvVar {
                name: key.clone(),
                value: Some(value.clone()),
                ..Default::default()
            });
        }
        container.env = Some(env);
    }

    // Add resource requirements if available
    if let Some(resources) = &config.resources {
        let mut resource_requirements = ResourceRequirements::default();

        // Add requests
        if let Some(requests) = &resources.requests {
            let mut request_map = BTreeMap::new();
            if let Some(cpu) = &requests.cpu {
                request_map.insert("cpu".to_string(), Quantity(cpu.clone()));
            }
            if let Some(memory) = &requests.memory {
                request_map.insert("memory".to_string(), Quantity(memory.clone()));
            }
            resource_requirements.requests = Some(request_map);
        }

        // Add limits
        if let Some(limits) = &resources.limits {
            let mut limit_map = BTreeMap::new();
            if let Some(cpu) = &limits.cpu {
                limit_map.insert("cpu".to_string(), Quantity(cpu.clone()));
            }
            if let Some(memory) = &limits.memory {
                limit_map.insert("memory".to_string(), Quantity(memory.clone()));
            }
            resource_requirements.limits = Some(limit_map);
        }

        container.resources = Some(resource_requirements);
    }

    // Create pod spec
    let pod_spec = PodSpec {
        containers: vec![container],
        ..Default::default()
    };

    // Create pod template spec
    let mut template_metadata =
        k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta::default();
    template_metadata.labels = Some(labels);

    let pod_template_spec = PodTemplateSpec {
        metadata: Some(template_metadata),
        spec: Some(pod_spec),
    };

    // Create selector
    let mut match_labels = BTreeMap::new();
    match_labels.insert("app".to_string(), config.name.clone());
    let selector = LabelSelector {
        match_labels: Some(match_labels),
        ..Default::default()
    };

    // Create deployment spec
    let deployment_spec = DeploymentSpec {
        replicas: config.replica_count,
        selector,
        template: pod_template_spec,
        ..Default::default()
    };

    // Create deployment
    let deployment = Deployment {
        metadata,
        spec: Some(deployment_spec),
        ..Default::default()
    };

    // Serialize to YAML
    let yaml =
        serde_yaml::to_string(&deployment).context("Failed to serialize deployment to YAML")?;

    Ok(yaml)
}

// Removed nested config functions

pub fn generate_prometheus_manifest(config: &KamutConfig) -> Result<String> {
    // Create metadata
    let mut metadata = k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta::default();
    metadata.name = Some(config.name.clone());

    // Create labels
    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), config.name.clone());
    metadata.labels = Some(labels.clone());

    // Create Prometheus spec
    let mut prometheus_spec = PrometheusSpec::default();

    // Set replicas
    prometheus_spec.replicas = config.replicas;

    // Set retention
    if let Some(retention) = &config.retention {
        prometheus_spec.retention = Some(retention.clone());
    }

    // Set resource requirements if available
    if let Some(resources) = &config.resources {
        // Create PrometheusResources
        let mut prometheus_resources = kube_custom_resources_rs::monitoring_coreos_com::v1::prometheuses::PrometheusResources::default();

        // Add requests
        if let Some(requests) = &resources.requests {
            let mut requests_map = BTreeMap::new();
            if let Some(cpu) = &requests.cpu {
                requests_map.insert("cpu".to_string(), IntOrString::String(cpu.clone()));
            }
            if let Some(memory) = &requests.memory {
                requests_map.insert("memory".to_string(), IntOrString::String(memory.clone()));
            }
            prometheus_resources.requests = Some(requests_map);
        }

        // Add limits
        if let Some(limits) = &resources.limits {
            let mut limits_map = BTreeMap::new();
            if let Some(cpu) = &limits.cpu {
                limits_map.insert("cpu".to_string(), IntOrString::String(cpu.clone()));
            }
            if let Some(memory) = &limits.memory {
                limits_map.insert("memory".to_string(), IntOrString::String(memory.clone()));
            }
            prometheus_resources.limits = Some(limits_map);
        }

        prometheus_spec.resources = Some(prometheus_resources);
    }

    // Ensure image is available
    let image = config.image.as_ref().ok_or_else(|| {
        anyhow::anyhow!("Image is required for Prometheus")
    })?;

    // Set image
    prometheus_spec.image = Some(image.clone());

    // Create Prometheus
    let prometheus = Prometheus {
        metadata,
        spec: prometheus_spec,
        status: None,
    };

    // Serialize to YAML
    let yaml =
        serde_yaml::to_string(&prometheus).context("Failed to serialize prometheus to YAML")?;

    Ok(yaml)
}

// Removed nested prometheus config function
