//! 设计变量 seed —— 语义调色板播种 / 快照 / 回滚。
//!
//! 编排开始时把 56-token 语义调色板([`crate::semantic_palette`],
//! TS `applySemanticPalette` 的 Rust 移植)播进文档,让 theme="system"
//! 元素发出的 `$color-*` 引用在 paint 时能解析(此前 seeding 休眠,
//! 悬空引用渲染灰底)。
//!
//! 合并语义:**已有者赢**(对齐 TS)。`set_variables_bulk(merge)` 在
//! editor-core 侧是 `.extend`(新值赢),所以这里先用快照算出缺失集,
//! seed 只发缺失的 token / 轴 —— 文档里已自定义的 `color-accent`
//! 永远不被覆盖。
//!
//! 忠实的 styleGuideName → 解析 guide → 播种 guide 调色板仍是后续项;
//! 本模块是其缺省底座(DEFAULT_PALETTE 等价物)。

use crate::plan::OrchestratorPlan;
use crate::semantic_palette;
use crate::types::DocSink;
use op_editor_core::EditorCommand;

/// 回滚快照 —— seed 前"不存在"的调色板变量名 / 主题轴集合(即 seed
/// 会真正新建的那批)。
#[derive(Debug, Clone, Default)]
pub struct VarSnapshot {
    /// seed 前不存在的变量名 —— 回滚时删除这些。
    pub created: Vec<String>,
    /// seed 前不存在的主题轴 —— 回滚时从 themes 里剔除。
    pub created_axes: Vec<String>,
}

/// 在 seed *之前* 调用:对照现有文档变量/主题轴,算出调色板里缺失
/// 的部分。快照同时是 [`seed_commands`] 的输入(只播缺失项)和
/// [`rollback`] 的依据(只删自己新建的)。
pub fn snapshot_plan_vars(sink: &dyn DocSink, _plan: &OrchestratorPlan) -> VarSnapshot {
    let doc = &sink.state().doc;
    let existing_vars = doc.variables.as_ref();
    let created = semantic_palette::palette_names()
        .into_iter()
        .filter(|name| !existing_vars.is_some_and(|vars| vars.contains_key(*name)))
        .map(str::to_string)
        .collect();
    let existing_axes = doc.themes.as_ref();
    let created_axes = semantic_palette::palette_themes()
        .into_keys()
        .filter(|axis| !existing_axes.is_some_and(|axes| axes.contains_key(axis)))
        .collect();
    VarSnapshot {
        created,
        created_axes,
    }
}

/// 缺失的调色板 token / 轴 → 一条 `MergeThemePreset`。全部已存在时
/// 返回空(完全播种过的文档零命令)。
pub fn seed_commands(plan: &OrchestratorPlan, snap: &VarSnapshot) -> Vec<EditorCommand> {
    if snap.created.is_empty() && snap.created_axes.is_empty() {
        return Vec::new();
    }
    let mut palette = semantic_palette::palette_variables();
    // Harmonize the cool-slate neutral ramp to the design's temperature: a warm
    // page (cream/orange) gets warm grays so the search bar / chips / avatar
    // circle no longer read as off-palette cool gray. Neutral pages keep slate.
    if let Some(page_bg) = plan.root_frame.first_solid_hex() {
        crate::palette_harmonize::harmonize_palette_neutrals(&mut palette, &page_bg);
    }
    let variables = snap
        .created
        .iter()
        .filter_map(|name| palette.get(name).map(|def| (name.clone(), def.clone())))
        .collect();
    let themes = semantic_palette::palette_themes()
        .into_iter()
        .filter(|(axis, _)| snap.created_axes.contains(axis))
        .collect();
    vec![EditorCommand::MergeThemePreset { variables, themes }]
}

