//! `impl Orchestrator` — the four-phase run driver (plan → scaffold →
//! sub-agents → finalize).

use super::*;

impl Orchestrator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adopt a host-owned indicator epoch. The host clears with
    /// `agent_indicators::clear_if_epoch(epoch)` on stop / new-chat, so
    /// the concurrent path registers under this epoch instead of minting
    /// its own — otherwise the host couldn't target the right run.
    pub fn with_indicator_epoch(mut self, epoch: u64) -> Self {
        self.agent_indicator_epoch = Some(epoch);
        self
    }

    /// 跑一次完整编排。见 spec §4 数据流。
    ///
    /// 规划 → 单根 scaffold → 顺序子 agent → 清理 的单一路径。
    pub async fn run(
        &self,
        request: DesignRequest,
        sink: &mut dyn DocSink,
        llm: &dyn LlmClient,
        on_progress: &mut dyn FnMut(Progress),
        abort: &AbortFlag,
        providers: &ValidationProviders<'_>,
    ) -> Result<RunSummary, OrchestratorError> {
        // -- 阶段 1:规划(单档 Rich + 规范化)--
        // `planning_loop` 内部已 normalize 并回传 `NormInfo`,此处不再二次规范化。
        on_progress(Progress::Planning);
        let (mut plan, norm) = planning_loop(&request, llm, abort).await?;

        // -- S3b-4 Task B2 call site 1: apply append context (TS :737) --
        // Must run AFTER planning_loop (which calls normalize) so root_frame.id
        // and subtasks are already normalized before we repoint them.
        let append_result =
            apply_append_context_to_plan(&mut plan, request.append_context.as_ref());

        // `OPENPENCIL_DEBUG_PLAN=1` prints what the planner decided beyond the
        // subtask list: headless runs (op-smoke, the arena) otherwise cannot
        // tell whether a style guide was picked or a phone root was planned.
        if std::env::var_os("OPENPENCIL_DEBUG_PLAN").is_some() {
            eprintln!(
                "[PLAN] root={}x{} styleGuideName={:?} subtasks={}",
                plan.root_frame.width,
                plan.root_frame.height,
                plan.style_guide_name,
                plan.subtasks.len()
            );
        }

        // Surface the FULL planned task list upfront (TS parity) so the UI can
        // render the complete checklist immediately, rather than revealing
        // subtasks one-by-one as each starts.
        on_progress(Progress::Planned {
            subtasks: plan
                .subtasks
                .iter()
                .map(|s| (s.id.clone(), s.label.clone()))
                .collect(),
        });

        // Run-wide `geometry_echo` budget (`concurrent::run_subtask_retry_ladder`'s
        // tail step) — `min(subtask_count, 6)` unless the
        // `OPENPENCIL_GEOMETRY_ECHO=0` rollback valve is set. Constructed
        // ONCE per run and shared (by reference) across BOTH the sequential
        // loop and every screen-group worker below, so the cap is a
        // genuine run-wide total, not per-path. See `geometry_echo_cap`'s
        // doc for why the env read stays this one untested line.
        let geometry_echo_budget =
            crate::types::GeometryEchoBudget::new(crate::types::geometry_echo_cap(
                plan.subtasks.len(),
                std::env::var("OPENPENCIL_GEOMETRY_ECHO").ok().as_deref(),
            ));

        // Dashboards (formerly a bespoke sidebar+main scaffold) flow through
        // the same single-root pipeline as any other single-screen plan —
        // they produced byte-identical per-subtask output, only differing in
        // scaffold shape. Multi-screen plans get N scaffold roots (item A)
        // and, when `request.concurrency` allows it, run their screen groups
        // genuinely CONCURRENTLY (item D-lite) — see `groups` / `effective_concurrency`
        // below and the module doc.
        let planned_root_id = plan.root_frame.id.clone();

        // -- 进入"已动文档"区,全程 undo batch 包裹 --
        sink.begin_undo_batch();
        let var_snapshot = snapshot_plan_vars(sink, &plan);

        // -- 阶段 2:画布搭建 --
        for cmd in seed_commands(&plan, &var_snapshot) {
            sink.apply(cmd);
        }
        let scaffold_root_ids_before: Vec<String> = sink
            .state()
            .active_children()
            .iter()
            .map(|n| n.id_str().to_string())
            .collect();

        // Screen grouping (multiscreen-fanout-break fix, item A): a plan
        // whose subtasks span ≥2 distinct `screen` labels gets one scaffold
        // root PER GROUP instead of one shared root. Zero labels, or every
        // subtask sharing the SAME one, both give `groups.len() <= 1` — the
        // single-root path below runs byte-identical to today (regression
        // lock). Multi-root is mutually exclusive with append mode (a
        // continuation turn always targets ONE existing frame) and with the
        // empty-canvas-reuse path (reuse only replaces ONE frame in place —
        // see `insert_screen_group_roots`'s doc). `groups.len()` also feeds
        // `effective_concurrency` below (item D-lite) — computed once here so
        // both the scaffold branch and the phase-3 executor choice see the
        // SAME grouping.
        let groups = if append_result.skip_root_insertion {
            Vec::new()
        } else {
            group_subtasks_by_screen(&plan.subtasks)
        };

        // `effective_concurrency` only needs `request.concurrency` +
        // `groups.len()` — computed here (before scaffold, not just before
        // phase 3) so the scaffold step below can decide whether to hand
        // `insert_screen_group_roots` ONE shared identity (still a single
        // agent working through N screens sequentially) or N DISTINCT
        // identities (genuine concurrent agents — three-piece visibility
        // fix, 2026-07-17: distinct per-group colour/name is what lets the
        // canvas show N cursors instead of one).
        let effective_concurrency =
            crate::concurrent::effective_concurrency(request.concurrency, groups.len());

        // Single source of truth for agent identity (2026-07-17,
        // dual-cursor-identity fix — see the module doc's "Identity" section):
        // the orchestrator ONLY mints/tags its own identities when groups
        // genuinely run CONCURRENTLY; the sequential path (single agent,
        // whether single-root or multiple screen groups run one after
        // another) tags NOTHING, so every reveal falls through to
        // `agent_indicators::cursor_agent` — the ONE identity the host's
        // transcript pump already confirms for the session. Before this fix
        // the sequential path independently minted its own "Kiki" (seed-0)
        // identity and tagged frames with it while the host separately
        // confirmed a random transcript identity ("Fern") — two unrelated
        // sources that `canvas_agent_cursor.rs`'s old confirmed-always-wins
        // precedence silently papered over. Flipping that precedence
        // (three-piece visibility fix) made the split visible as two
        // cursors; the real fix is here, not reverting the precedence.
        let group_identities: Vec<crate::agent_identity::AgentIdentity> = if groups.len() > 1
            && effective_concurrency > 1
        {
            // Some callers confirm a `cursor_agent` BEFORE calling `run()` at
            // all — the web streaming route announces + confirms a persona
            // to the client immediately (the SSE transcript needs one before
            // groups/concurrency is even known), well before this point. If
            // we minted a fresh set here regardless, the primary group's
            // badge would silently diverge from whatever persona the caller
            // already told its client — the SAME split as the desktop bug,
            // just from the opposite direction (the CALLER confirmed first,
            // not the orchestrator). So: adopt whatever is ALREADY confirmed
            // for OUR epoch as the primary identity instead of minting one,
            // and only generate fresh identities for the other groups.
            let already_confirmed = self.agent_indicator_epoch.and_then(|epoch| {
                (op_editor_core::agent_indicators::active_epoch() == Some(epoch))
                    .then(op_editor_core::agent_indicators::snapshot)
                    .and_then(|snap| snap.cursor_agent)
            });
            let identities = match already_confirmed {
                Some(tag) => crate::agent_identity::assign_agent_identities_with_primary(
                    crate::agent_identity::AgentIdentity {
                        color: tag.color,
                        name: tag.name,
                    },
                    groups.len(),
                ),
                None => crate::agent_identity::assign_agent_identities(groups.len()),
            };
            // Confirm the FIRST (primary) group's identity as the canonical
            // `cursor_agent` BEFORE any host-side pump gets a chance to mint
            // an independent one — a no-op when it was already confirmed
            // (the web case above), the FIRST confirmation otherwise (the
            // desktop case, where nothing pre-exists at this point).
            if let (Some(epoch), Some(primary)) = (self.agent_indicator_epoch, identities.first()) {
                op_editor_core::agent_indicators::confirm_cursor_agent(
                    epoch,
                    &primary.color,
                    &primary.name,
                );
            }
            identities
        } else {
            Vec::new()
        };

        let (root_ids, scaffold_baselines, created_scaffold_root_ids): (
            Vec<String>,
            Vec<usize>,
            Vec<String>,
        ) = if append_result.skip_root_insertion {
            let target_id = plan.root_frame.id.clone();
            for subtask in &mut plan.subtasks {
                subtask.parent_frame_id = Some(target_id.clone());
            }
            let baseline = descendant_count(sink.state(), &target_id);
            on_progress(Progress::ScaffoldDone);
            // Append mode targets a user-owned frame that existed before this
            // run. It participates in zero-content accounting, but it is NEVER
            // a disposable scaffold owned by the orchestrator.
            (vec![target_id], vec![baseline], Vec::new())
        } else {
            let effective_is_mobile = norm.is_mobile && !append_result.skip_status_bar;

            if groups.len() > 1 {
                match insert_screen_group_roots(
                    &mut plan,
                    &groups,
                    effective_is_mobile,
                    sink,
                    &scaffold_root_ids_before,
                    self.agent_indicator_epoch,
                    &group_identities,
                ) {
                    Ok((ids, baselines)) => {
                        on_progress(Progress::ScaffoldDone);
                        let created = ids.clone();
                        (ids, baselines, created)
                    }
                    Err(e) => {
                        rollback(sink, &var_snapshot);
                        sink.end_undo_batch();
                        return Err(OrchestratorError::Internal(e.to_string()));
                    }
                }
            } else {
                // TS `replaceEmptyFrame` parity: when the canvas is a single empty
                // top-level frame (the fresh-canvas starter), REUSE it as the design
                // root (ReplaceSubtree in place) instead of inserting a brand-new
                // root — which the host would otherwise clear + re-add, the visible
                // "delete then re-draw" flash the user flagged.
                let reuse_id = detect_reusable_empty_frame(sink.state());
                let reused_existing_frame = reuse_id.is_some();
                let (insert_x, insert_y) =
                    next_root_insert_position(sink.state(), plan.root_frame.width);
                let scaffold_cmds = match reuse_id.as_deref() {
                    Some(id) => build_scaffold_reusing(&plan, effective_is_mobile, id),
                    None => build_scaffold_at(&plan, effective_is_mobile, insert_x, insert_y),
                };
                match scaffold_cmds {
                    Ok(cmds) => {
                        for cmd in cmds {
                            if !apply_command_with_reveal(
                                sink,
                                cmd,
                                self.agent_indicator_epoch,
                                reveal_now_millis(),
                            ) {
                                rollback(sink, &var_snapshot);
                                sink.end_undo_batch();
                                return Err(OrchestratorError::Internal(
                                    "scaffold insert rejected by document".into(),
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        // scaffold 模板 bug —— 收尾后报内部错误。
                        rollback(sink, &var_snapshot);
                        sink.end_undo_batch();
                        return Err(OrchestratorError::Internal(e.to_string()));
                    }
                }
                let Some(rid) = sink.state().active_children().iter().find_map(|n| {
                    let id = n.id_str();
                    (!scaffold_root_ids_before.iter().any(|old| old == id)).then(|| id.to_string())
                }) else {
                    rollback(sink, &var_snapshot);
                    sink.end_undo_batch();
                    return Err(OrchestratorError::Internal(format!(
                        "scaffold root `{planned_root_id}` was not inserted"
                    )));
                };
                // Route subtasks into the scaffold. For a pre-built two-column
                // dashboard shell, the sidebar subtask fills the (260-wide) left
                // column and every other subtask fills the content column; the
                // column ids were remapped on insert, so re-resolve them by name.
                // Any other plan keeps the single-root behaviour (all → root).
                let two_col =
                    crate::scaffold::plan_is_sidebar_dashboard(&plan, effective_is_mobile)
                        .then(|| {
                            let sb = find_child_id_by_name(
                                sink.state(),
                                &rid,
                                crate::scaffold::SIDEBAR_COLUMN_NAME,
                            );
                            let ct = find_child_id_by_name(
                                sink.state(),
                                &rid,
                                crate::scaffold::CONTENT_COLUMN_NAME,
                            );
                            sb.zip(ct)
                        })
                        .flatten();
                for subtask in &mut plan.subtasks {
                    let parent = match &two_col {
                        Some((sidebar_id, content_id)) => {
                            if crate::dashboard_columns::is_sidebar_subtask(subtask) {
                                sidebar_id.clone()
                            } else {
                                content_id.clone()
                            }
                        }
                        None => rid.clone(),
                    };
                    subtask.parent_frame_id = Some(parent);
                }
                // No orchestrator-side frame tagging here — this is always
                // the single-agent sequential path (single root), so the
                // host's confirmed transcript identity (`cursor_agent`) is
                // the sole source; see the `group_identities` doc above for
                // why (dual-cursor-identity fix, 2026-07-17).
                let baseline = descendant_count(sink.state(), &rid);
                on_progress(Progress::ScaffoldDone);
                let created = if reused_existing_frame {
                    Vec::new()
                } else {
                    vec![rid.clone()]
                };
                (vec![rid], vec![baseline], created)
            }
        };

        // -- 阶段 3:子 agent(C3: 3-attempt tier-gated retry ladder)--
        //
        // Port of `orchestrator-sub-agent.ts:128-206` (sequential path).
        //
        // Per subtask:
        //   Attempt 1: reduced_complexity=false, minimal_skills=false
        //   Attempt 2: reduced_complexity=(tier==Basic), minimal_skills=false
        //   Attempt 3: reduced_complexity=true, minimal_skills=true
        //
        // A retryable failure = error.is_some() && node_count==0
        //                       && !abort.is_set() && !is_non_retryable(&err).
        // non_retryable is evaluated from attempt-1's error and cached
        // (matching TS semantics where `isNonRetryable` is computed once
        // before the retry chain). The ladder itself lives in
        // `concurrent::run_subtask_retry_ladder` — shared by this sequential
        // loop and every screen-group worker so parallelizing groups (item
        // D-lite) can never drift from this retry semantics.
        //
        // A partial result (node_count > 0) is never retried.
        // After 3 still-zero → zero_node_failure stop.
        let tier = resolve_model_profile(request.model.as_deref().unwrap_or("")).tier;
        // `effective_concurrency` was already computed above (before the
        // scaffold step, so `group_identities` could see it too) — reused
        // here unchanged, not recomputed.

        let (mut outcomes, mut aborted_mid, mut zero_node_failure, salvage): (
            Vec<SubtaskOutcome>,
            bool,
            bool,
            Vec<(usize, usize)>,
        ) = if effective_concurrency > 1 {
            // Item D-lite: ≥2 screen groups AND the user's ⚡Nx setting
            // allows it — run the groups genuinely CONCURRENTLY (never
            // same-screen section parallelism; see the module doc for why
            // that distinction matters against `aca0d3a0`'s data verdict).
            let result = crate::concurrent::run_screen_groups_concurrent(
                &groups,
                &group_identities,
                &plan,
                &request,
                llm,
                sink,
                abort,
                tier,
                effective_concurrency,
                self.agent_indicator_epoch,
                &geometry_echo_budget,
                on_progress,
            )
            .await;
            (
                result.outcomes,
                result.aborted_mid,
                result.zero_node_failure,
                result.salvage,
            )
        } else {
            // Single group / single screen / append mode — the ORIGINAL
            // sequential loop, unchanged in behavior (byte-identical
            // regression lock), just calling the extracted retry ladder.
            //
            // Self-diagnostic (2026-07-17, sequential-execution root-cause
            // hunt): when ≥2 screen groups exist but this branch still ran
            // (i.e. `effective_concurrency == 1` despite `groups.len() > 1`),
            // announce it — this is the dual of `ConcurrentGroupsStarted`. By
            // `effective_concurrency`'s own contract this can only happen
            // when `clamp_concurrency(request.concurrency) <= 1`, so the
            // announced `requested_workers` value is diagnostic gold: `1`
            // here proves the ⚡Nx picker's value never reached
            // `DesignRequest.concurrency` for this turn; anything `> 1`
            // would mean `effective_concurrency` itself has a bug.
            if groups.len() > 1 {
                on_progress(Progress::ScreenGroupsSequential {
                    group_count: groups.len(),
                    requested_workers: request.concurrency,
                });
            }
            let mut outcomes: Vec<SubtaskOutcome> = Vec::new();
            let mut aborted_mid = false;
            let mut zero_node_failure = false;
            // (subtask index, outcomes index) of every all-attempts-failed
            // subtask, for the end-of-run salvage pass below.
            let mut salvage: Vec<(usize, usize)> = Vec::new();
            for (subtask_index, subtask) in plan.subtasks.iter().enumerate() {
                if abort.is_set() {
                    aborted_mid = true;
                    break;
                }
                let outcome = crate::concurrent::run_subtask_retry_ladder_with_outcomes(
                    subtask,
                    &plan,
                    &request,
                    llm,
                    sink,
                    abort,
                    tier,
                    self.agent_indicator_epoch,
                    &geometry_echo_budget,
                    on_progress,
                    &outcomes,
                )
                .await;

                let zero = outcome.node_count == 0;
                let node_count = outcome.node_count;
                let err_msg = outcome.error.clone();
                outcomes.push(outcome);

                // abort 在 run_subtask 期间被置位 —— 优先于零节点判定归
                // abort 路径(否则 mid-stream abort 会被误判为错误路径,
                // 错误地移除 scaffold root 并返回 NoContent 而非 Aborted)。
                if abort.is_set() {
                    aborted_mid = true;
                    if zero {
                        on_progress(Progress::SubtaskFailed {
                            id: subtask.id.clone(),
                            error: err_msg.unwrap_or_else(|| "aborted".into()),
                        });
                    } else {
                        on_progress(Progress::SubtaskDone {
                            id: subtask.id.clone(),
                            node_count,
                        });
                    }
                    break;
                }
                if zero {
                    // 零节点失败(非 abort,全部 3 次皆失败)。**不 break** ——
                    // 一个 section 失败不该放弃后续所有 subtask。各 subtask
                    // 独立 InsertSubtree 到 root、互不依赖;break 会把失败点
                    // 之后的必要内容(bottom nav 等)全丢掉(用户报的"管线丢
                    // 内容")。跳过这个、继续后面的;`zero_node_failure` 仍标记
                    // "至少一个失败",最终若**全部**零内容(zero_content)才删
                    // scaffold root。
                    on_progress(Progress::SubtaskFailed {
                        id: subtask.id.clone(),
                        error: err_msg.unwrap_or_default(),
                    });
                    zero_node_failure = true;
                    salvage.push((subtask_index, outcomes.len() - 1));
                    continue;
                }
                on_progress(Progress::SubtaskDone {
                    id: subtask.id.clone(),
                    node_count,
                });
            }
            (outcomes, aborted_mid, zero_node_failure, salvage)
        };

        // -- 阶段 4.4:失败抢救轮 --
        // 瞬时故障(供应商网络抖动、偶发空回复)会把一个 subtask 的 3 次
        // 紧挨着的尝试全部烧掉(measured:Ark 连续 3 次 "empty content from
        // provider" → 侧栏子任务整段消失,设计**无侧栏出厂**且无可见信号)。
        // 其余 subtask 跑完后隔了几十秒再给每个失败者最后一次完整尝试 ——
        // 瞬时故障此时多已恢复;仍失败的维持 SubtaskFailed,不再重试。
        // Reuse attempt 3's minimal skill tier and persisted subtask feedback.
        // This keeps deterministic self-check guidance while still giving
        // transient provider failures one final recovery window.
        if !salvage.is_empty() && !abort.is_set() {
            for (subtask_index, outcome_index) in salvage {
                if abort.is_set() {
                    aborted_mid = true;
                    break;
                }
                if !crate::run_salvage_feedback::should_salvage(outcomes.get(outcome_index)) {
                    continue;
                }
                let subtask = crate::run_salvage_feedback::subtask_for_salvage(
                    outcomes.get(outcome_index),
                    &plan.subtasks[subtask_index],
                );
                let prior_outcomes = outcomes.get(..outcome_index).unwrap_or(&[]);
                on_progress(scope_progress_for_subtask(
                    &groups,
                    &group_identities,
                    subtask_index,
                    Progress::SubtaskRetry {
                        id: subtask.id.clone(),
                        attempt: 4,
                        reason: "salvage pass after transient failures".into(),
                    },
                ));
                let mut outcome = run_subtask_with_reveal_at_and_outcomes(
                    &subtask,
                    &plan,
                    &request,
                    llm,
                    sink,
                    abort,
                    true,
                    true,
                    self.agent_indicator_epoch,
                    reveal_now_millis(),
                    None,
                    prior_outcomes,
                )
                .await;
                if outcome.node_count > 0 {
                    // The salvage appended the section after everything
                    // generated since it first failed; put it back where
                    // the plan placed it.
                    crate::run_salvage_feedback::restore_planned_order(
                        sink,
                        &plan,
                        &outcomes,
                        subtask_index,
                        &outcome,
                    );
                    on_progress(scope_progress_for_subtask(
                        &groups,
                        &group_identities,
                        subtask_index,
                        Progress::SubtaskDone {
                            id: subtask.id.clone(),
                            node_count: outcome.node_count,
                        },
                    ));
                    outcomes[outcome_index] = outcome;
                } else {
                    let error = crate::run_salvage_feedback::finalize_failed_salvage(&mut outcome);
                    on_progress(scope_progress_for_subtask(
                        &groups,
                        &group_identities,
                        subtask_index,
                        Progress::SubtaskFailed {
                            id: subtask.id.clone(),
                            error,
                        },
                    ));
                    outcomes[outcome_index] = outcome;
                }
            }
            zero_node_failure = outcomes.iter().any(|o| o.node_count == 0);
        }

        // -- 阶段 4.5:收尾判定(spec §6.3 三路径)--
        // Compute zero-content BEFORE cleanup. `finalize_design`'s structural
        // passes swap the root via `ReplaceSubtree`, which allocates a FRESH root
        // id; a post-cleanup `descendant_count(&root_id)` would then look up the
        // now-STALE id, read 0, and declare a false "no content" — which rolls
        // back the theme variables of a perfectly good design and returns
        // `NoContent`. Cleanup only RESTRUCTURES (never adds content), so the
        // pre-cleanup count is the correct "did the subtasks produce content"
        // signal. See `reshaped_dashboard_root_is_not_a_false_no_content`.
        //
        // Multi-root (screen groups): SUM the per-root added-content across
        // every group instead of a single root's count. Every term is ≥0
        // (subtasks only ever ADD nodes), so a zero SUM implies every
        // individual root is ALSO empty — the all-roots-empty deletion below
        // is therefore never a false positive against a partially-successful
        // group.
        let zero_content = root_ids
            .iter()
            .zip(scaffold_baselines.iter())
            .map(|(id, baseline)| descendant_count(sink.state(), id).saturating_sub(*baseline))
            .sum::<usize>()
            == 0;
        if zero_content {
            // 错误路径才移除本轮真正新建的空 scaffold root(s);
            // append 目标和复用的既有空 frame 都是用户节点，绝不能删除。
            // abort / 正常零内容仍然只回滚变量。
            if zero_node_failure {
                for id in &created_scaffold_root_ids {
                    sink.apply(EditorCommand::DeleteNode {
                        node_id: NodeId::new(id.clone()),
                        page_id: None,
                    });
                }
            }
            rollback(sink, &var_snapshot);
            sink.end_undo_batch();
            return Err(if aborted_mid {
                OrchestratorError::Aborted
            } else if let Some(first_error) = outcomes
                .iter()
                .find_map(|o| o.error.as_deref().filter(|s| !s.is_empty()))
            {
                OrchestratorError::AllFailed(first_error.to_string())
            } else {
                OrchestratorError::NoContent
            });
        }

        // Let the reveal sweep FINISH before cleanup restructures the tree:
        // `finalize_design`'s ReplaceSubtree allocates fresh ids that were
        // never registered with the reveal overlay, so a section still
        // mid-animation snaps in all at once and the agent cursor loses its
        // target (measured: the tail of a run popped in "一口气" while
        // earlier sections streamed). Worker-thread wait, abort-aware.
        crate::subagent::wait_for_reveal_drain(self.agent_indicator_epoch, abort);

        // -- 阶段 4:清理 --（有内容才跑；空 root 无可清理）
        // Append mode (skip_root_insertion): scope cleanup to ONLY the roots
        // this run inserted (post-remap ids from each outcome) so pre-existing
        // nodes under the target frame are never restyled (Component 11b).
        // If inserted_root_ids is empty (nothing inserted, or buffered sink),
        // the empty slice is a safe no-op — do NOT fall back to the whole target
        // root, which would reprocess old nodes.
        //
        // Fresh-document mode reuses the single page/target root — every node
        // under it is new — so the behaviour is unchanged there.
        let mut quality = crate::repair_summary::RepairSummary::default();
        if append_result.skip_root_insertion {
            let new_roots: Vec<&str> = outcomes
                .iter()
                .flat_map(|o| o.inserted_root_ids.iter().map(String::as_str))
                .collect();
            let target_roots: Vec<&str> = root_ids.iter().map(String::as_str).collect();
            crate::repair_scope::finalize_appended_design(
                sink,
                &plan,
                &new_roots,
                &target_roots,
                &mut quality,
            );
        } else {
            // Every screen-group root (not just the first) goes in — this is
            // the co-op point with `wire_screen_navigation` (Track A of the
            // interactive-preview plan): `run_cleanup_passes` runs it LAST,
            // over `sink.state()` as a whole, so as long as every new root is
            // actually IN the document by now (it is — they're inserted
            // above, before any subtask runs) it links all of them into App
            // Mode navigation regardless of which root_ids are passed here.
            // Passing them all is still correct scoping for the OTHER
            // whole-root cleanup passes (dedup / avatar-repair / etc.).
            let root_id_refs: Vec<&str> = root_ids.iter().map(String::as_str).collect();
            finalize_design_with_summary_and_policy(
                sink,
                &plan,
                &root_id_refs,
                &mut quality,
                CleanupPolicy {
                    preserve_requested_root_height: norm.preserve_requested_root_height,
                    is_deck: norm.is_deck,
                    // `root_ids` here are the screen-group roots this run just
                    // inserted — every node under them is this run's own.
                    roots_are_run_output: true,
                },
            );
        }
        on_progress(Progress::CleanupDone);
        // Turn the cleanup stage's tally into a user-visible credential. Only
        // fires when the passes actually ran (an empty summary means cleanup
        // was skipped entirely), so nothing is ever vouched for that nobody
        // checked.
        if !quality.is_empty() {
            on_progress(Progress::QualityChecked {
                checks: quality
                    .checked()
                    .into_iter()
                    .map(|c| c.key().to_string())
                    .collect(),
                repairs: quality
                    .repaired()
                    .into_iter()
                    .map(|(check, count)| (check.key().to_string(), count))
                    .collect(),
                records: quality
                    .records()
                    .iter()
                    .map(crate::RepairRecord::line)
                    .collect(),
                notes: quality.notes().to_vec(),
            });
        }
        sink.end_undo_batch();

        // -- 阶段 5:视觉校验 (S3c D1) — 在 cleanup 后、返回 RunSummary 前 --
        // Port of `orchestrator.ts:1247-1292`.
        // 守卫: request.validation_enabled && !abort.is_set().
        if request.validation_enabled && !abort.is_set() {
            let _ = run_post_generation_validation(
                sink,
                providers.pre_validator,
                providers.screenshot,
                providers.vision,
                &providers.system_prompt,
                &request,
                on_progress,
                abort,
            );
        }

        // -- 阶段 6:承诺-交付不变量(classic 路径的诚实上报,与 loop 路径共享检测器)--
        // Runs AFTER cleanup + validation so a screen a structural pass or
        // the vision loop still touched is judged on its FINAL state, not
        // a stale mid-run snapshot. `mark_unfilled_screens` labels the
        // canvas itself (not just this summary) — a user scrolling the
        // layer panel sees it even without reading `RunSummary`.
        let unfilled_hits = crate::unfilled_screens::detect_unfilled_screens(sink.state());
        let unfilled_screens: Vec<String> = unfilled_hits.iter().map(|h| h.name.clone()).collect();
        if !unfilled_hits.is_empty() {
            crate::unfilled_screens::mark_unfilled_screens(sink, &unfilled_hits);
            on_progress(Progress::UnfilledScreens {
                names: unfilled_screens.clone(),
            });
        }

        let total_nodes = outcomes.iter().map(|o| o.node_count).sum();
        Ok(RunSummary {
            // First surviving root is the "primary" root_frame_id — mirrors
            // the deleted concurrent path's identical convention so this
            // field's meaning never changed shape for existing callers.
            root_frame_id: root_ids.first().cloned().unwrap_or_default(),
            subtasks: outcomes,
            total_nodes,
            unfilled_screens,
        })
    }
}
