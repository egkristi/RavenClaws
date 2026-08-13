//! # Kubernetes Operator Support (feature `k8s`)
//!
//! Programmatic lifecycle management for RavenClaws worker pods — the missing
//! "fleet" primitive that lets one RavenClaws binary provision *other* RavenClaws
//! binaries as Kubernetes pods. This is the foundation for the K8s-operator and
//! CRD-controller roadmap items.
//!
//! This module is **feature-gated** behind the `k8s` cargo feature to keep the
//! default binary small (a core RavenClaws constraint). Enable it with:
//!
//! ```text
//! cargo build --features k8s
//! ```
//!
//! ## Usage
//!
//! ```rust,no_run
//! use ravenclaws::k8s::K8sManager;
//!
//! # async fn example() -> ravenclaws::error::Result<()> {
//! let k8s = K8sManager::new("ravenclaws").await
//!     .map_err(|e| ravenclaws::error::RavenClawsError::K8s(e.to_string()))?;
//! let ready = k8s.get_ready_count().await
//!     .map_err(|e| ravenclaws::error::RavenClawsError::K8s(e.to_string()))?;
//! println!("{ready} workers ready");
//! # Ok(())
//! # }
//! ```
//!
//! This module exposes a public API consumed by library users rather than by the
//! default binary, so dead-code analysis on the binary produces false positives.
#![allow(dead_code)]

use k8s_openapi::{
    api::core::v1::{
        ConfigMapVolumeSource, Container, ContainerPort, EnvVar, EnvVarSource, HTTPGetAction, Pod,
        PodSpec, Probe, ResourceRequirements, SecretKeySelector, Volume, VolumeMount,
    },
    apimachinery::pkg::{api::resource::Quantity, apis::meta::v1::ObjectMeta},
};
use kube::{
    api::{Api, DeleteParams, ListParams, PostParams},
    Client,
};
use serde::{Deserialize, Serialize};

/// Configuration for a [`K8sManager`].
///
/// # Stability
/// This struct is `#[non_exhaustive]` — new fields may be added in minor releases.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct K8sManagerConfig {
    /// Kubernetes namespace to manage workers in
    pub namespace: String,
    /// Container image for worker pods
    pub image: String,
    /// Label key used to identify worker pods (default: "app")
    pub label_key: String,
    /// Label value used to identify worker pods (default: "ravenclaws-worker")
    pub label_value: String,
    /// Name of the ConfigMap mounted as `/etc/ravenclaws` in workers
    pub config_map_name: String,
    /// Name of the Secret holding the LiteLLM API key
    pub secret_name: String,
    /// Key within the secret holding the LiteLLM API key
    pub secret_key: String,
}

impl Default for K8sManagerConfig {
    fn default() -> Self {
        Self {
            namespace: "ravenclaws".to_string(),
            image: "ghcr.io/egkristi/ravenclaws:latest".to_string(),
            label_key: "app".to_string(),
            label_value: "ravenclaws-worker".to_string(),
            config_map_name: "ravenclaws-worker-config".to_string(),
            secret_name: "ravenclaws-secrets".to_string(),
            secret_key: "LITELLM_API_KEY".to_string(),
        }
    }
}

/// Manages RavenClaws worker pods dynamically via the Kubernetes API.
///
/// The AI orchestrator uses this to provision, scale, and terminate worker pods.
/// Tries in-cluster config first, falling back to the local kubeconfig.
#[derive(Clone)]
pub struct K8sManager {
    client: Client,
    config: K8sManagerConfig,
}

impl K8sManager {
    /// Create a new `K8sManager`. Tries in-cluster config first, then kubeconfig.
    pub async fn new(namespace: impl Into<String>) -> kube::Result<Self> {
        Self::with_config(K8sManagerConfig {
            namespace: namespace.into(),
            ..K8sManagerConfig::default()
        })
        .await
    }

    /// Create a new `K8sManager` with full configuration.
    pub async fn with_config(config: K8sManagerConfig) -> kube::Result<Self> {
        let client = Client::try_default().await?;
        Ok(Self { client, config })
    }

    /// The namespace this manager operates in.
    pub fn namespace(&self) -> &str {
        &self.config.namespace
    }

    /// The worker selector label (`key=value`).
    fn selector(&self) -> String {
        format!("{}={}", self.config.label_key, self.config.label_value)
    }

    // ── Pod Discovery ─────────────────────────────────────────

    /// List all worker pods with role labels.
    pub async fn list_worker_pods(&self) -> kube::Result<Vec<Pod>> {
        let api: Api<Pod> = Api::namespaced(self.client.clone(), self.namespace());
        let lp = ListParams::default().labels(&self.selector());
        let pods = api.list(&lp).await?;
        Ok(pods.items)
    }

