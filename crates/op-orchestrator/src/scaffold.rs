//! 阶段 2 —— 单屏画布搭建。
//!
//! 产出一条 `InsertSubtree`:把根 frame(移动端再带一个固定状态
//! 栏 child)插到活动页根。根 frame 用 JSON 构建后反序列化为
//! canonical `PenNode` —— 避免在 Rust 侧硬写富 schema 的每个字段,
//! 且与 `parse` 模块的解析路径一致。
//!
//! # Concurrent (multi-root) scaffold
//!
//! [`build_scaffold_concurrent`] handles the N-root-frame concurrent path
//! (S3b-2 Task A2). One root frame per [`ScreenGroup`], laid out
//! left-to-right with gap 100. Returns `(cmds, root_ids, baselines)` where
//! `baselines[i]` is the scaffold descendant count for root `i` (0 for
//! desktop, 1 for mobile with status bar).

use crate::concurrent::ScreenGroup;
use crate::plan::{OrchestratorPlan, Subtask};
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId};

/// 移动端固定状态栏的高度(iOS 风格 chrome,通用 mockup 约定)。
const STATUS_BAR_HEIGHT: f64 = 44.0;

/// 并发多屏路径的相邻 root frame 间距(像素)。
/// Port of `const gap = 100` in `orchestrator.ts:868`.
const CONCURRENT_GAP: f64 = 100.0;

/// 并发路径里每个 root 的最小高度(桌面端)。
/// Port of `Math.max(320, totalRegionHeight)` in `orchestrator.ts:883`.
const CONCURRENT_MIN_HEIGHT: f64 = 320.0;

/// 移动端无 rootFrame.height 时的默认视口高度。
/// Port of `plan.rootFrame.height || 812` in `orchestrator.ts:882`.
const MOBILE_DEFAULT_HEIGHT: f64 = 812.0;

/// Return type of [`build_scaffold_concurrent`] /
/// [`build_scaffold_concurrent_mobile`]:
/// `(commands, root_frame_ids, per_root_scaffold_baselines)`.
///
/// - `Vec<EditorCommand>` — one `InsertSubtree` per screen group.
/// - `Vec<String>` — the root frame ID for each group (index-aligned).
/// - `Vec<usize>` — scaffold descendant-count baseline per root
///   (0 for desktop, 1 for mobile with status bar).
pub(crate) type ConcurrentScaffoldResult = (Vec<EditorCommand>, Vec<String>, Vec<usize>);

/// 构建单个根 frame 的 `PenNode`,含可选状态栏子节点。
///
/// 内部辅助函数,被 `build_scaffold`(单屏)和
/// `build_scaffold_concurrent`(多屏)共用。
#[allow(clippy::too_many_arguments)]
fn build_root_frame_node(
    id: &str,
    name: &str,
    x: f64,
    width: f64,
    height: f64,
    layout: &str,
    gap: f64,
    fill_hex: &str,
    is_mobile: bool,
) -> Result<PenNode, String> {
    let children = if is_mobile {
        serde_json::json!([{
            "type": "frame",
            "id": format!("{id}-status-bar"),
            "name": "Status Bar",
            "x": 0,
            "y": 0,
            "width": width,
            "height": STATUS_BAR_HEIGHT,
            "fill": [{ "type": "solid", "color": fill_hex }],
            "children": [],
        }])
    } else {
        serde_json::json!([])
    };

    let frame = serde_json::json!({
        "type": "frame",
        "id": id,
        "name": name,
        "x": x,
        "y": 0,
        "width": width,
        "height": height,
        "layout": layout,
        "gap": gap,
        "fill": [{ "type": "solid", "color": fill_hex }],
        "children": children,
    });

    serde_json::from_value(frame).map_err(|e| format!("scaffold root frame `{id}`: {e}"))
}

/// 从 subtask 的 label 剥去括号后缀并 trim,用作 frame 名 fallback。
/// Port of `firstSt.label.replace(/\s*[（(].+$/, '').trim() || firstSt.label`
/// in `orchestrator.ts:888`.
#[allow(dead_code)]
fn short_label(st: &Subtask) -> String {
    let stripped = if let Some(pos) = st.label.find(['（', '(']) {
        st.label[..pos].trim().to_string()
    } else {
        st.label.trim().to_string()
    };
    if stripped.is_empty() {
        st.label.clone()
    } else {
        stripped
    }
}

