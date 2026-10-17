use super::*;
use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
use crate::types::{AbortFlag, AppendContext};
use futures::executor::block_on;
use op_editor_core::EditorState;
use serde_json::{json, Value};

// Retained 1018 two-screen regression: exact planner labels/elements and the
// generated section tree. Its section has no persistent-action name or role.
const RETAINED: &str = r###"{
  "plan": {
    "rootFrame": {
      "id": "page",
      "name": "Page",
      "width": 375,
      "height": 812,
      "layout": "vertical",
      "gap": 0,
      "fill": [
        {
          "type": "solid",
          "color": "#FFFCF6"
        }
      ]
    },
    "styleGuideName": "warm-food-mobile-light",
    "sharedContext": {},
    "subtasks": [
      {
        "id": "detail-header",
        "label": "详情头图区",
        "screen": "商家详情",
        "elements": "全宽头图 210px（美食实拍图），左上角返回按钮（onTap pop 回首页），底部向上叠加商家信息卡：商家名「川味小馆」、评分 4.8、月售 1200+、配送费与配送时间行、公告「满 30 减 5」优惠标签",
        "region": {
          "width": 375,
          "height": 262
        }
      },
      {
        "id": "detail-menu",
        "label": "菜品分类与列表",
        "screen": "商家详情",
        "elements": "左侧竖向分类栏（招牌热销/盖饭/小食/饮品）+ 顶部横滑分类 chips 联动；菜品列表六条：每条左侧 88×88 圆角菜品照片、菜名、月售、价格 ¥18 起、右侧圆形「+加入购物车」按钮（pressed 态缩放 0.9 动效，加入后变数量步进器 − 1 +）",
        "region": {
          "width": 375,
          "height": 400
        }
      },
      {
        "id": "detail-cart",
        "label": "购物车条",
        "screen": "商家详情",
        "elements": "底部购物车条：左侧购物车图标带数量角标 3、合计「¥58」及「另需配送费 ¥4」小字，右侧「去结算」按钮暖橙色圆角；悬浮于菜品列表上方，带阴影",
        "region": {
          "width": 375,
          "height": 78
        }
      }
    ]
  },
  "section": {
    "type": "frame",
    "id": "n544",
    "name": "购物车条",
    "width": "fill_container",
    "height": "fit_content",
    "layout": "vertical",
    "padding": [
      0.0,
      16.0,
      0.0,
      16.0
    ],
    "children": [
      {
        "type": "frame",
        "id": "n545",
        "name": "cart-pill",
        "width": "fill_container",
        "height": 60.0,
        "layout": "horizontal",
        "gap": 12.0,
        "padding": [
          0.0,
          6.0,
          0.0,
          8.0
        ],
        "alignItems": "center",
        "cornerRadius": 9999.0,
        "fill": [
          {
            "type": "solid",
            "color": "#21140F"
          }
        ],
        "effects": [
          {
            "type": "shadow",
            "offsetX": 0.0,
            "offsetY": 8.0,
            "blur": 20.0,
            "spread": 0.0,
            "color": "#21140F2E"
          }
        ],
        "children": [
          {
            "type": "frame",
            "id": "n546",
            "name": "cart-disc-wrap",
            "width": 46.0,
            "height": 46.0,
            "layout": "horizontal",
            "justifyContent": "center",
            "alignItems": "center",
            "fill": [],
            "stroke": {
              "thickness": 1.0,
              "fill": [
                {
                  "type": "solid",
                  "color": "$--sidebar-border"
                }
              ]
            },
            "children": [
              {
                "type": "icon_font",
                "id": "n547",
                "name": "cart-icon",
                "iconFontName": "shopping-cart",
                "iconFontFamily": "lucide",
                "width": 20.0,
                "height": 20.0,
                "fill": [
                  {
                    "type": "solid",
                    "color": "#FFFFFF"
                  }
                ]
              },
              {
                "type": "frame",
                "id": "n548",
                "name": "cart-count-badge",
                "role": "badge",
                "x": 28.0,
                "y": -4.0,
                "width": 16.0,
                "height": 16.0,
                "layout": "horizontal",
                "padding": [
                  0.0,
                  0.0
                ],
                "justifyContent": "center",
                "alignItems": "center",
                "cornerRadius": 999.0,
                "fill": [
                  {
                    "type": "solid",
                    "color": "$--primary"
                  }
                ],
                "children": [
                  {
                    "type": "text",
                    "id": "n549",
                    "name": "cart-count",
                    "width": "fit_content",
                    "height": "fit_content",
                    "content": "3",
                    "fontFamily": "Plus Jakarta Sans, Noto Sans SC",
                    "fontSize": 10.0,
                    "fontWeight": 700,
                    "lineHeight": 1.5,
                    "textAlign": "center",
                    "textGrowth": "auto",
                    "fill": [
                      {
                        "type": "solid",
                        "color": "$--accent-foreground"
                      }
                    ]
                  }
                ]
              }
            ]
          },
          {
            "type": "frame",
            "id": "n550",
            "name": "cart-total-block",
            "width": "fill_container",
            "height": "fit_content",
            "layout": "vertical",
            "gap": 2.0,
            "children": [
              {
                "type": "text",
                "id": "n551",
                "name": "cart-total",
                "width": "fit_content",
                "height": "fit_content",
                "content": "¥58",
                "fontFamily": "Plus Jakarta Sans, Noto Sans SC",
                "fontSize": 19.0,
                "fontWeight": 700,
                "letterSpacing": -0.3,
                "lineHeight": 1.5,
                "textGrowth": "auto",
                "fill": [
                  {
                    "type": "solid",
                    "color": "#FFFFFF"
                  }
                ]
              },
              {
                "type": "text",
                "id": "n552",
                "name": "cart-fee-hint",
                "width": "fit_content",
                "height": "fit_content",
                "content": "另需配送费 ¥4",
                "fontFamily": "Plus Jakarta Sans, Noto Sans SC",
                "fontSize": 11.0,
                "fontWeight": 400,
                "lineHeight": 1.5,
                "textGrowth": "auto",
                "fill": [
                  {
                    "type": "solid",
                    "color": "#D7C6B8"
                  }
                ]
              }
            ]
          },
          {
            "type": "frame",
            "id": "n553",
            "name": "checkout-button",
            "role": "button",
            "animations": [
              {
                "trigger": "mount",
                "keyframes": [
                  {
                    "offset": 0.0,
                    "values": {
                      "opacity": 0,
                      "scaleX": 0.94
                    }
                  },
                  {
                    "offset": 1.0,
                    "values": {
                      "opacity": 1,
                      "scaleX": 1
                    }
                  }
                ],
                "durationMs": 300,
                "easing": "emphasizedDecelerate"
              }
            ],
            "width": 104.0,
            "height": 48.0,
            "layout": "horizontal",
            "padding": [
              12.0,
              16.0
            ],
            "justifyContent": "center",
            "alignItems": "center",
            "cornerRadius": 9999.0,
            "fill": [
              {
                "type": "solid",
                "color": "$--primary"
              }
            ],
            "children": [
              {
                "type": "text",
                "id": "n554",
                "name": "checkout-label",
                "width": "fit_content",
                "height": "fit_content",
                "content": "去结算",
                "fontFamily": "Plus Jakarta Sans, Noto Sans SC",
                "fontSize": 15.0,
                "fontWeight": 600,
                "lineHeight": 1.5,
                "textGrowth": "auto",
                "fill": [
                  {
                    "type": "solid",
                    "color": "$--accent-foreground"
                  }
                ]
              }
            ]
          }
        ]
      }
    ]
  }
}"###;

