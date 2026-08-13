//! Deckwatch plugin — provisions a Redis Cache sidecar via the Infrastructure tab.
//!
//! ## WASM entry points
//!
//! - [`metadata`] — declares the "cache" provisionable resource; called once at load time.
//! - [`provision`] — injects a Redis sidecar and `REDIS_URL` env var when an operator
//!   adds the resource on the Application Infrastructure tab.
//! - [`deprovision`] — called when the resource is removed; returns a message explaining
//!   the reconcile behaviour.
//!
//! ## Architecture
//!
//! Association is managed at the application level via the Infrastructure tab —
//! no deployment annotation required. Deckwatch calls `provision()` to persist state
//! and sidecars, then injects them into all application deployments on the next
//! reconcile cycle. Removing the resource causes the sidecar to disappear on the
//! next reconcile via server-side apply.

#[cfg(target_arch = "wasm32")]
use deckwatch_plugin_sdk::PluginMetadata;
use deckwatch_plugin_sdk::{
    ConfigField, ConfigFieldType, PluginResource, ResourceDeprovisionRequest,
    ResourceDeprovisionResult, ResourceProvisionRequest, ResourceProvisionResult, SidecarSpec,
};
#[cfg(target_arch = "wasm32")]
use extism_pdk::*;

const DEFAULT_IMAGE: &str = "redis:7-alpine";
const DEFAULT_PORT: i32 = 6379;
const DEFAULT_PORT_STR: &str = "6379";

// ── Resource declaration ───────────────────────────────────────────────────────

/// Declares the "cache" provisionable resource shown on the Infrastructure tab.
///
/// Not `#[cfg(target_arch = "wasm32")]`-gated so it can be called in host-target
/// unit tests.
pub fn resource_decl() -> PluginResource {
    PluginResource {
        id: "cache".into(),
        label: "Redis Cache".into(),
        icon: "mdi-database-outline".into(),
        description: "Provision a Redis cache sidecar for this application. Injects REDIS_URL into all deployments.".into(),
        singleton: true,
        fields: vec![
            ConfigField {
                key: "REDIS_IMAGE".into(),
                label: "Redis Image".into(),
                description: "Redis container image to inject as a sidecar.".into(),
                field_type: ConfigFieldType::String,
                default: Some(DEFAULT_IMAGE.into()),
                required: false,
                options: vec![],
                env_source: None,
            },
            ConfigField {
                key: "REDIS_PORT".into(),
                label: "Redis Port".into(),
                description: "Port the Redis sidecar listens on.".into(),
                field_type: ConfigFieldType::String,
                default: Some(DEFAULT_PORT_STR.into()),
                required: false,
                options: vec![],
                env_source: None,
            },
        ],
        output_keys: vec!["REDIS_URL".into()],
    }
}

// ── Pure core: provision ───────────────────────────────────────────────────────

/// Provision a Redis cache sidecar for an application.
///
/// Returns a [`ResourceProvisionResult`] with:
/// - `state`: `REDIS_URL` injected into all deployments
/// - `sidecars`: the `redis-sidecar` container
/// - `deployment_annotations`: stamps `deckwatch.io/redis: "provisioned"`
///
/// Testable on the native target without extism.
pub fn provision_inner(req: &ResourceProvisionRequest) -> ResourceProvisionResult {
    let mut result = ResourceProvisionResult::default();

    let image = req
        .fields
        .get("REDIS_IMAGE")
        .filter(|s| !s.is_empty())
        .cloned()
        .unwrap_or_else(|| DEFAULT_IMAGE.to_string());

    let port = req
        .fields
        .get("REDIS_PORT")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(DEFAULT_PORT);

    let redis_url = format!("redis://localhost:{port}");

    result.state.insert("REDIS_URL".into(), redis_url);

    result
        .deployment_annotations
        .insert("deckwatch.io/redis".into(), "provisioned".into());

    result.sidecars.push(SidecarSpec {
        name: "redis-sidecar".into(),
        image,
        env: vec![],
        port: Some(port),
        cpu: Some("50m".into()),
        memory: Some("64Mi".into()),
    });

    result
}

// ── Pure core: deprovision ─────────────────────────────────────────────────────

/// Deprovision — the Redis sidecar is stateless; the deckwatch record is removed
/// and the sidecar disappears from deployments on the next reconcile via SSA.
pub fn deprovision_inner(_req: &ResourceDeprovisionRequest) -> ResourceDeprovisionResult {
    ResourceDeprovisionResult {
        errors: vec![],
        message: "Redis cache sidecar will be removed from all application deployments on the next reconcile.".into(),
    }
}

// ── WASM entry points ──────────────────────────────────────────────────────────