/// 构建阶段 2 的画布命令(单屏顺序路径)。
///
/// `is_mobile` 为真时根 frame 带一个固定状态栏 child。
/// 返回 `Err` 表示根 frame JSON 模板有问题(实现 bug,非用户输入问题)。
pub fn build_scaffold(
    plan: &OrchestratorPlan,
    is_mobile: bool,
) -> Result<Vec<EditorCommand>, String> {
    let rf = &plan.root_frame;
    let layout = rf.layout.as_deref().unwrap_or("vertical");
    let fill_hex = rf
        .first_solid_hex()
        .unwrap_or_else(|| "#FFFFFF".to_string());

    let node = build_root_frame_node(
        &rf.id,
        &rf.name,
        0.0,
        rf.width,
        rf.height,
        layout,
        rf.gap.unwrap_or(0.0),
        &fill_hex,
        is_mobile,
    )?;

    Ok(vec![EditorCommand::InsertSubtree {
        nodes: vec![node],
        parent_id: NodeId::NONE,
    }])
}

/// 构建 N-root-frame 并发画布搭建命令(多屏并发路径,S3b-2 Task A2)。
///
/// Port of `orchestrator.ts:856-937` (the `effectiveConcurrency > 1` scaffold
/// branch). One root frame per screen group, laid out left-to-right with
/// gap 100. Each group's subtasks get `parent_frame_id` = that group's root ID.
///
/// # Arguments
/// * `plan` — orchestrator plan (provides `root_frame` spec).
/// * `groups` — screen groups from [`crate::concurrent::group_subtasks_by_screen`].
///
/// # Returns
/// See [`ConcurrentScaffoldResult`]:
/// `(cmds, root_ids, baselines)`.
///
/// # Side effect on plan subtasks
/// The caller is responsible for assigning `parent_frame_id` on each subtask
/// from the returned `root_ids`. This function does NOT mutate `plan` — it
/// returns the mapping data needed for the caller to do so.
///
/// Callers land in S3b-2 Task C2 (`run.rs`).
#[allow(dead_code)]
pub(crate) fn build_scaffold_concurrent(
    plan: &OrchestratorPlan,
    groups: &[ScreenGroup],
) -> Result<ConcurrentScaffoldResult, String> {
    build_scaffold_concurrent_inner(plan, groups, false)
}

/// Mobile-aware variant; exposed as a separate entry point so tests can drive
/// the mobile path without going through `plan_normalize`.
///
/// Callers land in S3b-2 Task C2 (`run.rs`).
#[allow(dead_code)]
pub(crate) fn build_scaffold_concurrent_mobile(
    plan: &OrchestratorPlan,
    groups: &[ScreenGroup],
    is_mobile: bool,
) -> Result<ConcurrentScaffoldResult, String> {
    build_scaffold_concurrent_inner(plan, groups, is_mobile)
}