/// 回滚 seed 新建的变量与主题轴(已有者从未被覆盖,所以恢复 = 删除
/// 新建项)。
pub fn rollback(sink: &mut dyn DocSink, snap: &VarSnapshot) {
    for name in &snap.created {
        sink.apply(EditorCommand::DeleteVariable { name: name.clone() });
    }
    if !snap.created_axes.is_empty() {
        let themes = sink
            .state()
            .doc
            .themes
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter(|(axis, _)| !snap.created_axes.contains(axis))
            .collect();
        sink.apply(EditorCommand::SetThemes {
            themes,
            replace: true,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::VecDocSink;
    use jian_ops_schema::variable::{
        VariableDefinition, VariableKind, VariableScalar, VariableValue,
    };

    fn plan() -> OrchestratorPlan {
        crate::plan::build_fallback_plan(&crate::types::DesignRequest {
            prompt: "a page".into(),
            model: None,
            provider: None,
            design_md: None,
            concurrency: 1,
            append_context: None,
            validation_enabled: true,

            visual_ref_enabled: false,
        })
    }

    /// 空文档:快照报告全部 56 token + Mode 轴缺失,seed 出一条
    /// MergeThemePreset 把它们全部带上。
    #[test]
    fn seed_on_empty_doc_carries_full_palette() {
        let sink = VecDocSink::new();
        let snap = snapshot_plan_vars(&sink, &plan());
        assert_eq!(snap.created.len(), 56);
        assert_eq!(snap.created_axes, vec!["Mode".to_string()]);

        let cmds = seed_commands(&plan(), &snap);
        assert_eq!(cmds.len(), 1);
        let EditorCommand::MergeThemePreset { variables, themes } = &cmds[0] else {
            panic!("expected MergeThemePreset, got {:?}", cmds[0]);
        };
        assert_eq!(variables.len(), 56);
        assert!(variables.contains_key("color-accent"));
        assert_eq!(themes.get("Mode").map(Vec::len), Some(2));
    }

    /// 已有同名变量获胜(TS applySemanticPalette 语义):快照不把它
    /// 计入 created,seed 不发它 —— editor-core 的 merge 是新值赢,
    /// 守住"已有者赢"全靠这里不发。
    #[test]
    fn seed_respects_existing_variables_and_axes() {
        let mut sink = VecDocSink::new();
        sink.state.doc.variables = Some(
            [(
                "color-accent".to_string(),
                VariableDefinition {
                    kind: VariableKind::Color,
                    value: VariableValue::Scalar(VariableScalar::Str("#FF00FF".into())),
                },
            )]
            .into(),
        );
        sink.state.doc.themes = Some([("Mode".to_string(), vec!["Light".to_string()])].into());

        let snap = snapshot_plan_vars(&sink, &plan());
        assert_eq!(snap.created.len(), 55);
        assert!(!snap.created.contains(&"color-accent".to_string()));
        assert!(snap.created_axes.is_empty(), "Mode axis pre-exists");

        let cmds = seed_commands(&plan(), &snap);
        let EditorCommand::MergeThemePreset { variables, themes } = &cmds[0] else {
            panic!("expected MergeThemePreset");
        };
        assert!(!variables.contains_key("color-accent"));
        assert!(themes.is_empty());
    }

    /// 全量播种过的文档:零命令。
    #[test]
    fn seed_is_noop_when_fully_seeded() {
        let mut sink = VecDocSink::new();
        sink.state.doc.variables = Some(
            crate::semantic_palette::palette_variables()
                .into_iter()
                .collect(),
        );
        sink.state.doc.themes = Some(crate::semantic_palette::palette_themes());
        let snap = snapshot_plan_vars(&sink, &plan());
        assert!(snap.created.is_empty());
        assert!(snap.created_axes.is_empty());
        assert!(seed_commands(&plan(), &snap).is_empty());
    }

    /// 回滚删除新建变量,并把新建的 Mode 轴从 themes 里剔除。
    #[test]
    fn rollback_deletes_created_vars_and_axes() {
        let mut sink = VecDocSink::new();
        let snap = VarSnapshot {
            created: vec!["color-accent".to_string()],
            created_axes: vec!["Mode".to_string()],
        };
        rollback(&mut sink, &snap);
        assert!(sink.applied.iter().any(|c| matches!(
            c,
            EditorCommand::DeleteVariable { name } if name == "color-accent"
        )));
        assert!(sink.applied.iter().any(|c| matches!(
            c,
            EditorCommand::SetThemes { themes, replace: true } if !themes.contains_key("Mode")
        )));
    }

    #[test]
    fn rollback_of_empty_snapshot_is_noop() {
        let mut sink = VecDocSink::new();
        rollback(&mut sink, &VarSnapshot::default());
        assert!(sink.applied.is_empty());
    }
}
