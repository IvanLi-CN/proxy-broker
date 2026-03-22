use crate::models::{
    TaskListQuery, TaskRunDetail, TaskRunEventRecord, TaskRunStatus, TaskRunSummary, TaskSummary,
};

#[derive(Debug, Clone)]
pub enum TaskBusEvent {
    RunUpsert(TaskRunSummary),
    RunEvent(TaskRunEventRecord),
}

pub fn matches_task_query(run: &TaskRunSummary, query: &TaskListQuery) -> bool {
    if let Some(profile_id) = &query.profile_id
        && profile_id != "all"
        && &run.profile_id != profile_id
    {
        return false;
    }
    if let Some(kind) = query.kind
        && run.kind != kind
    {
        return false;
    }
    if let Some(status) = query.status
        && run.status != status
    {
        return false;
    }
    if let Some(trigger) = query.trigger
        && run.trigger != trigger
    {
        return false;
    }
    if query.running_only && run.status != TaskRunStatus::Running {
        return false;
    }
    true
}

pub fn build_task_summary(runs: &[TaskRunSummary]) -> TaskSummary {
    let mut summary = TaskSummary::default();
    for run in runs {
        summary.total_runs += 1;
        match run.status {
            TaskRunStatus::Queued => summary.queued_runs += 1,
            TaskRunStatus::Running => summary.running_runs += 1,
            TaskRunStatus::Failed => summary.failed_runs += 1,
            TaskRunStatus::Succeeded => summary.succeeded_runs += 1,
            TaskRunStatus::Skipped => summary.skipped_runs += 1,
        }

        let candidate = run.finished_at.or(run.started_at).unwrap_or(run.created_at);
        summary.last_run_at = Some(
            summary
                .last_run_at
                .map_or(candidate, |current| current.max(candidate)),
        );
    }
    summary
}

pub fn to_detail(run: TaskRunSummary, events: Vec<TaskRunEventRecord>) -> TaskRunDetail {
    TaskRunDetail {
        run,
        events: events.into_iter().map(|event| event.as_public()).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{TaskRunKind, TaskRunStage, TaskRunTrigger};

    fn sample_run(profile_id: &str, kind: TaskRunKind, status: TaskRunStatus) -> TaskRunSummary {
        TaskRunSummary {
            run_id: format!("run-{profile_id}"),
            profile_id: profile_id.to_string(),
            kind,
            trigger: TaskRunTrigger::Schedule,
            status,
            stage: TaskRunStage::Queued,
            progress_current: Some(0),
            progress_total: None,
            created_at: 1,
            started_at: None,
            finished_at: None,
            summary_json: None,
            error_code: None,
            error_message: None,
        }
    }

    #[test]
    fn matches_task_query_treats_all_profile_as_aggregate_scope() {
        let run = sample_run(
            "edge-jp",
            TaskRunKind::MetadataRefreshFull,
            TaskRunStatus::Succeeded,
        );
        let query = TaskListQuery {
            profile_id: Some("all".to_string()),
            ..TaskListQuery::default()
        };
        assert!(matches_task_query(&run, &query));
    }

    #[test]
    fn matches_task_query_applies_non_profile_filters() {
        let run = sample_run(
            "default",
            TaskRunKind::SubscriptionSync,
            TaskRunStatus::Running,
        );
        let query = TaskListQuery {
            kind: Some(TaskRunKind::MetadataRefreshIncremental),
            ..TaskListQuery::default()
        };
        assert!(!matches_task_query(&run, &query));
    }
}