#[allow(dead_code)]
fn build_scaffold_concurrent_inner(
    plan: &OrchestratorPlan,
    groups: &[ScreenGroup],
    is_mobile: bool,
) -> Result<ConcurrentScaffoldResult, String> {
    let rf = &plan.root_frame;
    let layout = rf.layout.as_deref().unwrap_or("vertical");
    let fill_hex = rf
        .first_solid_hex()
        .unwrap_or_else(|| "#FFFFFF".to_string());

    let mut cmds: Vec<EditorCommand> = Vec::with_capacity(groups.len());
    let mut root_ids: Vec<String> = Vec::with_capacity(groups.len());
    let mut baselines: Vec<usize> = Vec::with_capacity(groups.len());

    let mut next_x: f64 = 0.0;

    for (g, group) in groups.iter().enumerate() {
        // Original ID: `{root_frame.id}-{group.screen}` — mirrors TS `originalId`.
        // The first group uses the same id scheme; `run()` handles the
        // empty-canvas remap (only for g == 0) after applying the InsertSubtree.
        let original_id = format!("{}-{}", rf.id, group.screen);

        // Height: mobile uses fixed viewport; desktop = max(320, Σ region heights).
        let total_region_height: f64 = group
            .indices
            .iter()
            .map(|&i| {
                plan.subtasks
                    .get(i)
                    .map(|st| st.region.height)
                    .unwrap_or(0.0)
            })
            .sum();

        let frame_height = if is_mobile {
            if rf.height > 0.0 {
                rf.height
            } else {
                MOBILE_DEFAULT_HEIGHT
            }
        } else {
            total_region_height.max(CONCURRENT_MIN_HEIGHT)
        };

        // Frame name: use screen name if the first subtask has one,
        // else first subtask's short label. Port of `orchestrator.ts:886-888`.
        let frame_name =
            if let Some(first_st) = group.indices.first().and_then(|&i| plan.subtasks.get(i)) {
                if first_st.screen.is_some() {
                    group.screen.clone()
                } else {
                    short_label(first_st)
                }
            } else {
                group.screen.clone()
            };

        let node = build_root_frame_node(
            &original_id,
            &frame_name,
            next_x,
            rf.width,
            frame_height,
            layout,
            rf.gap.unwrap_or(0.0),
            &fill_hex,
            is_mobile,
        )?;

        // The first root frame uses InsertSubtree with NONE parent (same as the
        // single-root path) so the run() caller can handle the empty-canvas
        // ID-remap. Subsequent roots are plain InsertSubtree with NONE parent
        // (addNode into page root — mirrors TS `addNode(null, rootNode)`).
        let _ = g; // both paths use NodeId::NONE — kept for clarity
        cmds.push(EditorCommand::InsertSubtree {
            nodes: vec![node],
            parent_id: NodeId::NONE,
        });

        root_ids.push(original_id);
        // Baseline: mobile root has 1 scaffold child (status bar); desktop has 0.
        baselines.push(if is_mobile { 1 } else { 0 });

        next_x += rf.width + CONCURRENT_GAP;
    }

    Ok((cmds, root_ids, baselines))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::concurrent::ScreenGroup;
    use crate::plan::{OrchestratorPlan, PlanFill, Region, RootFrameSpec, Subtask};
    use op_editor_core::PenNodeExt;

    fn plan() -> OrchestratorPlan {
        OrchestratorPlan {
            root_frame: RootFrameSpec {
                id: "root".into(),
                name: "Design".into(),
                width: 1200.0,
                height: 800.0,
                layout: Some("vertical".into()),
                gap: Some(0.0),
                padding: Some(0.0),
                fill: Some(vec![PlanFill {
                    kind: "solid".into(),
                    color: "#FFFFFF".into(),
                }]),
            },
            subtasks: vec![],
            style_guide_name: None,
        }
    }

    fn plan_with_subtasks(subtasks: Vec<Subtask>) -> OrchestratorPlan {
        OrchestratorPlan { subtasks, ..plan() }
    }

    fn subtask(id: &str, screen: Option<&str>, height: f64) -> Subtask {
        Subtask {
            id: id.into(),
            label: id.into(),
            id_prefix: id.into(),
            region: Region {
                width: 1200.0,
                height,
            },
            parent_frame_id: None,
            elements: None,
            screen: screen.map(|s| s.to_string()),
        }
    }

    // ── existing single-root tests (must stay green) ───────────────────────────

    #[test]
    fn build_scaffold_desktop_one_root_no_children() {
        let cmds = build_scaffold(&plan(), false).expect("scaffold");
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            EditorCommand::InsertSubtree { nodes, parent_id } => {
                assert_eq!(nodes.len(), 1);
                assert!(!parent_id.is_real()); // NONE → page root
                assert_eq!(nodes[0].id_str(), "root");
                assert!(nodes[0].children().map(|c| c.is_empty()).unwrap_or(true));
            }
            other => panic!("expected InsertSubtree, got {other:?}"),
        }
    }

    #[test]
    fn build_scaffold_mobile_injects_status_bar() {
        let cmds = build_scaffold(&plan(), true).expect("scaffold");
        match &cmds[0] {
            EditorCommand::InsertSubtree { nodes, .. } => {
                let children = nodes[0].children().expect("frame children");
                assert_eq!(children.len(), 1);
            }
            other => panic!("expected InsertSubtree, got {other:?}"),
        }
    }

    // ── Task A2: concurrent scaffold tests ────────────────────────────────────

    /// 3 screen groups → 3 root frames.
    #[test]
    fn concurrent_scaffold_three_groups_gives_three_roots() {
        let subtasks = vec![
            subtask("a", Some("login"), 400.0),
            subtask("b", Some("home"), 600.0),
            subtask("c", Some("profile"), 500.0),
        ];
        let plan = plan_with_subtasks(subtasks);
        let groups = vec![
            ScreenGroup {
                screen: "login".into(),
                indices: vec![0],
            },
            ScreenGroup {
                screen: "home".into(),
                indices: vec![1],
            },
            ScreenGroup {
                screen: "profile".into(),
                indices: vec![2],
            },
        ];
        let (cmds, root_ids, baselines) =
            build_scaffold_concurrent(&plan, &groups).expect("concurrent scaffold");
        assert_eq!(cmds.len(), 3, "expected 3 InsertSubtree commands");
        assert_eq!(root_ids.len(), 3);
        assert_eq!(baselines.len(), 3);
        // All 3 are InsertSubtree commands.
        for cmd in &cmds {
            assert!(matches!(cmd, EditorCommand::InsertSubtree { .. }));
        }
    }

    /// Frame 2's x = width + 100, frame 3's x = 2*(width+100).
    #[test]
    fn concurrent_scaffold_x_layout_left_to_right_gap_100() {
        let subtasks = vec![
            subtask("a", Some("s1"), 400.0),
            subtask("b", Some("s2"), 400.0),
            subtask("c", Some("s3"), 400.0),
        ];
        let plan = plan_with_subtasks(subtasks);
        let groups = vec![
            ScreenGroup {
                screen: "s1".into(),
                indices: vec![0],
            },
            ScreenGroup {
                screen: "s2".into(),
                indices: vec![1],
            },
            ScreenGroup {
                screen: "s3".into(),
                indices: vec![2],
            },
        ];
        let (cmds, _, _) = build_scaffold_concurrent(&plan, &groups).expect("scaffold");
        let xs: Vec<f64> = cmds
            .iter()
            .map(|cmd| match cmd {
                EditorCommand::InsertSubtree { nodes, .. } => nodes[0].base().x.unwrap_or(0.0),
                _ => panic!("expected InsertSubtree"),
            })
            .collect();
        assert_eq!(xs[0], 0.0, "frame 0 x should be 0");
        assert_eq!(
            xs[1], 1300.0,
            "frame 1 x should be width(1200)+gap(100)=1300"
        );
        assert_eq!(xs[2], 2600.0, "frame 2 x should be 2*(1200+100)=2600");
    }

    /// Each group's root ID is assigned; root_ids are distinct.
    #[test]
    fn concurrent_scaffold_root_ids_are_distinct() {
        let subtasks = vec![
            subtask("a", Some("login"), 400.0),
            subtask("b", Some("home"), 600.0),
        ];
        let plan = plan_with_subtasks(subtasks);
        let groups = vec![
            ScreenGroup {
                screen: "login".into(),
                indices: vec![0],
            },
            ScreenGroup {
                screen: "home".into(),
                indices: vec![1],
            },
        ];
        let (_, root_ids, _) = build_scaffold_concurrent(&plan, &groups).expect("scaffold");
        assert_eq!(root_ids.len(), 2);
        assert_ne!(root_ids[0], root_ids[1], "root IDs must be distinct");
    }

    /// Desktop height = max(320, Σ region heights) per group.
    #[test]
    fn concurrent_scaffold_desktop_height_is_max_320_sum_region_heights() {
        // Group 0: subtask heights 200 + 300 = 500 → height 500 (≥320)
        // Group 1: subtask height 100 → height 320 (clamped to min 320)
        let subtasks = vec![
            subtask("a", Some("s1"), 200.0),
            subtask("b", Some("s1"), 300.0),
            subtask("c", Some("s2"), 100.0),
        ];
        let plan = plan_with_subtasks(subtasks);
        let groups = vec![
            ScreenGroup {
                screen: "s1".into(),
                indices: vec![0, 1],
            },
            ScreenGroup {
                screen: "s2".into(),
                indices: vec![2],
            },
        ];
        let (cmds, _, _) = build_scaffold_concurrent(&plan, &groups).expect("scaffold");
        let heights: Vec<f64> = cmds
            .iter()
            .map(|cmd| match cmd {
                EditorCommand::InsertSubtree { nodes, .. } => nodes[0].height_px().unwrap_or(0.0),
                _ => panic!("expected InsertSubtree"),
            })
            .collect();
        assert_eq!(heights[0], 500.0, "group 0 height should be sum=500");
        assert_eq!(heights[1], 320.0, "group 1 height should be min=320");
    }

    /// Mobile height = plan.root_frame.height (or 812 if 0).
    #[test]
    fn concurrent_scaffold_mobile_height_uses_root_frame_height() {
        let subtasks = vec![
            subtask("a", Some("s1"), 200.0),
            subtask("b", Some("s2"), 300.0),
        ];
        let plan = plan_with_subtasks(subtasks);
        let groups = vec![
            ScreenGroup {
                screen: "s1".into(),
                indices: vec![0],
            },
            ScreenGroup {
                screen: "s2".into(),
                indices: vec![1],
            },
        ];
        let (cmds, _, _) =
            build_scaffold_concurrent_mobile(&plan, &groups, true).expect("scaffold");
        // plan.root_frame.height = 800.0, so both roots should have height 800
        for cmd in &cmds {
            match cmd {
                EditorCommand::InsertSubtree { nodes, .. } => {
                    assert_eq!(
                        nodes[0].height_px().unwrap_or(0.0),
                        800.0,
                        "mobile height should equal root_frame.height"
                    );
                }
                _ => panic!("expected InsertSubtree"),
            }
        }
    }

    /// Mobile scaffold baseline = 1 per root (status bar injected).
    #[test]
    fn concurrent_scaffold_mobile_baseline_is_one_per_root() {
        let subtasks = vec![
            subtask("a", Some("s1"), 200.0),
            subtask("b", Some("s2"), 300.0),
        ];
        let plan = plan_with_subtasks(subtasks);
        let groups = vec![
            ScreenGroup {
                screen: "s1".into(),
                indices: vec![0],
            },
            ScreenGroup {
                screen: "s2".into(),
                indices: vec![1],
            },
        ];
        let (cmds, _, baselines) =
            build_scaffold_concurrent_mobile(&plan, &groups, true).expect("scaffold");
        assert_eq!(baselines, vec![1, 1], "mobile baselines should be 1 each");
        // Each root frame should have 1 child (status bar).
        for cmd in &cmds {
            match cmd {
                EditorCommand::InsertSubtree { nodes, .. } => {
                    let children = nodes[0].children().expect("frame has children");
                    assert_eq!(
                        children.len(),
                        1,
                        "mobile root should have status bar child"
                    );
                }
                _ => panic!("expected InsertSubtree"),
            }
        }
    }

    /// Desktop scaffold baseline = 0 per root.
    #[test]
    fn concurrent_scaffold_desktop_baseline_is_zero_per_root() {
        let subtasks = vec![
            subtask("a", Some("s1"), 400.0),
            subtask("b", Some("s2"), 400.0),
            subtask("c", Some("s3"), 400.0),
        ];
        let plan = plan_with_subtasks(subtasks);
        let groups = vec![
            ScreenGroup {
                screen: "s1".into(),
                indices: vec![0],
            },
            ScreenGroup {
                screen: "s2".into(),
                indices: vec![1],
            },
            ScreenGroup {
                screen: "s3".into(),
                indices: vec![2],
            },
        ];
        let (_, _, baselines) = build_scaffold_concurrent(&plan, &groups).expect("scaffold");
        assert_eq!(
            baselines,
            vec![0, 0, 0],
            "desktop baselines should be 0 each"
        );
    }

    /// Frame name = group's screen string (when first subtask has a screen).
    #[test]
    fn concurrent_scaffold_frame_name_uses_screen_string() {
        let subtasks = vec![
            subtask("a", Some("Login Screen"), 400.0),
            subtask("b", Some("Home Screen"), 400.0),
        ];
        let plan = plan_with_subtasks(subtasks);
        let groups = vec![
            ScreenGroup {
                screen: "Login Screen".into(),
                indices: vec![0],
            },
            ScreenGroup {
                screen: "Home Screen".into(),
                indices: vec![1],
            },
        ];
        let (cmds, _, _) = build_scaffold_concurrent(&plan, &groups).expect("scaffold");
        let names: Vec<String> = cmds
            .iter()
            .map(|cmd| match cmd {
                EditorCommand::InsertSubtree { nodes, .. } => {
                    nodes[0].base().name.clone().unwrap_or_default()
                }
                _ => panic!("expected InsertSubtree"),
            })
            .collect();
        assert_eq!(names[0], "Login Screen");
        assert_eq!(names[1], "Home Screen");
    }

    /// All InsertSubtree commands use NodeId::NONE as parent_id (page root insert).
    #[test]
    fn concurrent_scaffold_all_inserts_use_none_parent() {
        let subtasks = vec![
            subtask("a", Some("s1"), 400.0),
            subtask("b", Some("s2"), 400.0),
        ];
        let plan = plan_with_subtasks(subtasks);
        let groups = vec![
            ScreenGroup {
                screen: "s1".into(),
                indices: vec![0],
            },
            ScreenGroup {
                screen: "s2".into(),
                indices: vec![1],
            },
        ];
        let (cmds, _, _) = build_scaffold_concurrent(&plan, &groups).expect("scaffold");
        for cmd in &cmds {
            match cmd {
                EditorCommand::InsertSubtree { parent_id, .. } => {
                    assert!(
                        !parent_id.is_real(),
                        "all concurrent roots insert at page level"
                    );
                }
                _ => panic!("expected InsertSubtree"),
            }
        }
    }

    /// Empty groups slice → empty results (no panic).
    #[test]
    fn concurrent_scaffold_empty_groups_returns_empty() {
        let (cmds, root_ids, baselines) =
            build_scaffold_concurrent(&plan(), &[]).expect("scaffold");
        assert!(cmds.is_empty());
        assert!(root_ids.is_empty());
        assert!(baselines.is_empty());
    }
}
