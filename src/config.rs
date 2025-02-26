use anyhow::{Context, Result};
use glob::glob;
use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
use k8s_openapi::api::core::v1::{Container, EnvVar, PodSpec, PodTemplateSpec, ResourceRequirements};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
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

    let config: KamutConfig = serde_yaml::from_str(&contents)
        .with_context(|| format!("Failed to parse YAML from: {}", file_path.display()))?;

    println!("Config: {:#?}", config);
    
    // Generate Kubernetes manifest
    if config.kind == "Deployment" {
        let manifest = generate_deployment_manifest(&config)?;
        println!("\nKubernetes Manifest:\n{}", manifest);
    } else {
        println!("\nUnsupported kind: {}", config.kind);
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
