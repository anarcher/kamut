use anyhow::{Context, Result};
use glob::glob;
use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
use k8s_openapi::api::core::v1::{
    Container, EnvVar, PodSpec, PodTemplateSpec, ResourceRequirements, Service, ServiceAccount,
    ServicePort, ServiceSpec,
};
use k8s_openapi::api::networking::v1::{
    HTTPIngressPath, HTTPIngressRuleValue, Ingress, IngressBackend, IngressRule,
    IngressServiceBackend, IngressSpec, ServiceBackendPort,
};
use k8s_openapi::api::rbac::v1::{ClusterRole, ClusterRoleBinding, PolicyRule, RoleRef, Subject};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use kube_custom_resources_rs::monitoring_coreos_com::v1::prometheuses::{
    Prometheus, PrometheusResources, PrometheusSecurityContext, PrometheusSpec, PrometheusStorage,
    PrometheusStorageVolumeClaimTemplate, PrometheusStorageVolumeClaimTemplateSpec,
    PrometheusStorageVolumeClaimTemplateSpecResources, PrometheusTolerations,
};
use kube_custom_resources_rs::monitoring_coreos_com::v1alpha1::scrapeconfigs::{
    ScrapeConfig, ScrapeConfigKubernetesSdConfigs, ScrapeConfigKubernetesSdConfigsRole,
    ScrapeConfigRelabelings, ScrapeConfigRelabelingsAction, ScrapeConfigSpec,
};
use std::collections::BTreeMap;
use std::fs::{self, File};
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

    // Store the generated manifests
    let mut manifests = Vec::new();

    // Handle multi-document YAML files by splitting on "---" separator
    let documents: Vec<&str> = contents.split("---").collect();
    let mut doc_count = 0;

    for doc in documents {
        // Skip empty documents
        if doc.trim().is_empty() {
            continue;
        }

        doc_count += 1;
        println!(
            "\nProcessing document {} in {}",
            doc_count,
            file_path.display()
        );

        // Parse the YAML to KamutConfig
        let config: KamutConfig = serde_yaml::from_str(doc).with_context(|| {
            format!(
                "Failed to parse document {} in {}",
                doc_count,
                file_path.display()
            )
        })?;

        // Check if kind is specified, return error if missing
        let kind = config.kind.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "Error: 'kind' field is required in document {} of {}",
                doc_count,
                file_path.display()
            )
        })?;

        // Process configs based on what's present in the file
        let mut processed = false;

        // Process based on the specified kind
        match kind.as_str() {
            "Deployment" => {
                if config.image.is_some() {
                    let manifest = generate_deployment_manifest(&config)?;
                    manifests.push(manifest);
                    processed = true;
                } else {
                    println!("\nError: Deployment requires an image to be specified");
                }
            }
            "Prometheus" => {
                if config.image.is_some() {
                    let manifest = generate_prometheus_manifest(&config)?;
                    manifests.push(manifest);
                    println!("Generated Prometheus for Prometheus");

                    // Generate Service for Prometheus
                    let service_manifest = generate_prometheus_service(&config)?;
                    manifests.push(service_manifest);
                    println!("Generated Service for Prometheus");

                    // Generate Ingress if specified
                    if let Some(ingress_config) = &config.ingress {
                        let ingress_manifest =
                            generate_prometheus_ingress(&config, ingress_config)?;
                        manifests.push(ingress_manifest);
                        println!("Generated Ingress for Prometheus");
                    }

                    // Generate ServiceAccount, ClusterRole, and ClusterRoleBinding by default
                    // If service_account is specified, use its configuration, otherwise use defaults
                    let sa_manifests = generate_prometheus_service_account(&config)?;
                    if !sa_manifests.is_empty() {
                        manifests.extend(sa_manifests);
                        println!("Generated ServiceAccount for Prometheus");
                        println!("Generated ClusterRole and ClusterRoleBinding for Prometheus");
                    }

                    processed = true;
                } else {
                    println!("\nError: Prometheus requires an image to be specified");
                }
            }
            "KubeScrapeConfig" => {
                if let Some(_role) = &config.role {
                    let manifest = generate_scrape_config_manifest(&config)?;
                    manifests.push(manifest);
                    println!("Generated ScrapeConfig");
                    processed = true;
                } else {
                    println!("\nError: KubeScrapeConfig requires a role to be specified");
                }
            }
            kind => {
                println!("\nUnsupported kind: {}", kind);
            }
        }

        // If still not processed
        if !processed {
            println!(
                "\nWarning: Could not determine resource type for document {}",
                doc_count
            );
        }
    }

    if doc_count == 0 {
        println!("No valid YAML documents found in file");
    } else if !manifests.is_empty() {
        // Create output file name based on the input file name
        if let Some(file_name) = file_path.file_name().and_then(|f| f.to_str()) {
            // Extract the base name without the extension
            let base_name = if let Some(dot_pos) = file_name.find(".kamut.") {
                &file_name[0..dot_pos]
            } else if let Some(dot_pos) = file_name.find('.') {
                &file_name[0..dot_pos]
            } else {
                file_name // No extension, use the whole name
            };

            let base_name = if base_name.starts_with('.') {
                &base_name[1..]
            } else {
                base_name
            };

            // Create the output file name with .yaml extension
            let output_file_name = format!("{}.yaml", base_name);
            let output_path = file_path
                .parent()
                .unwrap_or(Path::new(""))
                .join(output_file_name);

            // Join all manifests with "---" separator
            let combined_manifest = manifests.join("\n---\n");

            // Write the manifest to the output file
            fs::write(&output_path, &combined_manifest)
                .with_context(|| format!("Failed to write to file: {}", output_path.display()))?;

            println!("\nSaved manifest to: {}", output_path.display());
        }
    }

    Ok(())
}

