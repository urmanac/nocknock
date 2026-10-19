# ConfigHub, Flux, and OCI: A Sovereign Strategy for Multi-Plane Configuration

> **A Practical Architecture for Managing Sovereign AI (Qwen3.8 & LFM2.5), Terraform, and Serverless Workloads Beyond the Kubernetes Monoculture**  
> *Published in Urmanac Sovereign Knowledge Base (`docs/kb/`)*  
> *Target Milestone: v0.4.0 (`refactor/vacate-port-8080-for-strata`)*

---

```
                       ┌──────────────────────────────────────────────┐
                       │     THE CONFIGURATION-AS-DATA ECOSYSTEM      │
                       └──────────────────────────────────────────────┘

                                   ConfigHub
                       (SaaS Control Plane & System of Record)
                      Web Console · cub CLI · Audit ChangeSets
                                      │
                         Release Attestation & Diffs
                                      │
                                      ▼
                        ┌───────────────────────────┐
                        │   OCI Artifact Registry   │
                        │  (Harbor / ECR / IronBank)│
                        │  SHA-256 Pinned & Signed  │
                        └─────────────┬─────────────┘
                                      │
            ┌─────────────────────────┼─────────────────────────┐
            │                         │                         │
            ▼                         ▼                         ▼
   ┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐
   │ Kubernetes / AI │       │ Serverless App  │       │ Infrastructure  │
   │ (Flux Engine)   │       │ (AWS Lambda)    │       │ (Terraform)     │
   ├─────────────────┤       ├─────────────────┤       ├─────────────────┤
   │ Qwen3.8 MoE     │       │ Container / Zip │       │ VPCs, IAM, S3   │
   │ LFM2.5 Edge     │       │ Function Config │       │ Managed DBs     │
   │ WASM Brain      │       │ Event Triggers  │       │ Cloud State     │
   └─────────────────┘       └─────────────────┘       └─────────────────┘
            ▲                         ▲                         ▲
            └─────────────────────────┼─────────────────────────┘
                                      │
                                  cub-scout
                        Read-Only Fleet Explorer & Map
                        "What is deployed where?"
```

---

## 1. Executive Summary: Escaping the Kubernetes Monoculture

In modern platform engineering, an observable reality exists across many software teams: **Kubernetes is not the universal center of the universe.**

In most engineering organizations, specialized Kubernetes expertise is concentrated among a relatively small group of platform and infrastructure specialists. Meanwhile, the broader organization writes and runs software on **AWS Lambda**, managed cloud services, and serverless platforms, while provisioning infrastructure using **Terraform / OpenTofu**.

When platform teams attempt to unify operations by forcing everything into Kubernetes—such as through **Crossplane**—the initiative often meets organizational resistance:
- Crossplane attempts to make Kubernetes the universal control plane for the cloud by turning AWS VPCs, S3 buckets, and Lambda functions into Custom Resource Definitions (CRDs).
- This requires application developers to learn `kubectl`, understand CRD specifications, and debug complex controller reconciliation loops for standard cloud resources.
- If a team does not already possess broad, deep Kubernetes expertise, running an in-cluster control plane to manage cloud primitives can add substantial operational friction.

The strategic question is:

> *Can we establish an authoritative, auditable system of record for "what is deployed where"—spanning sovereign edge AI models (Qwen3.8, LFM2.5), AWS Lambda serverless functions, and Terraform infrastructure—**without** making Kubernetes a mandatory runtime dependency, and **without** building a complex custom internal developer platform?*

By pairing **ConfigHub** (for configuration governance and variant promotion) with **Flux** (for cluster reconciliation) and **OCI** (as the universal transport standard), we can solve this problem cleanly.

---

## 2. What ConfigHub Actually Is (And What It Does)

To evaluate ConfigHub objectively, we must look past marketing terminology and examine its concrete software components, interfaces, and operational model.

### The Interfaces You Interact With
ConfigHub provides three primary operational interfaces:
1. **The Web Console (`hub.confighub.com`):** A graphical interface providing a unified view of your organization's components, spaces, variants, and release pipelines. It displays visual diffs between environments (e.g., comparing `staging` vs. `prod`), tracks approval gates, and maintains an immutable timeline of who changed what, when, and why.
2. **The Command-Line Tool (`cub`):** A client CLI used by operators, CI/CD pipelines, and AI coding agents. `cub` allows you to inspect configurations, run local pre-flight checks, upload bases, create environment variants, and trigger release promotions.
3. **The Diagnostic Explorer (`cub-scout`):** A read-only discovery tool that queries target environments (Kubernetes clusters, GitOps controllers) to map live resource ownership, lineage, and drift without requiring write privileges.

