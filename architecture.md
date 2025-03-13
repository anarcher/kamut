# Kamut Architecture

## Overview

Kamut is a tool for generating Kubernetes manifests from simplified configuration files. It takes `.kamut.yaml` files as input and generates Kubernetes-compatible YAML manifests.

## Release Process

Kamut uses GitHub Actions with GoReleaser to automate the release process:

1. Create and push a new tag with the version number (e.g., `v0.1.0`)
2. GitHub Actions automatically triggers the release workflow
3. The workflow builds the Rust binary for multiple platforms (Linux, macOS, Windows)
4. GoReleaser packages the binaries and creates a GitHub release
5. Release artifacts are uploaded to the GitHub release page

To create a new release:
```bash
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0
```

## Components

### CLI (cli.rs)

The command-line interface for the application. It defines the available commands and arguments:

- Default behavior: Generates Kubernetes manifests from kamut files
  - `pattern`: File pattern to search for (default: "*.kamut.yaml")
- `generate`: Explicit command to generate Kubernetes manifests (optional)
  - `pattern`: File pattern to search for (default: "*.kamut.yaml")
- `version`: Display the version information of the application

### Config (config.rs)

Handles the processing of configuration files:

- `find_config_files`: Finds files matching a given pattern
- `process_file`: Processes a single file, generating manifests and saving them to output files
- `generate_deployment_manifest`: Generates a Kubernetes Deployment manifest
- `generate_prometheus_manifest`: Generates a Prometheus manifest with `serviceMonitorNamespaceSelector` set to `null`
- `generate_prometheus_ingress`: Generates a Kubernetes Ingress manifest for Prometheus

### Models (models.rs)

Defines the data structures used in the application:

- `KamutConfig`: The main configuration structure with common fields:
  - `name`: Name of the resource
  - `kind`: Type of resource (Deployment or Prometheus) - **Required field**
  - `image`: Container image to use
  - `env`: Environment variables
  - `resources`: Resource requirements
  - `replicas`: Number of replicas (used for both Deployment and Prometheus)
  - `retention`: Retention period for Prometheus (defaults to 15d)
  - `ingress`: Ingress configuration for Prometheus:
    - `host`: Hostname for the Ingress resource
- `DeploymentConfig`: Configuration for Kubernetes Deployments
- `PrometheusConfig`: Configuration for Prometheus
- `Resources`: Resource requirements
- `ResourceSpec`: CPU and memory specifications

## File Processing

1. The application searches for files matching the specified pattern
2. For each file, it:
   - Reads the file content
   - Splits the content into documents (separated by "---")
   - Processes each document:
     - Parses the YAML to KamutConfig
     - Validates that the `kind` field is specified (returns an error if missing)
     - Generates the appropriate manifest based on the specified kind
     - For Prometheus resources with ingress configuration, an additional Ingress manifest is generated
   - Saves all generated manifests to a file with the same base name but with a ".yaml" extension, separated by "---"
   - For example, if the input file is "a.kamut.yaml", the output will be saved to "a.yaml"

## Output Behavior

- The application does not print the generated manifests to the console
- It only saves the manifests to output files
- It prints information about the processing steps and the location of the saved files

## Testing

The project includes a comprehensive test suite to ensure functionality and reliability:

### Unit Tests

1. **Models Tests** (`tests/models_test.rs`):
   - Tests deserialization of YAML to KamutConfig
   - Validates handling of required and optional fields
   - Tests error handling for missing required fields

2. **Config Tests** (`tests/config_test.rs`):
   - Tests file finding functionality
   - Tests manifest generation for Deployments and Prometheus resources
   - Tests Ingress manifest generation
   - Tests file processing

3. **CLI Tests** (`tests/cli_test.rs`):
   - Tests command-line argument parsing
   - Tests default values
   - Tests subcommand handling

### Integration Tests

1. **Integration Tests** (`tests/integration_test.rs`):
   - Tests the complete workflow from finding files to generating manifests
   - Tests error handling for missing required fields
   - Tests processing of multiple documents in a single file

### Running Tests

To run the test suite:
```bash
cargo test
```

To run a specific test:
```bash
cargo test test_name
```

To run tests with output:
```bash
cargo test -- --nocapture
```