pub fn generate_prometheus_ingress(
    config: &KamutConfig,
    ingress_config: &crate::models::Ingress,
) -> Result<String> {
    // Create metadata
    let mut metadata = ObjectMeta::default();
    metadata.name = Some(format!("{}-ingress", config.name));

    // Set namespace if provided
    if let Some(namespace) = &config.namespace {
        metadata.namespace = Some(namespace.clone());
    }

    // Create labels
    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), config.name.clone());
    metadata.labels = Some(labels);

    // Create ingress rule
    let ingress_rule = IngressRule {
        host: Some(ingress_config.host.clone()),
        http: Some(HTTPIngressRuleValue {
            paths: vec![HTTPIngressPath {
                path: Some("/".to_string()),
                path_type: "Prefix".to_string(),
                backend: IngressBackend {
                    service: Some(IngressServiceBackend {
                        name: format!("prometheus-{}", config.name),
                        port: Some(ServiceBackendPort {
                            number: Some(9090),
                            name: None,
                        }),
                    }),
                    resource: None,
                },
            }],
        }),
    };

    // Create ingress spec
    let ingress_spec = IngressSpec {
        rules: Some(vec![ingress_rule]),
        ..Default::default()
    };

    // Create ingress
    let ingress = Ingress {
        metadata,
        spec: Some(ingress_spec),
        status: None,
    };

    // Serialize to YAML
    let yaml = serde_yaml::to_string(&ingress).context("Failed to serialize ingress to YAML")?;

    Ok(yaml)
}

pub fn generate_deployment_manifest(config: &KamutConfig) -> Result<String> {
    // Create metadata
    let mut metadata = ObjectMeta::default();
    metadata.name = Some(config.name.clone());

    // Set namespace if provided
    if let Some(namespace) = &config.namespace {
        metadata.namespace = Some(namespace.clone());
    }

    // Create labels
    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), config.name.clone());
    metadata.labels = Some(labels.clone());

    // Ensure image is available
    let image = config
        .image
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Image is required for Deployment"))?;

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
    let mut pod_spec = PodSpec {
        containers: vec![container],
        ..Default::default()
    };

    // Add nodeSelector if available
    if let Some(node_selector) = &config.node_selector {
        let node_selector_map = node_selector.clone().into_iter().collect();
        pod_spec.node_selector = Some(node_selector_map);
    };

    // Create pod template spec
    let mut template_metadata = ObjectMeta::default();
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
        replicas: config.replicas, // Use replicas from config
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