### What You Get Out of the Box
When you adopt ConfigHub, you are not buying an execution engine (it does not apply Kubernetes YAML or run `terraform apply` itself). You are buying a **configuration governance and proof layer**:
* **The "Configuration-as-Data" Model:** ConfigHub stores configuration as fully materialized, structured data rather than opaque templates. This eliminates the "template explosion" where nested Helm values or complex HCL variables obscure what will actually be applied.
* **Variant Chaining:** You can define a public or corporate `base` variant, and derive specific deployment variants (`dev`, `staging`, `prod`) from it. When upstream fixes or security patches are made to the base, ConfigHub flows those updates down to deployment variants while preserving local overrides.
* **Pre-Flight Diffs & Approval Gates:** Before a change reaches a target environment, ConfigHub renders the exact diff of the underlying objects and enforces approval policies (Changesets).
* **Cryptographic Receipts:** Every change, check, and promotion produces an auditable receipt with an immutable hash, linking the source configuration to the target release.

---

## 3. SaaS vs. Self-Hosted: The Architecture Decision

A common point of deliberation is whether to consume ConfigHub as a managed SaaS service or deploy ConfigHub Server on-premises.

### The SaaS Reality
For most teams, the primary goal of adopting a configuration platform is to **offload operational toil**, not create more of it. 
* In the SaaS model (`hub.confighub.com`), ConfigHub hosts the metadata database, web console, and change management workflows.
* Your local machines and CI/CD runners simply use `cub` to authenticate, validate configurations, and publish release bundles.
* You do not have to manage high-availability PostgreSQL clusters, backup regimens, control plane upgrades, or monitoring for the configuration tool itself.

### The Enterprise / Self-Hosted Path
ConfigHub offers an enterprise deployment option for organizations operating in air-gapped networks or under strict regulatory constraints where configuration metadata cannot leave private boundaries.
* However, self-hosting introduces immediate overhead: your platform team becomes responsible for maintaining the availability and integrity of the configuration database.
* **The Recommendation:** Start with the SaaS architecture to validate the workflow and reap the velocity benefits of managed infrastructure. If compliance requirements dictate on-prem hosting later, the underlying data model (Components, Units, OCI bundles) remains fully portable.

---

## 4. How ConfigHub Tracks Repositories, Lambdas, and Terraform

ConfigHub's core architectural strength is that it does not require a Kubernetes runtime to track software assets. It uses four core primitives: **Components**, **Variants (Spaces)**, **Units**, and **Targets**.

```
┌────────────────────────────────────────────────────────────────────────┐
│             CONFIGHUB MULTI-PLANE OBJECT MAPPING MODEL                 │
└────────────────────────────────────────────────────────────────────────┘

  PLANE               COMPONENT            UNITS (Config Data)        TARGET
  ─────               ─────────            ───────────────────        ──────
  Sovereign AI        qwen38-moe-runner    • Runtime manifest (YAML)  Edge Cluster
                      lfm25-edge-daemon    • Model OCI digest ref     (Flux Engine)
                                           • VRAM & GPU prerequisites

  Serverless App      nocknock-ingestion   • Function config (JSON)   AWS Account
                      price-feed-webhook   • OCI image / S3 zip ref   (CI/CD Pipeline)
                                           • Timeout, memory, env

  Infrastructure      aws-vpc-network      • terraform.tfvars.json    AWS Region
                      timescale-database   • Module version pins      (Terraform Cloud/
                                           • Subnet / security rules   GitHub Actions)
```

### 1. How It Tracks What is in a Repository
When you import or manage a repository with ConfigHub, it constructs a **Source and Intent Record**:
- It records the source identity: the repository URL, branch, commit SHA, and path.
- It records the generation inputs: the exact values, overlays, or environment flags applied.
- It computes a cryptographic hash of the resulting configuration files (the object-set hash).
- When a release is published, ConfigHub packages the data into an OCI artifact (or Git commit) annotated with the source repository metadata and package SHA. This establishes an unbroken chain of custody from code commit to deployment.

### 2. Tracking AWS Lambda Serverless Workloads
AWS Lambda functions can be tracked as first-class Components without any Kubernetes involvement:
- **Component Definition:** Create a Component named `ingestion-worker-lambda`.
- **The Unit:** The Unit contains the function's operational configuration:
  - Runtime settings: memory allocation, execution timeout, architecture (`arm64`), and concurrency limits.
  - Deployment artifact reference: the immutable OCI container image digest (e.g., `ecr.aws/.../worker@sha256:...`) or the S3 bucket/key for a zip bundle.
  - Environment variables and secret references.
- **Variants:** Define `dev`, `staging`, and `production` variants with differing concurrency and environment values.
- **The Delivery Loop:** When a new release is approved in ConfigHub, your existing CI/CD runner (GitHub Actions, GitLab CI, or AWS CodePipeline) pulls the published release artifact and executes standard AWS CLI / SDK commands:
  ```bash
  aws lambda update-function-code --function-name ingestion-worker --image-uri <OCI_DIGEST>
  aws lambda update-function-configuration --function-name ingestion-worker --cli-input-json file://config.json
  ```

