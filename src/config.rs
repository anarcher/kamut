use anyhow::{Context, Result};
use glob::glob;
use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
use k8s_openapi::api::core::v1::{Container, EnvVar, PodSpec, PodTemplateSpec, ResourceRequirements};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use kube_custom_resources_rs::monitoring_coreos_com::v1::prometheuses::{Prometheus, PrometheusSpec};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::models::{KamutConfig, DeploymentConfig, PrometheusConfig};

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

    let config: KamutConfig = serde_yaml::from_str(&contents)
        .with_context(|| format!("Failed to parse YAML from: {}", file_path.display()))?;

    println!("Config: {:#?}", config);
    
    // Generate Kubernetes manifest
    match config.kind.as_str() {
        "Deployment" => {
            let manifest = generate_deployment_manifest(&config)?;
            println!("\nKubernetes Deployment Manifest:\n{}", manifest);
        },
        "Prometheus" => {
            let manifest = generate_prometheus_manifest(&config)?;
            println!("\nKubernetes Prometheus Manifest:\n{}", manifest);
        },
        _ => {
            println!("\nUnsupported kind: {}", config.kind);
        }
    }

    // Process nested configs if present
    if let Some(deployment_config) = &config.deployment {
        println!("\nProcessing nested Deployment config");
        let manifest = generate_deployment_manifest_from_config(&config.name, deployment_config)?;
        println!("\nNested Kubernetes Deployment Manifest:\n{}", manifest);
    }

    if let Some(prometheus_config) = &config.prometheus {
        println!("\nProcessing nested Prometheus config");
        let manifest = generate_prometheus_manifest_from_config(&config.name, prometheus_config)?;
        println!("\nNested Kubernetes Prometheus Manifest:\n{}", manifest);
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
    
    // Create container
    let mut container = Container {
        name: config.name.clone(),
        image: Some(config.image.clone()),
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
    let mut template_metadata = k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta::default();
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
    let yaml = serde_yaml::to_string(&deployment)
        .context("Failed to serialize deployment to YAML")?;
    
    Ok(yaml)
}

pub fn generate_deployment_manifest_from_config(parent_name: &str, config: &DeploymentConfig) -> Result<String> {
    // Create metadata
    let mut metadata = k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta::default();
    
    // Use the nested config name if available, otherwise use parent name + "-deployment"
    let name = match &config.name {
        Some(name) => name.clone(),
        None => format!("{}-deployment", parent_name),
    };
    
    metadata.name = Some(name.clone());
    
    // Create labels
    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), name.clone());
    metadata.labels = Some(labels.clone());
    
    // Create container with the provided image or fallback to parent config
    let image = config.image.clone().unwrap_or_else(|| format!("default-image:{}", name));
    
    let mut container = Container {
        name: name.clone(),
        image: Some(image),
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
    let mut template_metadata = k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta::default();
    template_metadata.labels = Some(labels);
    
    let pod_template_spec = PodTemplateSpec {
        metadata: Some(template_metadata),
        spec: Some(pod_spec),
    };
    
    // Create selector
    let mut match_labels = BTreeMap::new();
    match_labels.insert("app".to_string(), name.clone());
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
    let yaml = serde_yaml::to_string(&deployment)
        .context("Failed to serialize deployment to YAML")?;
    
    Ok(yaml)
}

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
    
    // Set image
    prometheus_spec.image = Some(config.image.clone());
    
    // Create Prometheus
    let prometheus = Prometheus {
        metadata,
        spec: prometheus_spec,
        status: None,
    };
    
    // Serialize to YAML
    let yaml = serde_yaml::to_string(&prometheus)
        .context("Failed to serialize prometheus to YAML")?;
    
    Ok(yaml)
}

pub fn generate_prometheus_manifest_from_config(parent_name: &str, config: &PrometheusConfig) -> Result<String> {
    // Create metadata
    let mut metadata = k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta::default();
    
    // Use the nested config name if available, otherwise use parent name + "-prometheus"
    let name = match &config.name {
        Some(name) => name.clone(),
        None => format!("{}-prometheus", parent_name),
    };
    
    metadata.name = Some(name.clone());
    
    // Create labels
    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), name.clone());
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
    
    // Set image from config or use default
    let image = config.image.clone().unwrap_or_else(|| "prom/prometheus:latest".to_string());
    prometheus_spec.image = Some(image);
    
    // Create Prometheus
    let prometheus = Prometheus {
        metadata,
        spec: prometheus_spec,
        status: None,
    };
    
    // Serialize to YAML
    let yaml = serde_yaml::to_string(&prometheus)
        .context("Failed to serialize prometheus to YAML")?;
    
    Ok(yaml)
}