fn fixture() -> (OrchestratorPlan, Vec<PenNode>) {
    let value: Value = serde_json::from_str(RETAINED).unwrap();
    (
        serde_json::from_value(value["plan"].clone()).unwrap(),
        vec![serde_json::from_value(value["section"].clone()).unwrap()],
    )
}

#[test]
fn retained_bottom_floating_intent_marks_the_section_without_renaming_it() {
    let (plan, mut nodes) = fixture();
    assert!(!crate::mobile_action_bar::is_action_bar(&nodes[0]));
    let mut expected = serde_json::to_value(&nodes).unwrap();
    preserve(
        &mut nodes,
        plan.subtasks.last().unwrap(),
        &plan,
        &DesignRequest::default(),
    );
    assert!(crate::mobile_action_bar::is_action_bar(&nodes[0]));
    expected[0]["explain"] = json!(MARKER);
    assert_eq!(serde_json::to_value(&nodes).unwrap(), expected);
    let once = nodes.clone();
    preserve(
        &mut nodes,
        plan.subtasks.last().unwrap(),
        &plan,
        &DesignRequest::default(),
    );
    assert_eq!(nodes, once);
}

#[test]
fn floating_bottom_actions_support_generic_chinese_and_english_layout_intent() {
    for elements in [
        "Floating bottom action with a Continue button",
        "Bottom action bar with a floating Submit button above the content",
        "底部悬浮操作栏，包含继续按钮",
        "浮动于底部的确认按钮",
    ] {
        let (mut plan, mut nodes) = fixture();
        let task = plan.subtasks.last_mut().unwrap();
        task.label = "Action area".into();
        task.elements = Some(elements.into());
        preserve(
            &mut nodes,
            plan.subtasks.last().unwrap(),
            &plan,
            &DesignRequest::default(),
        );
        assert!(is_marked(&nodes[0]), "{elements}");
    }
}

