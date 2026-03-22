import { act, render, waitFor } from "@testing-library/react";
import type { ComponentProps } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { TasksRoute } from "@/routes/TasksRoute";

const { mockOutletContext, mockUseQuery, mockUseTaskEvents } = vi.hoisted(() => ({
  mockOutletContext: vi.fn(),
  mockUseQuery: vi.fn(),
  mockUseTaskEvents: vi.fn(),
}));

let latestTasksPageProps: ComponentProps<typeof import("@/pages/TasksPage").TasksPage> | null =
  null;
let tasksQueryResult: {
  data: unknown;
  error: unknown;
  isError: boolean;
  isLoading: boolean;
};
let detailQueryResult: {
  data: unknown;
  error: unknown;
  isError: boolean;
  isLoading: boolean;
};
let latestTasksQueryKey: [string, ...unknown[]] | null = null;

vi.mock("@tanstack/react-query", () => ({
  useQuery: (options: unknown) => mockUseQuery(options),
}));

vi.mock("react-router-dom", () => ({
  useOutletContext: () => mockOutletContext(),
}));

vi.mock("@/hooks/use-task-events", () => ({
  useTaskEvents: () => mockUseTaskEvents(),
}));

vi.mock("@/pages/TasksPage", () => ({
  TasksPage: (props: ComponentProps<typeof import("@/pages/TasksPage").TasksPage>) => {
    latestTasksPageProps = props;
    return null;
  },
}));

describe("TasksRoute", () => {
  beforeEach(() => {
    latestTasksPageProps = null;
    latestTasksQueryKey = null;
    mockOutletContext.mockReset();
    mockUseQuery.mockReset();
    mockUseTaskEvents.mockReset();

    tasksQueryResult = {
      data: null,
      error: null,
      isError: false,
      isLoading: false,
    };
    detailQueryResult = {
      data: null,
      error: null,
      isError: false,
      isLoading: false,
    };
    mockUseQuery.mockImplementation((options: { queryKey: [string, ...unknown[]] }) => {
      if (options.queryKey[0] === "tasks") {
        latestTasksQueryKey = options.queryKey;
        return tasksQueryResult;
      }
      return detailQueryResult;
    });
    mockUseTaskEvents.mockReturnValue("connecting");
  });

  it("shows access denied when the current user is anonymous", () => {
    mockOutletContext.mockReturnValue({
      profileId: "default",
      authMe: null,
      currentUser: { status: "anonymous" },
    });

    render(<TasksRoute />);

    expect(latestTasksPageProps?.accessDenied).toBe(true);
  });

  it("keeps access denied hidden while auth state is still loading", () => {
    mockOutletContext.mockReturnValue({
      profileId: "default",
      authMe: null,
      currentUser: { status: "loading" },
    });

    render(<TasksRoute />);

    expect(latestTasksPageProps?.accessDenied).toBe(false);
  });

  it("surfaces auth resolution failures without mislabeling them as access denied", () => {
    mockOutletContext.mockReturnValue({
      profileId: "default",
      authMe: null,
      currentUser: { status: "error", message: "auth_unavailable: upstream timeout" },
    });

    render(<TasksRoute />);

    expect(latestTasksPageProps?.accessDenied).toBe(false);
    expect(latestTasksPageProps?.taskError).toBe("auth_unavailable: upstream timeout");
    expect(latestTasksPageProps?.streamState).toBe("reconnecting");
  });

  it("scopes the default task query to the current profile and last 7 days", () => {
    vi.useFakeTimers();
    try {
      vi.setSystemTime(new Date("2026-03-23T00:00:00Z"));
      mockOutletContext.mockReturnValue({
        profileId: "default",
        authMe: { is_admin: true },
        currentUser: { status: "resolved", identity: { is_admin: true } },
      });

      render(<TasksRoute />);

      expect(latestTasksQueryKey).not.toBeNull();
      expect(latestTasksQueryKey?.[1]).toMatchObject({
        profile_id: "default",
        since: 1773619200,
      });
    } finally {
      vi.useRealTimers();
    }
  });

  it("slides the default task query window forward while the board stays open", () => {
    vi.useFakeTimers();
    try {
      vi.setSystemTime(new Date("2026-03-23T00:00:00Z"));
      mockOutletContext.mockReturnValue({
        profileId: "default",
        authMe: { is_admin: true },
        currentUser: { status: "resolved", identity: { is_admin: true } },
      });

      render(<TasksRoute />);

      expect(latestTasksQueryKey?.[1]).toMatchObject({
        profile_id: "default",
        since: 1773619200,
      });

      act(() => {
        vi.advanceTimersByTime(60_000);
      });

      expect(latestTasksQueryKey?.[1]).toMatchObject({
        profile_id: "default",
        since: 1773619260,
      });
    } finally {
      vi.useRealTimers();
    }
  });

  it("clears the selected run while a new task query is loading", async () => {
    const run = {
      run_id: "run-1",
      profile_id: "default",
      kind: "subscription_sync",
      trigger: "schedule",
      status: "running",
      stage: "probing",
      progress_current: 1,
      progress_total: 2,
      created_at: 1,
      started_at: 1,
      finished_at: null,
      summary_json: null,
      error_code: null,
      error_message: null,
    };
    let outletContext = {
      profileId: "default",
      authMe: { is_admin: true },
      currentUser: { status: "authenticated" },
    };
    mockOutletContext.mockImplementation(() => outletContext);
    tasksQueryResult = {
      data: {
        summary: {
          total_runs: 1,
          queued_runs: 0,
          running_runs: 1,
          failed_runs: 0,
          succeeded_runs: 0,
          skipped_runs: 0,
          last_run_at: 1,
        },
        runs: [run],
        next_cursor: null,
      },
      error: null,
      isError: false,
      isLoading: false,
    };
    detailQueryResult = {
      data: { run, events: [] },
      error: null,
      isError: false,
      isLoading: false,
    };

    const view = render(<TasksRoute />);

    await waitFor(() => expect(latestTasksPageProps?.selectedRunId).toBe("run-1"));

    outletContext = {
      ...outletContext,
      profileId: "other",
    };
    tasksQueryResult = {
      data: null,
      error: null,
      isError: false,
      isLoading: true,
    };
    detailQueryResult = {
      data: null,
      error: null,
      isError: false,
      isLoading: false,
    };

    view.rerender(<TasksRoute />);

    await waitFor(() => expect(latestTasksPageProps?.selectedRunId).toBe(null));
  });
});