    /// Returns the count of ready worker pods.
    pub async fn get_ready_count(&self) -> kube::Result<usize> {
        let pods = self.list_worker_pods().await?;
        Ok(pods.iter().filter(|p| is_pod_ready(p)).count())
    }

    // ── Pod Creation ─────────────────────────────────────────

    /// Create a single RavenClaws worker pod with a specific role.
    pub async fn create_worker_pod(&self, name: &str, role: &str) -> kube::Result<Pod> {
        let api: Api<Pod> = Api::namespaced(self.client.clone(), self.namespace());

        let pod = Pod {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                labels: Some(
                    [
                        (
                            self.config.label_key.clone(),
                            self.config.label_value.clone(),
                        ),
                        ("role".to_string(), role.to_string()),
                    ]
                    .into(),
                ),
                ..Default::default()
            },
            spec: Some(PodSpec {
                containers: vec![Container {
                    name: "ravenclaws".to_string(),
                    image: Some(self.config.image.clone()),
                    command: Some(vec!["/app/ravenclaws".to_string()]),
                    args: Some(vec![
                        "--config".to_string(),
                        "/etc/ravenclaws/ravenclaws.toml".to_string(),
                        "--serve".to_string(),
                    ]),
                    ports: Some(vec![ContainerPort {
                        container_port: 8080,
                        ..Default::default()
                    }]),
                    env: Some(vec![
                        EnvVar {
                            name: "LITELLM_API_KEY".to_string(),
                            value_from: Some(EnvVarSource {
                                secret_key_ref: Some(SecretKeySelector {
                                    name: Some(self.config.secret_name.clone()),
                                    key: self.config.secret_key.clone(),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                        EnvVar {
                            name: "RAVENCLAWS_ROLE".to_string(),
                            value: Some(role.to_string()),
                            ..Default::default()
                        },
                        EnvVar {
                            name: "RAVENCLAW_OTEL_DISABLED".to_string(),
                            value: Some("true".to_string()),
                            ..Default::default()
                        },
                        EnvVar {
                            name: "RUST_LOG".to_string(),
                            value: Some(
                                std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
                            ),
                            ..Default::default()
                        },
                    ]),
                    volume_mounts: Some(vec![VolumeMount {
                        name: "config".to_string(),
                        mount_path: "/etc/ravenclaws".to_string(),
                        read_only: Some(true),
                        ..Default::default()
                    }]),
                    resources: Some(ResourceRequirements {
                        requests: Some(
                            [
                                ("cpu".to_string(), Quantity("100m".to_string())),
                                ("memory".to_string(), Quantity("128Mi".to_string())),
                            ]
                            .into_iter()
                            .collect(),
                        ),
                        limits: Some(
                            [
                                ("cpu".to_string(), Quantity("500m".to_string())),
                                ("memory".to_string(), Quantity("256Mi".to_string())),
                            ]
                            .into_iter()
                            .collect(),
                        ),
                        ..Default::default()
                    }),
                    liveness_probe: Some(Probe {
                        http_get: Some(HTTPGetAction {
                            path: Some("/health".to_string()),
                            port: k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int(
                                8080,
                            ),
                            ..Default::default()
                        }),
                        initial_delay_seconds: Some(10),
                        period_seconds: Some(15),
                        ..Default::default()
                    }),
                    readiness_probe: Some(Probe {
                        http_get: Some(HTTPGetAction {
                            path: Some("/ready".to_string()),
                            port: k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int(
                                8080,
                            ),
                            ..Default::default()
                        }),
                        initial_delay_seconds: Some(5),
                        period_seconds: Some(15),
                        timeout_seconds: Some(5),
                        failure_threshold: Some(3),
                        ..Default::default()
                    }),
                    ..Default::default()
                }],
                volumes: Some(vec![Volume {
                    name: "config".to_string(),
                    config_map: Some(ConfigMapVolumeSource {
                        name: Some(self.config.config_map_name.clone()),
                        ..Default::default()
                    }),
                    ..Default::default()
                }]),
                restart_policy: Some("Always".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let pod = api.create(&PostParams::default(), &pod).await?;
        Ok(pod)
    }

    // ── Pod Deletion ─────────────────────────────────────────

    /// Delete a worker pod by name.
    pub async fn delete_worker_pod(&self, name: &str) -> kube::Result<()> {
        let api: Api<Pod> = Api::namespaced(self.client.clone(), self.namespace());
        api.delete(name, &DeleteParams::default()).await?;
        Ok(())
    }

    /// Delete all pods of a given role, optionally keeping a minimum.
    /// Returns the number of pods deleted.
    pub async fn delete_workers_by_role(&self, role: &str, keep: usize) -> kube::Result<usize> {
        let pods = self.list_worker_pods().await?;
        let role_pods: Vec<_> = pods.iter().filter(|p| pod_role(p) == Some(role)).collect();
        let to_delete = role_pods.len().saturating_sub(keep);
        let mut deleted = 0;
        for pod in role_pods.iter().take(to_delete) {
            if let Some(name) = &pod.metadata.name {
                self.delete_worker_pod(name).await?;
                deleted += 1;
            }
        }
        Ok(deleted)
    }

    // ── Service Management ───────────────────────────────────

    /// Ensure the worker ClusterIP service exists.
    pub async fn ensure_worker_service(&self) -> kube::Result<()> {
        use k8s_openapi::api::core::v1::{Service, ServicePort, ServiceSpec};
        let api: Api<Service> = Api::namespaced(self.client.clone(), self.namespace());
        let svc_name = format!("{}-workers", self.config.label_value);

        if api.get_opt(&svc_name).await?.is_some() {
            return Ok(());
        }

        let service = Service {
            metadata: ObjectMeta {
                name: Some(svc_name.clone()),
                labels: Some(
                    [(
                        self.config.label_key.clone(),
                        self.config.label_value.clone(),
                    )]
                    .into(),
                ),
                ..Default::default()
            },
            spec: Some(ServiceSpec {
                selector: Some(
                    [(
                        self.config.label_key.clone(),
                        self.config.label_value.clone(),
                    )]
                    .into(),
                ),
                ports: Some(vec![ServicePort {
                    port: 80,
                    target_port: Some(
                        k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int(8080),
                    ),
                    protocol: Some("TCP".to_string()),
                    name: Some("http".to_string()),
                    ..Default::default()
                }]),
                type_: Some("ClusterIP".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        api.create(&PostParams::default(), &service).await?;
        Ok(())
    }

    // ── Stats ────────────────────────────────────────────────

    /// Return aggregated stats about the worker fleet.
    pub async fn get_stats(&self) -> kube::Result<serde_json::Value> {
        let pods = self.list_worker_pods().await?;
        let mut roles: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut ready = 0usize;

        for pod in &pods {
            let role = pod_role(pod).unwrap_or("unknown").to_string();
            *roles.entry(role).or_default() += 1;
            if is_pod_ready(pod) {
                ready += 1;
            }
        }

        Ok(serde_json::json!({
            "total_pods": pods.len(),
            "ready_pods": ready,
            "roles": roles,
            "namespace": self.namespace(),
        }))
    }
}

/// Return the `role` label of a pod, if present.
fn pod_role(pod: &Pod) -> Option<&str> {
    pod.metadata
        .labels
        .as_ref()
        .and_then(|l| l.get("role"))
        .map(String::as_str)
}

/// Whether a pod is in the `Running` phase with a `Ready=True` condition.
fn is_pod_ready(pod: &Pod) -> bool {
    pod.status.as_ref().is_some_and(|s| {
        s.phase.as_deref() == Some("Running")
            && s.conditions.as_ref().is_some_and(|conds| {
                conds
                    .iter()
                    .any(|c| c.type_ == "Ready" && c.status == "True")
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use k8s_openapi::api::core::v1::PodCondition;
    use k8s_openapi::api::core::v1::PodStatus;

    fn ready_pod() -> Pod {
        Pod {
            metadata: ObjectMeta {
                name: Some("worker-1".to_string()),
                labels: Some([("role".to_string(), "general".to_string())].into()),
                ..Default::default()
            },
            status: Some(PodStatus {
                phase: Some("Running".to_string()),
                conditions: Some(vec![PodCondition {
                    type_: "Ready".to_string(),
                    status: "True".to_string(),
                    ..Default::default()
                }]),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn pending_pod() -> Pod {
        Pod {
            metadata: ObjectMeta {
                name: Some("worker-2".to_string()),
                labels: Some([("role".to_string(), "general".to_string())].into()),
                ..Default::default()
            },
            status: Some(PodStatus {
                phase: Some("Pending".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn test_is_pod_ready() {
        assert!(is_pod_ready(&ready_pod()));
        assert!(!is_pod_ready(&pending_pod()));
    }

    #[test]
    fn test_pod_role() {
        assert_eq!(pod_role(&ready_pod()), Some("general"));
        assert_eq!(
            pod_role(&Pod {
                metadata: ObjectMeta::default(),
                ..Default::default()
            }),
            None
        );
    }

    #[test]
    fn test_config_defaults() {
        let cfg = K8sManagerConfig::default();
        assert_eq!(cfg.namespace, "ravenclaws");
        assert_eq!(cfg.label_value, "ravenclaws-worker");
        assert!(cfg.image.contains("ravenclaws"));
    }

    #[test]
    fn test_config_selector_format() {
        let cfg = K8sManagerConfig::default();
        assert_eq!(
            format!("{}={}", cfg.label_key, cfg.label_value),
            "app=ravenclaws-worker"
        );
    }
}
