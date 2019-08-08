use k8s_openapi::api::batch::v1 as batchapi;
use k8s_openapi::api::core::v1 as api;
use k8s_openapi::apimachinery::pkg::apis::meta::v1 as meta;
use kube::client::APIClient;

use crate::schematic::component::Component;
use crate::workload_type::{InstigatorResult, KubeName, ParamMap, WorkloadType};

use std::collections::BTreeMap;

/// Task represents a non-daemon process.
///
/// It is currently implemented as a Kubernetes Job.
pub struct Task {
    pub name: String,
    pub component_name: String,
    pub instance_name: String,
    pub namespace: String,
    pub definition: Component,
    pub client: APIClient,
    pub params: ParamMap,
    pub owner_ref: Option<Vec<meta::OwnerReference>>,
}
impl Task {
    /// Create a Job
    pub fn to_job(&self) -> batchapi::Job {
        let mut labels = BTreeMap::new();
        let podname = self.kube_name();
        labels.insert("app".to_string(), self.name.clone());
        batchapi::Job {
            // TODO: Could make this generic.
            metadata: Some(meta::ObjectMeta {
                name: Some(podname.clone()),
                labels: Some(labels.clone()),
                owner_references: self.owner_ref.clone(),
                ..Default::default()
            }),
            spec: Some(batchapi::JobSpec {
                backoff_limit: Some(4),
                template: api::PodTemplateSpec {
                    metadata: Some(meta::ObjectMeta {
                        name: Some(podname),
                        labels: Some(labels),
                        owner_references: self.owner_ref.clone(),
                        ..Default::default()
                    }),
                    spec: Some(self.definition.to_pod_spec_with_policy("Never".into())),
                },
                ..Default::default()
            }),
            ..Default::default()
        }
    }
}

impl KubeName for Task {
    fn kube_name(&self) -> String {
        self.instance_name.to_string()
    }
}
impl WorkloadType for Task {
    fn add(&self) -> InstigatorResult {
        let job = self.to_job();
        let pp = kube::api::PostParams::default();

        // Right now, the Batch API is not transparent through Kube.
        // TODO: Commit upstream
        let batch = kube::api::RawApi {
            group: "batch".into(),
            resource: "jobs".into(),
            prefix: "apis".into(),
            namespace: Some(self.namespace.to_string()),
            version: "v1".into(),
        };

        let req = batch.create(&pp, serde_json::to_vec(&job)?)?;
        self.client.request::<batchapi::Job>(req)?;
        Ok(())
    }
}
