import type { TaskListQuery, TaskRunSummary, TaskStreamEnvelope, TaskSummary } from "@/lib/types";

export function buildTaskSearchParams(query: TaskListQuery) {
  const params = new URLSearchParams();

  if (query.profile_id) {
    params.set("profile_id", query.profile_id);
  }
  if (query.kind) {
    params.set("kind", query.kind);
  }
  if (query.status) {
    params.set("status", query.status);
  }
  if (query.trigger) {
    params.set("trigger", query.trigger);
  }
  if (query.running_only) {
    params.set("running_only", "true");
  }
  if (query.since != null) {
    params.set("since", String(query.since));
  }
  if (query.limit != null) {
    params.set("limit", String(query.limit));
  }
  if (query.cursor) {
    params.set("cursor", query.cursor);
  }

  return params;
}

export function matchesTaskQuery(run: TaskRunSummary, query: TaskListQuery) {
  if (query.profile_id && run.profile_id !== query.profile_id) {
    return false;
  }
  if (query.kind && run.kind !== query.kind) {
    return false;
  }
  if (query.status && run.status !== query.status) {
    return false;
  }
  if (query.trigger && run.trigger !== query.trigger) {
    return false;
  }
  if (query.running_only && run.status !== "running") {
    return false;
  }
  if (query.since != null && run.created_at < query.since) {
    return false;
  }
  return true;
}

export function sortTaskRuns(runs: TaskRunSummary[]) {
  return [...runs].sort((left, right) => {
    if (right.created_at !== left.created_at) {
      return right.created_at - left.created_at;
    }
    return right.run_id.localeCompare(left.run_id);
  });
}

export function summarizeTaskRuns(runs: TaskRunSummary[]): TaskSummary {
  const summary: TaskSummary = {
    total_runs: 0,
    queued_runs: 0,
    running_runs: 0,
    failed_runs: 0,
    succeeded_runs: 0,
    skipped_runs: 0,
    last_run_at: null,
  };

  for (const run of runs) {
    summary.total_runs += 1;
    if (run.status === "queued") {
      summary.queued_runs += 1;
    }
    if (run.status === "running") {
      summary.running_runs += 1;
    }
    if (run.status === "failed") {
      summary.failed_runs += 1;
    }
    if (run.status === "succeeded") {
      summary.succeeded_runs += 1;
    }
    if (run.status === "skipped") {
      summary.skipped_runs += 1;
    }

    const candidate = run.finished_at ?? run.started_at ?? run.created_at;
    summary.last_run_at = Math.max(summary.last_run_at ?? 0, candidate);
  }

  return summary;
}

export function parseTaskEnvelope<T>(payload: string): TaskStreamEnvelope<T> {
  return JSON.parse(payload) as TaskStreamEnvelope<T>;
}