#[test]
fn negated_floating_and_ordinary_content_never_gain_pinning() {
    for elements in [
        "Bottom action button, not floating",
        "Bottom action button, not a floating bar",
        "Non-floating bottom action button",
        "No floating bottom action button",
        "Bottom floating action button, not fixed",
        "Bottom floating action button, scrolls with content",
        "Bottom floating action button in normal document flow",
        "Floating top action with a Submit button",
        "Floating decoration at the bottom of the article",
        "An ordinary body paragraph followed by a button",
        "Bottom action button after the article",
        "底部按钮，不要悬浮",
        "底部按钮不悬浮",
        "非悬浮的底部操作栏",
        "底部按钮不要浮动",
        "底部按钮不浮动",
        "非浮动的底部操作栏",
        "底部悬浮按钮，非固定",
        "底部悬浮操作栏随内容滚动",
        "底部悬浮按钮随正文滚动",
        "普通正文中的底部按钮，使用正常文档流，不固定",
        "正文最后的继续按钮",
        "底部悬浮装饰带，不含操作",
    ] {
        let (mut plan, mut nodes) = fixture();
        let task = plan.subtasks.last_mut().unwrap();
        task.label = "Section".into();
        task.elements = Some(elements.into());
        let before = nodes.clone();
        preserve(
            &mut nodes,
            plan.subtasks.last().unwrap(),
            &plan,
            &DesignRequest::default(),
        );
        assert_eq!(nodes, before, "{elements}");
    }
}

#[test]
fn floating_intent_keeps_existing_structural_and_request_boundaries() {
    for case in [
        "positioned",
        "constraints",
        "multiple roots",
        "desktop",
        "short board",
        "absolute root",
        "absolute section",
        "numeric section width",
        "large section",
        "no action",
        "nonterminal",
        "append",
        "reference",
        "continuation",
    ] {
        let (mut plan, mut nodes) = fixture();
        let mut req = DesignRequest::default();
        match case {
            "positioned" => nodes[0].base_mut().y = Some(0.0),
            "constraints" => {
                let mut value = serde_json::to_value(&nodes[0]).unwrap();
                value["constraints"] = json!({"h":"left","v":"bottom"});
                nodes[0] = serde_json::from_value(value).unwrap();
            }
            "multiple roots" => nodes.push(nodes[0].clone()),
            "desktop" => plan.root_frame.width = 1440.0,
            "short board" => plan.root_frame.height = 300.0,
            "absolute root" => plan.root_frame.layout = Some("none".into()),
            "absolute section" => {
                if let PenNode::Frame(frame) = &mut nodes[0] {
                    frame.container.layout = Some(LayoutMode::None);
                }
            }
            "numeric section width" => {
                if let PenNode::Frame(frame) = &mut nodes[0] {
                    frame.container.width = Some(SizingBehavior::Number(375.0));
                }
            }
            "large section" => plan.subtasks.last_mut().unwrap().region.height = 300.0,
            "no action" => {
                if let PenNode::Frame(frame) = &mut nodes[0] {
                    frame.children = Some(vec![]);
                }
            }
            "nonterminal" => {
                let mut extra = plan.subtasks[0].clone();
                extra.id = "later-content".into();
                plan.subtasks.push(extra);
            }
            "append" => {
                req.append_context = Some(AppendContext {
                    target_parent_id: "existing".into(),
                    target_width: 375.0,
                    existing_section_labels: vec![],
                    is_mobile: true,
                })
            }
            "reference" => {
                req.reference_skeleton = Some(
                    serde_json::from_value(json!({
                        "source":"reference","width":375,"sections":[],"navKind":"none",
                        "heroKind":"none","columnRhythm":[]
                    }))
                    .unwrap(),
                )
            }
            "continuation" => {
                req.continuation_context = Some(crate::types::ContinuationContext {
                    screen_width: 375.0,
                    screen_height: 812.0,
                    background_color: None,
                    screen_names: vec!["Detail".into()],
                })
            }
            _ => unreachable!(),
        }
        let before = nodes.clone();
        let task = plan
            .subtasks
            .iter()
            .find(|task| task.id == "detail-cart")
            .unwrap();
        preserve(&mut nodes, task, &plan, &req);
        assert_eq!(nodes, before, "{case}");
    }
}

