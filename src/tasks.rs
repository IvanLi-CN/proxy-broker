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
        summary.last_run_at = Some(summary.last_run_at.map_or(candidate, |current| current.max(candidate)));
    }
    summary
}

pub fn to_detail(run: TaskRunSummary, events: Vec<TaskRunEventRecord>) -> TaskRunDetail {
    TaskRunDetail {
        run,
        events: events.into_iter().map(|event| event.as_public()).collect(),
    }
}
