import { render } from "@testing-library/react";
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

vi.mock("@tanstack/react-query", () => ({
  useQuery: () => mockUseQuery(),
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
    mockOutletContext.mockReset();
    mockUseQuery.mockReset();
    mockUseTaskEvents.mockReset();

    mockUseQuery.mockReturnValue({
      data: null,
      error: null,
      isError: false,
      isLoading: false,
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
});
