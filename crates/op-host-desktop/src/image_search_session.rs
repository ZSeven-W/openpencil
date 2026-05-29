//! Background image-search enrichment for generated image nodes.

use std::collections::HashSet;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

use jian_ops_schema::node::PenNode;
use op_editor_core::{walkers, EditorState, NodeId, PenNodeExt as _};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImageSearchTarget {
    pub node_id: NodeId,
    pub query: String,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OpenverseCredentials {
    client_id: String,
    client_secret: String,
}

impl OpenverseCredentials {
    fn from_state(state: &EditorState) -> Option<Self> {
        let settings = &state.editor_ui.agent_settings;
        let client_id = settings.openverse_client_id.trim();
        let client_secret = settings.openverse_client_secret.trim();
        if client_id.is_empty() || client_secret.is_empty() {
            None
        } else {
            Some(Self {
                client_id: client_id.to_string(),
                client_secret: client_secret.to_string(),
            })
        }
    }
}

struct ImageSearchJob {
    node_id: NodeId,
    rx: Receiver<Option<String>>,
}

#[derive(Default)]
pub(crate) struct ImageSearchSession {
    in_flight: HashSet<String>,
    completed: HashSet<String>,
    jobs: Vec<ImageSearchJob>,
}

impl ImageSearchSession {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn reset(&mut self) {
        self.in_flight.clear();
        self.completed.clear();
        self.jobs.clear();
    }

    pub(crate) fn is_pending(&self) -> bool {
        !self.jobs.is_empty()
    }

    pub(crate) fn enqueue_missing(&mut self, state: &EditorState) -> bool {
        let mut known = self.completed.clone();
        known.extend(self.in_flight.iter().cloned());
        let targets = collect_targets(state, &known);
        if targets.is_empty() {
            return false;
        }
        let credentials = OpenverseCredentials::from_state(state);
        for target in targets {
            let id = target.node_id.as_str().to_string();
            self.in_flight.insert(id);
            self.jobs.push(spawn_job(target, credentials.clone()));
        }
        true
    }

    pub(crate) fn poll_into(&mut self, state: &mut EditorState) -> bool {
        let mut changed = false;
        let mut i = 0;
        while i < self.jobs.len() {
            match self.jobs[i].rx.try_recv() {
                Ok(url) => {
                    let job = self.jobs.swap_remove(i);
                    let id = job.node_id.as_str().to_string();
                    self.in_flight.remove(&id);
                    self.completed.insert(id);
                    if let Some(url) = url {
                        changed |= apply_result(state, &job.node_id, &url);
                    }
                }
                Err(TryRecvError::Empty) => {
                    i += 1;
                }
                Err(TryRecvError::Disconnected) => {
                    let job = self.jobs.swap_remove(i);
                    let id = job.node_id.as_str().to_string();
                    self.in_flight.remove(&id);
                    self.completed.insert(id);
                }
            }
        }
        changed
    }
}

fn spawn_job(
    target: ImageSearchTarget,
    credentials: Option<OpenverseCredentials>,
) -> ImageSearchJob {
    let (tx, rx) = mpsc::channel();
    let node_id = target.node_id.clone();
    std::thread::spawn(move || {
        let _ = tx.send(fetch_first_image_url_blocking(
            &target.query,
            credentials.as_ref(),
        ));
    });
    ImageSearchJob { node_id, rx }
}

pub(crate) fn collect_targets(
    state: &EditorState,
    known_node_ids: &HashSet<String>,
) -> Vec<ImageSearchTarget> {
    let mut targets = Vec::new();
    collect_from_children(state.active_children(), known_node_ids, &mut targets);
    targets
}

fn collect_from_children(
    children: &[PenNode],
    known_node_ids: &HashSet<String>,
    targets: &mut Vec<ImageSearchTarget>,
) {
    for node in children {
        if let PenNode::Image(image) = node {
            let id = image.base.id.as_str();
            let query = image
                .image_search_query
                .as_deref()
                .filter(|q| !q.trim().is_empty())
                .or(image.base.name.as_deref())
                .unwrap_or("placeholder")
                .trim();
            if image.src.trim().is_empty() && !query.is_empty() && !known_node_ids.contains(id) {
                targets.push(ImageSearchTarget {
                    node_id: NodeId::new(id),
                    query: query.to_string(),
                });
            }
        }
        if let Some(grand) = node.children() {
            collect_from_children(grand, known_node_ids, targets);
        }
    }
}

pub(crate) fn apply_result(state: &mut EditorState, node_id: &NodeId, url: &str) -> bool {
    let url = url.trim();
    if url.is_empty() {
        return false;
    }
    let Some(node) = walkers::find_node_mut(state.active_children_mut(), node_id) else {
        return false;
    };
    let PenNode::Image(image) = node else {
        return false;
    };
    if image.src == url {
        return false;
    }
    image.src = url.to_string();
    true
}

fn fetch_first_image_url_blocking(
    query: &str,
    credentials: Option<&OpenverseCredentials>,
) -> Option<String> {
    let query = query.trim();
    if query.is_empty() {
        return None;
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()?;
    runtime.block_on(fetch_first_image_url(query, credentials))
}

async fn fetch_first_image_url(
    query: &str,
    credentials: Option<&OpenverseCredentials>,
) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent(concat!("openpencil-desktop/", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()?;
    if let Some(url) = fetch_openverse(&client, query, credentials).await {
        return Some(url);
    }
    let words: Vec<&str> = query.split_whitespace().filter(|w| !w.is_empty()).collect();
    if words.len() > 2 {
        let truncated = words[..2].join(" ");
        if let Some(url) = fetch_openverse(&client, &truncated, credentials).await {
            return Some(url);
        }
        if let Some(url) = fetch_wikimedia(&client, &truncated).await {
            return Some(url);
        }
    }
    fetch_wikimedia(&client, query).await
}

async fn fetch_openverse(
    client: &reqwest::Client,
    query: &str,
    credentials: Option<&OpenverseCredentials>,
) -> Option<String> {
    let url = reqwest::Url::parse_with_params(
        "https://api.openverse.org/v1/images/",
        &[("q", query), ("page_size", "1")],
    )
    .ok()?;
    let mut request = client.get(url);
    if let Some(credentials) = credentials {
        if let Some(token) = fetch_openverse_token(client, credentials).await {
            request = request.bearer_auth(token);
        }
    }
    let resp = request.send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    let result = json.get("results")?.as_array()?.first()?;
    result
        .get("thumbnail")
        .and_then(serde_json::Value::as_str)
        .or_else(|| result.get("url").and_then(serde_json::Value::as_str))
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
}

async fn fetch_openverse_token(
    client: &reqwest::Client,
    credentials: &OpenverseCredentials,
) -> Option<String> {
    let resp = client
        .post("https://api.openverse.org/v1/auth_tokens/token/")
        .form(&[
            ("grant_type", "client_credentials"),
            ("client_id", credentials.client_id.as_str()),
            ("client_secret", credentials.client_secret.as_str()),
        ])
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    json.get("access_token")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(str::to_string)
}

async fn fetch_wikimedia(client: &reqwest::Client, query: &str) -> Option<String> {
    let url = reqwest::Url::parse_with_params(
        "https://commons.wikimedia.org/w/api.php",
        &[
            ("action", "query"),
            ("generator", "search"),
            ("gsrsearch", query),
            ("gsrnamespace", "6"),
            ("gsrlimit", "1"),
            ("prop", "imageinfo"),
            ("iiprop", "url|size|mime"),
            ("iiurlwidth", "800"),
            ("format", "json"),
            ("origin", "*"),
        ],
    )
    .ok()?;
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    let pages = json.get("query")?.get("pages")?.as_object()?;
    for page in pages.values() {
        if let Some(info) = page
            .get("imageinfo")
            .and_then(serde_json::Value::as_array)
            .and_then(|items| items.first())
        {
            if let Some(url) = info
                .get("thumburl")
                .and_then(serde_json::Value::as_str)
                .or_else(|| info.get("url").and_then(serde_json::Value::as_str))
                .filter(|s| !s.trim().is_empty())
            {
                return Some(url.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use jian_ops_schema::node::base::PenNodeBase;
    use jian_ops_schema::node::ImageNode;
    use jian_ops_schema::node::PenNode;
    use jian_ops_schema::sizing::SizingBehavior;

    fn image_node(id: &str, src: &str, query: Option<&str>) -> PenNode {
        PenNode::Image(ImageNode {
            base: PenNodeBase {
                id: id.to_string(),
                name: Some("Menu photo".into()),
                ..Default::default()
            },
            src: src.to_string(),
            object_fit: None,
            width: Some(SizingBehavior::Number(240.0)),
            height: Some(SizingBehavior::Number(160.0)),
            corner_radius: None,
            effects: None,
            exposure: None,
            contrast: None,
            saturation: None,
            temperature: None,
            tint: None,
            highlights: None,
            shadows: None,
            image_prompt: None,
            image_search_query: query.map(str::to_string),
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        })
    }

    #[test]
    fn collect_targets_prefers_query_on_empty_image_nodes() {
        let mut state = EditorState::default();
        state.active_children_mut().clear();
        state
            .active_children_mut()
            .push(image_node("img1", "", Some("burger fries")));

        let targets = collect_targets(&state, &HashSet::new());

        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].node_id.as_str(), "img1");
        assert_eq!(targets[0].query, "burger fries");
    }

    #[test]
    fn apply_result_sets_empty_image_src() {
        let mut state = EditorState::default();
        state.active_children_mut().clear();
        state
            .active_children_mut()
            .push(image_node("img1", "", Some("burger fries")));

        assert!(apply_result(
            &mut state,
            &NodeId::new("img1"),
            "https://example.com/photo.jpg"
        ));
        let PenNode::Image(image) = &state.active_children()[0] else {
            panic!("expected image");
        };
        assert_eq!(image.src, "https://example.com/photo.jpg");
    }

    #[test]
    fn openverse_credentials_require_both_fields() {
        let mut state = EditorState::default();
        assert!(OpenverseCredentials::from_state(&state).is_none());

        state.editor_ui.agent_settings.openverse_client_id = " client ".into();
        assert!(OpenverseCredentials::from_state(&state).is_none());

        state.editor_ui.agent_settings.openverse_client_secret = " secret ".into();
        let credentials = OpenverseCredentials::from_state(&state).expect("complete credentials");
        assert_eq!(credentials.client_id, "client");
        assert_eq!(credentials.client_secret, "secret");
    }

    #[tokio::test]
    #[ignore = "network smoke test for Openverse/Wikimedia"]
    async fn fetch_first_image_url_smoke() {
        let url = fetch_first_image_url("burger fries", None)
            .await
            .expect("common query should return an image URL");
        assert!(url.starts_with("http"), "got {url}");
    }
}
