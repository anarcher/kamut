# Kamut Architecture

## Overview

Kamut is a tool for generating Kubernetes manifests from simplified configuration files. It takes `.kamut.yaml` files as input and generates Kubernetes-compatible YAML manifests.

## Components

### CLI (cli.rs)

The command-line interface for the application. It defines the available commands and arguments:

- Default behavior: Generates Kubernetes manifests from kamut files
  - `pattern`: File pattern to search for (default: "*.kamut.yaml")
- `generate`: Explicit command to generate Kubernetes manifests (optional)
  - `pattern`: File pattern to search for (default: "*.kamut.yaml")

### Config (config.rs)

Handles the processing of configuration files:

- `find_config_files`: Finds files matching a given pattern
- `process_file`: Processes a single file, generating manifests and saving them to output files
- `generate_deployment_manifest`: Generates a Kubernetes Deployment manifest
- `generate_prometheus_manifest`: Generates a Prometheus manifest

### Models (models.rs)

Defines the data structures used in the application:

- `KamutConfig`: The main configuration structure
- `DeploymentConfig`: Configuration for Kubernetes Deployments
- `PrometheusConfig`: Configuration for Prometheus
- `Resources`: Resource requirements
- `ResourceSpec`: CPU and memory specifications

## File Processing

1. The application searches for files matching the specified pattern
2. For each file, it:
   - Reads the file content
   - Splits the content into documents (separated by "---")
   - Processes each document, generating a manifest
   - Saves the last generated manifest to a file with the same base name but with a ".yaml" extension
   - For example, if the input file is "a.kamut.yaml", the output will be saved to "a.yaml"

## Output Behavior

- The application does not print the generated manifests to the console
- It only saves the manifests to output files
- It prints information about the processing steps and the location of the saved files