/// Return static plugin metadata including the "cache" resource declaration.
/// Called once by deckwatch at load time.
#[cfg(target_arch = "wasm32")]
#[plugin_fn]
pub fn metadata() -> FnResult<Json<PluginMetadata>> {
    Ok(Json(PluginMetadata {
        name: "redis".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        description: "Provisions a Redis Cache sidecar via the Application Infrastructure tab"
            .into(),
        provides: vec!["sidecar:redis".into()],
        depends_on: vec![],
        optional_depends_on: vec![],
        config_schema: vec![ConfigField {
            key: "REDIS_IMAGE".into(),
            label: "Redis Image".into(),
            description: "Default Redis container image (can be overridden per-resource).".into(),
            field_type: ConfigFieldType::String,
            default: Some(DEFAULT_IMAGE.into()),
            required: false,
            options: vec![],
            env_source: None,
        }],
        resources: vec![resource_decl()],
    }))
}

/// Provision a Redis cache sidecar. Called when an operator clicks "Add Redis Cache"
/// and submits the provisioning form on the Application Infrastructure tab.
#[cfg(target_arch = "wasm32")]
#[plugin_fn]
pub fn provision(
    Json(req): Json<ResourceProvisionRequest>,
) -> FnResult<Json<ResourceProvisionResult>> {
    Ok(Json(provision_inner(&req)))
}

/// Deprovision the Redis cache resource. The sidecar is removed from all deployments
/// on the next reconcile cycle via server-side apply.
#[cfg(target_arch = "wasm32")]
#[plugin_fn]
pub fn deprovision(
    Json(req): Json<ResourceDeprovisionRequest>,
) -> FnResult<Json<ResourceDeprovisionResult>> {
    Ok(Json(deprovision_inner(&req)))
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn provision_req(fields: &[(&str, &str)]) -> ResourceProvisionRequest {
        ResourceProvisionRequest {
            application_name: "my-app".into(),
            namespace: "production".into(),
            resource_id: "cache".into(),
            fields: fields
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    fn deprovision_req() -> ResourceDeprovisionRequest {
        ResourceDeprovisionRequest {
            application_name: "my-app".into(),
            namespace: "production".into(),
            resource_id: "cache".into(),
            state: HashMap::new(),
            fields: HashMap::new(),
        }
    }

    #[test]
    fn resource_decl_has_expected_id_and_output_keys() {
        let r = resource_decl();
        assert_eq!(r.id, "cache");
        assert_eq!(r.label, "Redis Cache");
        assert!(r.singleton, "redis cache must be a singleton resource");
        assert!(
            r.output_keys.contains(&"REDIS_URL".to_string()),
            "REDIS_URL must be declared as an output key"
        );
        assert!(
            r.fields.iter().any(|f| f.key == "REDIS_IMAGE"),
            "REDIS_IMAGE field must be declared"
        );
        assert!(
            r.fields.iter().any(|f| f.key == "REDIS_PORT"),
            "REDIS_PORT field must be declared"
        );
    }

    #[test]
    fn provision_defaults_inject_correct_url_and_sidecar() {
        let result = provision_inner(&provision_req(&[]));
        assert!(
            result.errors.is_empty(),
            "no errors expected: {:?}",
            result.errors
        );
        assert_eq!(
            result.state.get("REDIS_URL").map(String::as_str),
            Some("redis://localhost:6379")
        );
        assert_eq!(result.sidecars.len(), 1);
        let sc = &result.sidecars[0];
        assert_eq!(sc.name, "redis-sidecar");
        assert_eq!(sc.image, DEFAULT_IMAGE);
        assert_eq!(sc.port, Some(DEFAULT_PORT));
    }

    #[test]
    fn provision_custom_image_is_used() {
        let result = provision_inner(&provision_req(&[("REDIS_IMAGE", "redis:8-alpine")]));
        assert_eq!(result.sidecars[0].image, "redis:8-alpine");
    }

    #[test]
    fn provision_custom_port_updates_url() {
        let result = provision_inner(&provision_req(&[("REDIS_PORT", "6380")]));
        assert_eq!(
            result.state.get("REDIS_URL").map(String::as_str),
            Some("redis://localhost:6380")
        );
        assert_eq!(result.sidecars[0].port, Some(6380));
    }

    #[test]
    fn provision_stamps_annotation() {
        let result = provision_inner(&provision_req(&[]));
        assert_eq!(
            result
                .deployment_annotations
                .get("deckwatch.io/redis")
                .map(String::as_str),
            Some("provisioned")
        );
    }

    #[test]
    fn provision_sidecar_has_resource_requests() {
        let result = provision_inner(&provision_req(&[]));
        assert!(result.sidecars[0].cpu.is_some(), "cpu request must be set");
        assert!(
            result.sidecars[0].memory.is_some(),
            "memory request must be set"
        );
    }

    #[test]
    fn deprovision_returns_non_empty_message() {
        let result = deprovision_inner(&deprovision_req());
        assert!(result.errors.is_empty());
        assert!(
            !result.message.is_empty(),
            "deprovision must return a user-visible message"
        );
    }
}
