use anyhow::{Context, Result, anyhow, bail};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const DEFAULT_ENDPOINT: &str = "https://backboard.railway.com/graphql/v2";

pub struct Railway {
    client: Client,
    endpoint: String,
    token: String,
}

impl Railway {
    pub fn new(token: String) -> Result<Self> {
        let endpoint = std::env::var("RLWY_GRAPHQL_ENDPOINT")
            .unwrap_or_else(|_| DEFAULT_ENDPOINT.to_string());
        let client = Client::builder()
            .user_agent(concat!("rlwy/", env!("CARGO_PKG_VERSION")))
            .build()
            .context("building HTTP client")?;
        Ok(Self { client, endpoint, token })
    }

    async fn graphql<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
        variables: Value,
    ) -> Result<T> {
        #[derive(Deserialize)]
        struct Response<T> {
            data: Option<T>,
            errors: Option<Vec<GqlError>>,
        }
        #[derive(Deserialize)]
        struct GqlError {
            message: String,
        }

        let body = json!({ "query": query, "variables": variables });
        let res = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await
            .context("sending GraphQL request")?;

        let status = res.status();
        let text = res.text().await.context("reading response body")?;
        if !status.is_success() {
            bail!("Railway API returned HTTP {status}: {text}");
        }
        let parsed: Response<T> = serde_json::from_str(&text)
            .with_context(|| format!("parsing GraphQL response: {text}"))?;
        if let Some(errs) = parsed.errors {
            let joined = errs
                .into_iter()
                .map(|e| e.message)
                .collect::<Vec<_>>()
                .join("; ");
            bail!("Railway API error: {joined}");
        }
        parsed.data.ok_or_else(|| anyhow!("empty GraphQL response"))
    }

    pub async fn me(&self) -> Result<Me> {
        #[derive(Deserialize)]
        struct Data { me: Me }
        let q = r#"query { me { id name email } }"#;
        let data: Data = self.graphql(q, json!({})).await?;
        Ok(data.me)
    }

    pub async fn projects(&self) -> Result<Vec<Project>> {
        #[derive(Deserialize)]
        struct Data { projects: Connection<Project> }
        let q = r#"
            query {
              projects {
                edges {
                  node {
                    id
                    name
                    environments {
                      edges { node { id name } }
                    }
                    services {
                      edges {
                        node {
                          id
                          name
                          deployments(first: 1) {
                            edges {
                              node {
                                id
                                status
                                createdAt
                                staticUrl
                                meta
                                environmentId
                              }
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
            }
        "#;
        let data: Data = self.graphql(q, json!({})).await?;
        Ok(data.projects.into_vec())
    }

    /// Scan accessible projects to find a service by id and return its latest
    /// deployment together with the project and env name it belongs to.
    /// Team/project tokens typically can't hit top-level
    /// `deployments(input:{serviceId})` or `service(id:)` queries, so we use the
    /// nested connection that we already know works for `ls`.
    pub async fn latest_deployment(&self, service_id: &str) -> Result<Option<DeploymentCtx>> {
        let projects = self.projects().await?;
        for p in projects {
            for svc in p.services() {
                if svc.id == service_id {
                    let Some(dep) = svc.latest_deployment().cloned() else {
                        return Ok(None);
                    };
                    let env_name = dep
                        .environment_id
                        .as_deref()
                        .and_then(|eid| p.env_name(eid))
                        .map(str::to_string);
                    return Ok(Some(DeploymentCtx {
                        deployment: dep,
                        env_name,
                        project_name: p.name.clone(),
                        service_name: svc.name.clone(),
                    }));
                }
            }
        }
        Ok(None)
    }

    pub async fn redeploy_deployment(&self, deployment_id: &str) -> Result<Deployment> {
        #[derive(Deserialize)]
        struct Data { #[serde(rename = "deploymentRedeploy")] dep: Deployment }
        let q = r#"
            mutation($id: String!) {
              deploymentRedeploy(id: $id) {
                id status createdAt staticUrl meta
              }
            }
        "#;
        let data: Data = self.graphql(q, json!({ "id": deployment_id })).await?;
        Ok(data.dep)
    }

    pub async fn deployment(&self, id: &str) -> Result<Deployment> {
        #[derive(Deserialize)]
        struct Data { deployment: Deployment }
        let q = r#"
            query($id: String!) {
              deployment(id: $id) { id status createdAt staticUrl environmentId }
            }
        "#;
        let data: Data = self.graphql(q, json!({ "id": id })).await?;
        Ok(data.deployment)
    }

    pub async fn build_logs(
        &self,
        deployment_id: &str,
        limit: i32,
        start_date: Option<&str>,
    ) -> Result<Vec<LogLine>> {
        #[derive(Deserialize)]
        struct Data { #[serde(rename = "buildLogs")] logs: Vec<LogLine> }
        let q = r#"
            query($id: String!, $limit: Int, $start: DateTime) {
              buildLogs(deploymentId: $id, limit: $limit, startDate: $start) {
                message timestamp severity
              }
            }
        "#;
        let data: Data = self
            .graphql(
                q,
                json!({ "id": deployment_id, "limit": limit, "start": start_date }),
            )
            .await?;
        Ok(data.logs)
    }

    pub async fn deployment_logs(
        &self,
        deployment_id: &str,
        limit: i32,
        start_date: Option<&str>,
    ) -> Result<Vec<LogLine>> {
        #[derive(Deserialize)]
        struct Data { #[serde(rename = "deploymentLogs")] logs: Vec<LogLine> }
        let q = r#"
            query($id: String!, $limit: Int, $start: DateTime) {
              deploymentLogs(deploymentId: $id, limit: $limit, startDate: $start) {
                message timestamp severity
              }
            }
        "#;
        let data: Data = self
            .graphql(
                q,
                json!({ "id": deployment_id, "limit": limit, "start": start_date }),
            )
            .await?;
        Ok(data.logs)
    }
}

#[derive(Debug, Deserialize)]
pub struct Connection<T> {
    pub edges: Vec<Edge<T>>,
}

#[derive(Debug, Deserialize)]
pub struct Edge<T> {
    pub node: T,
}

impl<T> Connection<T> {
    fn into_vec(self) -> Vec<T> {
        self.edges.into_iter().map(|e| e.node).collect()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Me {
    pub id: String,
    pub name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub services: Option<Connection<Service>>,
    #[serde(default)]
    pub environments: Option<Connection<Environment>>,
}

impl Project {
    pub fn services(&self) -> Vec<&Service> {
        self.services
            .as_ref()
            .map(|c| c.edges.iter().map(|e| &e.node).collect())
            .unwrap_or_default()
    }

    pub fn environments(&self) -> Vec<&Environment> {
        self.environments
            .as_ref()
            .map(|c| c.edges.iter().map(|e| &e.node).collect())
            .unwrap_or_default()
    }

    pub fn env_name(&self, id: &str) -> Option<&str> {
        self.environments()
            .into_iter()
            .find(|e| e.id == id)
            .map(|e| e.name.as_str())
    }
}

#[derive(Debug, Deserialize)]
pub struct Environment {
    pub id: String,
    pub name: String,
}

pub struct DeploymentCtx {
    pub deployment: Deployment,
    pub env_name: Option<String>,
    #[allow(dead_code)]
    pub project_name: String,
    #[allow(dead_code)]
    pub service_name: String,
}

#[derive(Debug, Deserialize)]
pub struct Service {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub deployments: Option<Connection<Deployment>>,
}

impl Service {
    pub fn latest_deployment(&self) -> Option<&Deployment> {
        self.deployments
            .as_ref()
            .and_then(|c| c.edges.first().map(|e| &e.node))
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Deployment {
    pub id: String,
    pub status: DeploymentStatus,
    #[serde(rename = "createdAt")]
    #[allow(dead_code)]
    pub created_at: Option<String>,
    #[serde(rename = "staticUrl")]
    pub static_url: Option<String>,
    #[serde(default)]
    pub meta: Option<Value>,
    #[serde(rename = "environmentId", default)]
    pub environment_id: Option<String>,
}

impl Deployment {
    pub fn commit_hash(&self) -> Option<&str> {
        self.meta.as_ref()?.get("commitHash")?.as_str()
    }

    pub fn commit_message(&self) -> Option<&str> {
        self.meta.as_ref()?.get("commitMessage")?.as_str()
    }

    pub fn commit_author(&self) -> Option<&str> {
        self.meta.as_ref()?.get("commitAuthor")?.as_str()
    }

    pub fn image(&self) -> Option<&str> {
        self.meta.as_ref()?.get("image")?.as_str()
    }
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeploymentStatus {
    Queued,
    Initializing,
    Waiting,
    Building,
    Deploying,
    Success,
    Failed,
    Crashed,
    Removed,
    Removing,
    Skipped,
    #[serde(other)]
    Unknown,
}

impl DeploymentStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Success
                | Self::Failed
                | Self::Crashed
                | Self::Removed
                | Self::Skipped
        )
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Queued => "QUEUED",
            Self::Initializing => "INITIALIZING",
            Self::Waiting => "WAITING",
            Self::Building => "BUILDING",
            Self::Deploying => "DEPLOYING",
            Self::Success => "SUCCESS",
            Self::Failed => "FAILED",
            Self::Crashed => "CRASHED",
            Self::Removed => "REMOVED",
            Self::Removing => "REMOVING",
            Self::Skipped => "SKIPPED",
            Self::Unknown => "UNKNOWN",
        }
    }

    pub fn progress_fraction(self) -> f64 {
        match self {
            Self::Queued => 0.05,
            Self::Initializing => 0.15,
            Self::Waiting => 0.20,
            Self::Building => 0.45,
            Self::Deploying => 0.80,
            Self::Success => 1.00,
            Self::Failed | Self::Crashed | Self::Removed | Self::Removing | Self::Skipped => 1.00,
            Self::Unknown => 0.0,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LogLine {
    pub message: String,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub severity: Option<String>,
}