### 3. Tracking Terraform Infrastructure
Rather than attempting to replace Terraform with an in-cluster controller, ConfigHub acts as the **variable and module governance layer**:
- **Component Definition:** Create a Component for an infrastructure module, such as `core-vpc` or `rds-database`.
- **The Unit:** The Unit holds the environment-specific `terraform.tfvars.json` or declarative HCL configuration.
- **Promotion Across Environments:** You can compare the exact variable differences between `staging` and `production` in the ConfigHub UI before promoting.
- **Execution:** Your existing Terraform execution pipeline (Spacelift, Atlantis, Terraform Cloud, or CI runners) consumes the pinned `tfvars` artifact from ConfigHub and executes `terraform plan` and `terraform apply`.

---

## 5. Deploying Sovereign AI: Qwen3.8 & LFM2.5 via Flux and OCI

For NockNock's upcoming architecture, deploying local and edge foundation models introduces unique challenges:
* **Qwen-3.8-Flash-Next (125B MoE):** Requires multi-gigabyte quantized weight streaming, Metal/CUDA acceleration, HugePages, and specialized NVMe cache mounts.
* **Liquid AI LFM2.5 (Recurrent Liquid Models):** Lightweight edge models that adapt continuously, optimized for ultra-low latency sovereign reasoning.

Traditional Git repositories choke on multi-gigabyte weight files, and raw Helm charts often hide the node-level driver and memory prerequisites required to run these models.

### The Unified OCI Distribution Strategy

```
                          ┌───────────────────────────┐
                          │   OCI Artifact Registry   │
                          │ (Harbor / ECR / IronBank) │
                          └─────────────┬─────────────┘
                                        │
             ┌──────────────────────────┼──────────────────────────┐
             │                          │                          │
             ▼                          ▼                          ▼
    ┌─────────────────┐        ┌─────────────────┐        ┌─────────────────┐
    │  Model Weights  │        │   WASM Logic    │        │  Config Bundles │
    ├─────────────────┤        ├─────────────────┤        ├─────────────────┤
    │ Qwen3.8 MoE GGUF│        │ nocknock_brain  │        │ Pre-rendered    │
    │ LFM2.5 Metal bin│        │ (Deterministic  │        │ Flux Manifests  │
    │ Pinned by SHA   │        │  LP math engine)│        │ Pinned by SHA   │
    └─────────────────┘        └─────────────────┘        └─────────────────┘
             │                          │                          │
             └──────────────────────────┼──────────────────────────┘
                                        │
                                        ▼
                               ┌─────────────────┐
                               │ Flux Controller │
                               │ (Reconciliation)│
                               └─────────────────┘
```

1. **OCI as the Universal Transport:**
   - Model weights (GGUF, Metal streaming arenas) are packaged into OCI registries. OCI registries are natively designed for content-addressable storage and multi-gigabyte blob chunking.
   - The compiled deterministic WASM brain (`nocknock_brain.wasm`) is published as an OCI artifact.
   - The Kubernetes deployment manifests are packaged as ConfigHub OCI bundles.
2. **Pre-Flight Hardware Verification:**
   - Using ConfigHub Stacks, hardware prerequisites (NVIDIA GPU Operator, Metal device plugins, NVMe cache paths) are verified *before* an AI deployment is promoted.
3. **Flux Zero-Drift Reconciliation:**
   - Flux's `OCIRepository` reconciles the manifests against target clusters. When ConfigHub promotes a new model variant, Flux detects the new SHA-256 digest and performs a clean rollout.

---

## 6. Public Credibility: Presenting Technical Truth

When discussing these architectural patterns publicly—in open-source documentation, technical blog posts, or community forums—credibility depends on intellectual and factual rigor:

1. **Avoid Synthetic Statistics & Generalize Honestly:**
   - Do not cite speculative numbers or anecdotal percentages about industry-wide adoption. Instead, describe observable structural dynamics: engineering teams naturally specialize, serverless workflows cater to rapid application development, and forcing all teams into a single runtime abstraction introduces friction.
2. **Acknowledge Open Source Lineage:**
   - Credit the foundational principles: GitOps originated by Weaveworks and Flux, declarative configuration concepts rooted in the Kubernetes Resource Model (KRM), and OCI packaging standards. Acknowledging established prior art builds confidence with senior engineers.
3. **Rely on Machine-Verifiable Receipts:**
   - The core innovation of ConfigHub and OCI is that claims are not based on human assertions; they are backed by **cryptographic receipts, immutable SHA-256 digests, and deterministic diffs**. Demonstrating real, reproducible code and verifiable receipts ensures your technical claims remain unassailable.

---

## 7. The Final Verdict

Adopting **ConfigHub** alongside **Flux** and **OCI** is an exceptionally sound architectural strategy:
- **It decouples configuration from the runtime:** You avoid the trap of forcing non-Kubernetes engineers to learn complex cluster controllers, allowing Terraform and AWS Lambda to be governed in the same system of record as your Kubernetes workloads.
- **It establishes OCI as the single distribution standard:** Using OCI for container images, WASM binaries, large language model checkpoints, and configuration data creates a clean, unified supply chain.
- **It delivers immediate visibility:** Through ConfigHub’s web console and `cub-scout`’s read-only discovery, your team gains an authoritative answer to "what is deployed where" without building or maintaining an expensive internal developer platform.
