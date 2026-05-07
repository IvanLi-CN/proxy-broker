import { render } from "@testing-library/react";
import type { ComponentProps } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ApiError } from "@/lib/api";
import { RootRoute } from "@/routes/RootRoute";

const {
  mockToast,
  mockUseLocation,
  mockUseMutation,
  mockUseNavigate,
  mockUseProjectPreference,
  mockUseQuery,
  mockUseQueryClient,
} = vi.hoisted(() => ({
  mockToast: {
    error: vi.fn(),
    info: vi.fn(),
    success: vi.fn(),
  },
  mockUseLocation: vi.fn(),
  mockUseMutation: vi.fn(),
  mockUseNavigate: vi.fn(),
  mockUseProjectPreference: vi.fn(),
  mockUseQuery: vi.fn(),
  mockUseQueryClient: vi.fn(),
}));

let latestAppShellProps: ComponentProps<typeof import("@/components/AppShell").AppShell> | null =
  null;

vi.mock("@tanstack/react-query", () => ({
  useMutation: () => mockUseMutation(),
  useQuery: () => mockUseQuery(),
  useQueryClient: () => mockUseQueryClient(),
}));

vi.mock("sonner", () => ({
  toast: mockToast,
}));

vi.mock("@/hooks/use-project-preference", () => ({
  useProjectPreference: () => mockUseProjectPreference(),
}));

vi.mock("@/components/AppShell", () => ({
  AppShell: (props: ComponentProps<typeof import("@/components/AppShell").AppShell>) => {
    latestAppShellProps = props;
    return null;
  },
}));

vi.mock("react-router-dom", () => ({
  Outlet: () => null,
  useLocation: () => mockUseLocation(),
  useNavigate: () => mockUseNavigate(),
}));

describe("RootRoute", () => {
  beforeEach(() => {
    latestAppShellProps = null;
    mockUseMutation.mockReset();
    mockUseNavigate.mockReset();
    mockUseProjectPreference.mockReset();
    mockUseQuery.mockReset();
    mockUseQueryClient.mockReset();
    mockUseLocation.mockReset();
    mockToast.error.mockReset();
    mockToast.info.mockReset();
    mockToast.success.mockReset();

    mockUseProjectPreference.mockReturnValue(["default", vi.fn()]);
    mockUseLocation.mockReturnValue({ pathname: "/" });
    mockUseNavigate.mockReturnValue(vi.fn());
    mockUseQueryClient.mockReturnValue({
      invalidateQueries: vi.fn().mockResolvedValue(undefined),
    });
    mockUseMutation.mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    });
  });

  it("keeps cached projects visible when a background refetch fails", () => {
    mockUseQuery
      .mockReturnValueOnce({
        data: { status: "healthy" },
      })
      .mockReturnValueOnce({
        data: {
          authenticated: true,
          principal_type: "human",
          subject: "admin@example.com",
          email: "admin@example.com",
          groups: ["admins"],
          is_admin: true,
        },
      })
      .mockReturnValueOnce({
        data: { projects: ["default", "edge-jp"] },
        error: new ApiError(500, {
          code: "http_500",
          message: "Projects temporarily unavailable",
        }),
        isError: true,
        isLoading: false,
        refetch: vi.fn(),
      });

    render(<RootRoute />);

    expect(latestAppShellProps?.projects).toEqual(["default", "edge-jp"]);
    expect(latestAppShellProps?.projectsError).toBeNull();
  });

  it("keeps the active project unchanged when create returns project_exists", async () => {
    const setProjectId = vi.fn();
    const invalidateQueries = vi.fn().mockResolvedValue(undefined);
    const duplicateError = new ApiError(409, {
      code: "project_exists",
      message: "Project already exists",
    });

    mockUseProjectPreference.mockReturnValue(["default", setProjectId]);
    mockUseQueryClient.mockReturnValue({ invalidateQueries });
    mockUseQuery
      .mockReturnValueOnce({
        data: { status: "healthy" },
      })
      .mockReturnValueOnce({
        data: {
          authenticated: true,
          principal_type: "human",
          subject: "admin@example.com",
          email: "admin@example.com",
          groups: ["admins"],
          is_admin: true,
        },
      })
      .mockReturnValueOnce({
        data: { projects: ["default", "edge-jp"] },
        isError: false,
        isLoading: false,
        refetch: vi.fn(),
      });
    mockUseMutation.mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn().mockRejectedValue(duplicateError),
    });

    render(<RootRoute />);

    await expect(latestAppShellProps?.onCreateProject("  edge-jp  ")).rejects.toBe(duplicateError);
    expect(setProjectId).not.toHaveBeenCalled();
    expect(invalidateQueries).toHaveBeenCalledWith({ queryKey: ["projects"] });
    expect(mockToast.info).toHaveBeenCalledWith(
      "Project edge-jp already exists. Refreshing catalog.",
    );
    expect(mockToast.error).toHaveBeenCalledWith("project_exists: Project already exists");
  });

  it("maps auth 401 into an anonymous current-user state", () => {
    mockUseQuery
      .mockReturnValueOnce({
        data: { status: "healthy" },
      })
      .mockReturnValueOnce({
        data: undefined,
        error: new ApiError(401, {
          code: "authentication_required",
          message: "authentication required",
        }),
        isError: true,
        isLoading: false,
      })
      .mockReturnValueOnce({
        data: { projects: ["default"] },
        isError: false,
        isLoading: false,
        refetch: vi.fn(),
      });

    render(<RootRoute />);

    expect(latestAppShellProps?.currentUser).toEqual({
      status: "anonymous",
    });
  });

  it("redirects persisted global selection back to /proxies", () => {
    const navigate = vi.fn();

    mockUseNavigate.mockReturnValue(navigate);
    mockUseProjectPreference.mockReturnValue(["__global__", vi.fn()]);
    mockUseLocation.mockReturnValue({ pathname: "/" });
    mockUseQuery
      .mockReturnValueOnce({
        data: { status: "healthy" },
      })
      .mockReturnValueOnce({
        data: {
          authenticated: true,
          principal_type: "human",
          subject: "admin@example.com",
          email: "admin@example.com",
          groups: ["admins"],
          is_admin: true,
        },
      })
      .mockReturnValueOnce({
        data: { projects: ["default", "edge-jp"] },
        isError: false,
        isLoading: false,
        refetch: vi.fn(),
      });

    render(<RootRoute />);

    expect(navigate).toHaveBeenCalledWith("/proxies", { replace: true });
  });
});