#[test]
fn floating_action_is_terminal_per_screen_in_a_multi_screen_plan() {
    let (mut plan, mut nodes) = fixture();
    let mut next_screen = plan.subtasks[0].clone();
    next_screen.id = "another-screen".into();
    next_screen.screen = Some("Another screen".into());
    plan.subtasks.push(next_screen);
    let task = plan
        .subtasks
        .iter()
        .find(|task| task.id == "detail-cart")
        .unwrap();
    preserve(&mut nodes, task, &plan, &DesignRequest::default());
    assert!(is_marked(&nodes[0]));
}

#[test]
fn retained_floating_section_is_kept_outside_scroll_after_insertion_cleanup_and_save() {
    let (mut plan, nodes) = fixture();
    plan.subtasks.last_mut().unwrap().parent_frame_id = Some("page".into());
    let task = plan.subtasks.last().unwrap().clone();
    let req = DesignRequest {
        prompt:"外卖 App 两屏可交互（375×812）：首页(地址栏+分类九宫格+限时横幅+商家列表五条)、商家详情(头图+菜品分类横滑+菜品列表六条+购物车条)。交互：商家行 onTap 进详情。动效：限时横幅倒计时数字滚动，商家卡 inView 交错 fade-up，加入购物车按钮 pressed 态缩放。".into(),
        ..DesignRequest::default()
    };
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(format!(
        "I(null, {});",
        serde_json::to_string(&nodes[0]).unwrap()
    ))]);
    let mut sink = VecDocSink::new();
    sink.state = EditorState::from_document(serde_json::from_value(json!({
        "version":"1.0.0","children":[{
            "type":"frame","id":"page","name":"商家详情","width":375,"height":812,
            "layout":"vertical","gap":16,"children":[
                {"type":"frame","id":"status","role":"status-bar","name":"Status Bar","width":"fill_container","height":62},
                {"type":"frame","id":"body","name":"Content","width":"fill_container","height":1000,"layout":"vertical","children":[
                    {"type":"text","id":"body-copy","content":"Menu list","fontSize":16,"height":1000}
                ]}
            ]
        }]
    })).unwrap());
    let result = block_on(crate::subagent::run_subtask(
        &task,
        &plan,
        &req,
        &llm,
        &mut sink,
        &AbortFlag::new(),
        false,
        false,
    ));
    assert!(result.error.is_none(), "{:?}", result.error);
    assert!(is_marked(
        sink.state.active_children()[0]
            .children()
            .unwrap()
            .last()
            .unwrap()
    ));
    crate::cleanup::finalize_design_with_summary_and_policy(
        &mut sink,
        &plan,
        &["page"],
        &mut crate::repair_summary::RepairSummary::default(),
        crate::cleanup::CleanupPolicy {
            preserve_requested_root_height: true,
            roots_are_run_output: true,
            is_deck: false,
        },
    );
    let stored = serde_json::to_string(&sink.state.doc).unwrap();
    sink.state = EditorState::from_document(serde_json::from_str(&stored).unwrap());
    let root = &sink.state.active_children()[0];
    let sections = root.children().unwrap();
    let footer = sections.last().unwrap();
    assert!(is_marked(footer));
    assert!(sections
        .iter()
        .any(|node| node.base().role.as_deref() == Some("scroll-area")));
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(&sink.state);
    let page = scene.active_page().unwrap();
    let board = page.find(root.id_str()).unwrap().bounds;
    let bar = page.find(footer.id_str()).unwrap().bounds;
    assert_eq!(board.size.y, 812.0);
    assert!(bar.origin.y > board.origin.y + 650.0, "{bar:?}");
    assert!(
        bar.origin.y + bar.size.y <= board.origin.y + board.size.y,
        "{bar:?}"
    );
    assert!(bar.size.y >= 60.0, "{bar:?}");
}
