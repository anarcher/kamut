# Kamut

Kamut is a tool for generating Kubernetes manifests from simplified configuration files. It takes `.kamut.yaml` files as input and generates Kubernetes-compatible YAML manifests. See the [kamut.yaml specification](spec.md) for detailed documentation.

## Installation

### From GitHub Releases

Download the latest release for your platform from the [GitHub Releases page](https://github.com/anarcher/kamut/releases).

### From Source

```bash
git clone https://github.com/anarcher/kamut.git
cd kamut
cargo build --release
```

The binary will be available at `target/release/kamut`.

## Usage

```bash
# Generate manifests from all *.kamut.yaml files in the current directory
kamut

# Generate manifests from files matching a specific pattern
kamut "examples/*.kamut.yaml"

# Using the explicit generate command
kamut generate "examples/*.kamut.yaml"

# Display version information
kamut version
```

## Example

Input file (`deploy.kamut.yaml`):

```yaml
kind: Deployment
name: my-app
image: nginx:latest
replicas: 3
env:
  APP_ENV: production
  DEBUG: "false"
resources:
  requests:
    cpu: 100m
    memory: 128Mi
  limits:
    cpu: 200m
    memory: 256Mi
```

Output file (`deploy.yaml`):

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: my-app
spec:
  replicas: 3
  selector:
    matchLabels:
      app: my-app
  template:
    metadata:
      labels:
        app: my-app
    spec:
      containers:
      - name: my-app
        image: nginx:latest
        env:
        - name: APP_ENV
          value: production
        - name: DEBUG
          value: "false"
        resources:
          requests:
            cpu: 100m
            memory: 128Mi
          limits:
            cpu: 200m
            memory: 256Mi
```

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

## License

MIT