pub fn generate_prometheus_manifest(config: &KamutConfig) -> Result<String> {
    // Create metadata
    let mut metadata = ObjectMeta::default();
    metadata.name = Some(config.name.clone());

    // Set namespace if provided
    if let Some(namespace) = &config.namespace {
        metadata.namespace = Some(namespace.clone());
    }

    // Create labels
    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), config.name.clone());
    metadata.labels = Some(labels.clone());

    // Create Prometheus spec
    let mut prometheus_spec = PrometheusSpec::default();

    // Set replicas
    prometheus_spec.replicas = config.replicas;

    // Add podMetadata with app label
    use kube_custom_resources_rs::monitoring_coreos_com::v1::prometheuses::PrometheusPodMetadata;
    let mut pod_labels = BTreeMap::new();
    pod_labels.insert("app".to_string(), config.name.clone());
    prometheus_spec.pod_metadata = Some(PrometheusPodMetadata {
        labels: Some(pod_labels),
        annotations: None,
        name: None,
    });

    // Set retention (default to 15d if not provided)
    prometheus_spec.retention = Some(
        config
            .retention
            .clone()
            .unwrap_or_else(|| "15d".to_string()),
    );

    // Set resource requirements if available
    if let Some(resources) = &config.resources {
        // Create PrometheusResources
        let mut prometheus_resources = PrometheusResources::default();

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
    let image = config
        .image
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Image is required for Prometheus"))?;

    // Set image
    prometheus_spec.image = Some(image.clone());

    // Set security context
    prometheus_spec.security_context = Some(PrometheusSecurityContext {
        fs_group: Some(2000),
        run_as_non_root: Some(true),
        run_as_user: Some(1000),
        ..Default::default()
    });

    // Set serviceMonitor to null
    prometheus_spec.service_monitor_namespace_selector = None;
    prometheus_spec.service_monitor_selector = None;
    prometheus_spec.pod_monitor_namespace_selector = None;
    prometheus_spec.pod_monitor_selector = None;
    
    // Configure ScrapeConfig selectors to match all ScrapeConfigs in the current namespace
    // Reference: https://prometheus-operator.dev/docs/operator/api/#prometheusnamespaceselector
    prometheus_spec.scrape_config_namespace_selector = None; // Null selector matches the current namespace only
    
    // Using PrometheusScrapeConfigSelector from kube_custom_resources_rs crate
    use kube_custom_resources_rs::monitoring_coreos_com::v1::prometheuses::PrometheusScrapeConfigSelector;
    let empty_selector = PrometheusScrapeConfigSelector {
        match_labels: Some(BTreeMap::new()),
        match_expressions: None,
    };
    prometheus_spec.scrape_config_selector = Some(empty_selector); // Empty selector matches all objects

    // Set storage if available
    if let Some(storage_cfg) = &config.storage {
        let mut requests = BTreeMap::new();
        requests.insert(
            "storage".to_string(),
            IntOrString::String(storage_cfg.size.clone()),
        );

        let storage = PrometheusStorage {
            volume_claim_template: Some(PrometheusStorageVolumeClaimTemplate {
                spec: Some(PrometheusStorageVolumeClaimTemplateSpec {
                    storage_class_name: Some(storage_cfg.class_name.clone()),
                    resources: Some(PrometheusStorageVolumeClaimTemplateSpecResources {
                        requests: Some(requests.clone()),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        // Set storage spec in Prometheus spec
        prometheus_spec.storage = Some(storage);
    }

    // Set nodeSelector if available
    if let Some(node_selector) = &config.node_selector {
        let node_selector_map = node_selector.clone().into_iter().collect();
        prometheus_spec.node_selector = Some(node_selector_map);

        let tolerations = Some(
            node_selector
                .iter()
                .map(|(key, value)| PrometheusTolerations {
                    effect: Some("NoSchedule".to_string()),
                    key: Some(key.clone()),
                    operator: Some("Equal".to_string()),
                    value: Some(value.clone()),
                    ..Default::default()
                })
                .collect(),
        );

        prometheus_spec.tolerations = tolerations;
    }

    // Set serviceAccountName
    // If service_account is specified, check if it should be created, otherwise set by default
    let should_create_sa = match &config.service_account {
        Some(sa_config) => sa_config.create,
        None => true, // Create by default if not specified
    };

    if should_create_sa {
        prometheus_spec.service_account_name = Some(format!("prometheus-{}", config.name));
    }

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

// Function to generate Service for Prometheus
pub fn generate_prometheus_service(config: &KamutConfig) -> Result<String> {
    // Create metadata
    let mut metadata = ObjectMeta::default();
    metadata.name = Some(format!("prometheus-{}", config.name));

    // Set namespace if provided
    if let Some(namespace) = &config.namespace {
        metadata.namespace = Some(namespace.clone());
    }

    // Create labels
    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), config.name.clone());
    metadata.labels = Some(labels.clone());

    // Create selector
    let mut selector = BTreeMap::new();
    selector.insert("prometheus".to_string(), config.name.clone());

    // Create service port
    let service_port = ServicePort {
        name: Some("web".to_string()),
        port: 9090,
        target_port: Some(IntOrString::Int(9090)),
        protocol: Some("TCP".to_string()),
        ..Default::default()
    };

    // Create service spec
    let service_spec = ServiceSpec {
        selector: Some(selector),
        ports: Some(vec![service_port]),
        type_: Some("ClusterIP".to_string()),
        ..Default::default()
    };

    // Create service
    let service = Service {
        metadata,
        spec: Some(service_spec),
        status: None,
    };

    // Serialize to YAML
    let yaml = serde_yaml::to_string(&service).context("Failed to serialize service to YAML")?;

    Ok(yaml)
}

// Function to generate ScrapeConfig manifest using kube_custom_resources_rs type
pub fn generate_scrape_config_manifest(config: &KamutConfig) -> Result<String> {
    // Create metadata
    let mut metadata = ObjectMeta::default();
    metadata.name = Some(config.name.clone());

    // Set namespace if provided
    if let Some(namespace) = &config.namespace {
        metadata.namespace = Some(namespace.clone());
    }

    // Create labels
    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), config.name.clone());
    metadata.labels = Some(labels);

    // Create a match labels map
    let mut match_labels = std::collections::BTreeMap::new();
    if let Some(label_map) = &config.labels {
        for (key, value) in label_map {
            match_labels.insert(key.clone(), value.clone());
        }
    } else {
        // Default to app: <name> if no labels provided
        match_labels.insert("app".to_string(), config.name.clone());
    }

    // Extract role (required parameter)
    let role_str = config.role.as_deref().unwrap_or("pod");
    // Convert string to enum value
    let role = match role_str.to_lowercase().as_str() {
        "pod" => ScrapeConfigKubernetesSdConfigsRole::Pod,
        "endpoints" => ScrapeConfigKubernetesSdConfigsRole::Endpoints,
        "ingress" => ScrapeConfigKubernetesSdConfigsRole::Ingress,
        "service" => ScrapeConfigKubernetesSdConfigsRole::Service,
        "node" => ScrapeConfigKubernetesSdConfigsRole::Node,
        "endpointslice" => ScrapeConfigKubernetesSdConfigsRole::EndpointSlice,
        _ => ScrapeConfigKubernetesSdConfigsRole::Pod, // Default to Pod
    };

    // Import necessary types for namespaces configuration
    use kube_custom_resources_rs::monitoring_coreos_com::v1alpha1::scrapeconfigs::{
        ScrapeConfigKubernetesSdConfigsNamespaces,
    };

    // Handle namespace configuration
    let namespaces_config = if let Some(namespace) = &config.namespace {
        // Create a namespaces configuration that targets the specified namespace
        Some(ScrapeConfigKubernetesSdConfigsNamespaces {
            own_namespace: Some(false),
            names: Some(vec![namespace.clone()]),
        })
    } else {
        None
    };

    // Create kubernetes SD config with namespaces support
    let kubernetes_sd_config = ScrapeConfigKubernetesSdConfigs {
        role,
        api_server: None,
        attach_metadata: None,
        authorization: None,
        basic_auth: None,
        enable_http2: Some(true),
        follow_redirects: Some(true),
        namespaces: namespaces_config,
        no_proxy: None,
        oauth2: None,
        proxy_connect_header: None,
        proxy_from_environment: None,
        proxy_url: None,
        selectors: None,
        tls_config: None,
    };

    // Create relabel configs
    let keep_relabel_config = ScrapeConfigRelabelings {
        action: Some(ScrapeConfigRelabelingsAction::Keep),
        source_labels: Some(vec!["__meta_kubernetes_pod_label_app".to_string()]),
        regex: Some(config.name.clone()),
        target_label: None,
        modulus: None,
        replacement: None,
        separator: None,
    };

    let replace_relabel_config = ScrapeConfigRelabelings {
        action: Some(ScrapeConfigRelabelingsAction::Replace),
        source_labels: Some(vec!["__meta_kubernetes_pod_name".to_string()]),
        target_label: Some("pod".to_string()),
        regex: None,
        modulus: None,
        replacement: None,
        separator: None,
    };
    
    // Port relabel config based on container port number or name from config
    let port_relabel_config = if let Some(port) = &config.port {
        // Check if port is a number or name (string)
        if port.parse::<i32>().is_ok() {
            // If port is a number, use port_number
            Some(ScrapeConfigRelabelings {
                action: Some(ScrapeConfigRelabelingsAction::Keep),
                source_labels: Some(vec!["__meta_kubernetes_pod_container_port_number".to_string()]),
                separator: Some(";".to_string()),
                regex: Some(port.clone()),
                replacement: Some("$1".to_string()),
                target_label: None,
                modulus: None,
            })
        } else {
            // If port is a string, use port_name
            Some(ScrapeConfigRelabelings {
                action: Some(ScrapeConfigRelabelingsAction::Keep),
                source_labels: Some(vec!["__meta_kubernetes_pod_container_port_name".to_string()]),
                separator: Some(";".to_string()),
                regex: Some(port.clone()),
                replacement: Some("$1".to_string()),
                target_label: None,
                modulus: None,
            })
        }
    } else {
        // If no port is specified, don't add a port relabeling config
        None
    };
    
    // Drop pods with Failed or Succeeded phase
    let drop_terminated_pods_config = ScrapeConfigRelabelings {
        action: Some(ScrapeConfigRelabelingsAction::Drop),
        source_labels: Some(vec!["__meta_kubernetes_pod_phase".to_string()]),
        separator: Some(";".to_string()),
        regex: Some("(Failed|Succeeded)".to_string()),
        replacement: Some("$1".to_string()),
        target_label: None,
        modulus: None,
    };

    // Create ScrapeConfig spec
    let mut spec = ScrapeConfigSpec::default();
    spec.job_name = Some(config.name.clone());

    // 주석이 포함된 문자열을 정리합니다
    if let Some(interval) = &config.scrape_interval {
        spec.scrape_interval = Some(
            interval
                .split_whitespace()
                .next()
                .unwrap_or(interval)
                .to_string(),
        );
    }

    if let Some(timeout) = &config.scrape_timeout {
        spec.scrape_timeout = Some(
            timeout
                .split_whitespace()
                .next()
                .unwrap_or(timeout)
                .to_string(),
        );
    }

    spec.metrics_path = config.metrics_path.clone();
    spec.kubernetes_sd_configs = Some(vec![kubernetes_sd_config]);
    
    // Create a vector of relabelings, conditionally including port_relabel_config if it exists
    let mut relabelings = vec![keep_relabel_config, replace_relabel_config];
    if let Some(port_config) = port_relabel_config {
        relabelings.push(port_config);
    }
    relabelings.push(drop_terminated_pods_config);
    
    spec.relabelings = Some(relabelings);

    // Create ScrapeConfig
    let scrape_config = ScrapeConfig { metadata, spec };

    // Serialize to YAML
    let yaml = serde_yaml::to_string(&scrape_config)
        .context("Failed to serialize ScrapeConfig to YAML")?;

    Ok(yaml)
}

// Function to generate ServiceAccount, ClusterRole, and ClusterRoleBinding for Prometheus

pub fn generate_prometheus_service_account(config: &KamutConfig) -> Result<Vec<String>> {
    let mut manifests = Vec::new();

    // Determine if service account should be created
    // If service_account is specified, use its configuration, otherwise do not create
    let should_create = match &config.service_account {
        Some(sa_config) => sa_config.create,
        None => true, // create if service_account is not specified
    };

    if should_create {
        // Create ServiceAccount
        let mut sa_metadata = ObjectMeta::default();
        sa_metadata.name = Some(format!("prometheus-{}", config.name));

        // Set namespace if provided
        if let Some(namespace) = &config.namespace {
            sa_metadata.namespace = Some(namespace.clone());
        }

        // Create labels
        let mut labels = BTreeMap::new();
        labels.insert("app".to_string(), config.name.clone());
        sa_metadata.labels = Some(labels);

        // Add annotations if provided
        if let Some(sa_config) = &config.service_account {
            if let Some(annotations) = &sa_config.annotations {
                let annotations_map: BTreeMap<String, String> =
                    annotations.clone().into_iter().collect();
                sa_metadata.annotations = Some(annotations_map);
            }
        }

        // Create ServiceAccount
        let service_account = ServiceAccount {
            metadata: sa_metadata,
            automount_service_account_token: Some(true),
            ..Default::default()
        };

        // Serialize to YAML
        let sa_yaml = serde_yaml::to_string(&service_account)
            .context("Failed to serialize ServiceAccount to YAML")?;
        manifests.push(sa_yaml);

        // Determine if ClusterRole should be created
        // If service_account is specified, use its cluster_role configuration, otherwise create by default
        let should_create_cluster_role = match &config.service_account {
            Some(sa_config) => sa_config.cluster_role.unwrap_or(true),
            None => true, // Create by default if not specified
        };

        if should_create_cluster_role {
            // Create ClusterRole
            let mut cr_metadata = ObjectMeta::default();
            cr_metadata.name = Some(format!("{}-role", config.name));

            // Create labels for ClusterRole
            let mut cr_labels = BTreeMap::new();
            cr_labels.insert("app".to_string(), config.name.clone());
            cr_metadata.labels = Some(cr_labels);

            // Define rules for Prometheus
            let rules = vec![
                PolicyRule {
                    api_groups: Some(vec!["".to_string()]),
                    resources: Some(vec![
                        "nodes".to_string(),
                        "nodes/proxy".to_string(),
                        "services".to_string(),
                        "endpoints".to_string(),
                        "pods".to_string(),
                    ]),
                    verbs: vec!["get".to_string(), "list".to_string(), "watch".to_string()],
                    ..Default::default()
                },
                PolicyRule {
                    api_groups: Some(vec!["extensions".to_string()]),
                    resources: Some(vec!["ingresses".to_string()]),
                    verbs: vec!["get".to_string(), "list".to_string(), "watch".to_string()],
                    ..Default::default()
                },
                PolicyRule {
                    api_groups: Some(vec!["networking.k8s.io".to_string()]),
                    resources: Some(vec!["ingresses".to_string()]),
                    verbs: vec!["get".to_string(), "list".to_string(), "watch".to_string()],
                    ..Default::default()
                },
                PolicyRule {
                    non_resource_urls: Some(vec!["/metrics".to_string()]),
                    verbs: vec!["get".to_string()],
                    ..Default::default()
                },
            ];

            // Create ClusterRole
            let cluster_role = ClusterRole {
                metadata: cr_metadata,
                rules: Some(rules),
                ..Default::default()
            };

            // Serialize to YAML
            let cr_yaml = serde_yaml::to_string(&cluster_role)
                .context("Failed to serialize ClusterRole to YAML")?;
            manifests.push(cr_yaml);

            // Create ClusterRoleBinding
            let mut crb_metadata = ObjectMeta::default();
            crb_metadata.name = Some(format!("{}-role-binding", config.name));

            // Create labels for ClusterRoleBinding
            let mut crb_labels = BTreeMap::new();
            crb_labels.insert("app".to_string(), config.name.clone());
            crb_metadata.labels = Some(crb_labels);

            // Create RoleRef
            let role_ref = RoleRef {
                api_group: "rbac.authorization.k8s.io".to_string(),
                kind: "ClusterRole".to_string(),
                name: format!("{}-role", config.name),
            };

            // Create Subject
            let subject = Subject {
                kind: "ServiceAccount".to_string(),
                name: format!("prometheus-{}", config.name),
                namespace: config.namespace.clone(), // Use the namespace from config if provided
                ..Default::default()
            };

            // Create ClusterRoleBinding
            let cluster_role_binding = ClusterRoleBinding {
                metadata: crb_metadata,
                role_ref,
                subjects: Some(vec![subject]),
            };

            // Serialize to YAML
            let crb_yaml = serde_yaml::to_string(&cluster_role_binding)
                .context("Failed to serialize ClusterRoleBinding to YAML")?;
            manifests.push(crb_yaml);
        }
    }

    Ok(manifests)
}
