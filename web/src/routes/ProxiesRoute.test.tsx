import { render } from "@testing-library/react";
import type { ComponentProps } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ProxiesRoute } from "@/routes/ProxiesRoute";

const { mockOutletContext, mockUseMutation, mockUseQuery, mockUseQueryClient } = vi.hoisted(() => ({
  mockOutletContext: vi.fn(),
  mockUseMutation: vi.fn(),
  mockUseQuery: vi.fn(),
  mockUseQueryClient: vi.fn(),
}));

let latestProxiesPageProps: ComponentProps<
  typeof import("@/pages/ProxiesPage").ProxiesPage
> | null = null;
let observedQueryOptions: Array<{ queryKey?: unknown; enabled?: boolean }> = [];

vi.mock("@tanstack/react-query", () => ({
  useMutation: () => mockUseMutation(),
  useQuery: (options: unknown) => mockUseQuery(options),
  useQueryClient: () => mockUseQueryClient(),
}));

vi.mock("react-router-dom", () => ({
  useOutletContext: () => mockOutletContext(),
}));

vi.mock("@/pages/ProxiesPage", () => ({
  ProxiesPage: (props: ComponentProps<typeof import("@/pages/ProxiesPage").ProxiesPage>) => {
    latestProxiesPageProps = props;
    return null;
  },
}));

vi.mock("sonner", () => ({
  toast: {
    error: vi.fn(),
    success: vi.fn(),
  },
}));

describe("ProxiesRoute", () => {
  beforeEach(() => {
    latestProxiesPageProps = null;
    observedQueryOptions = [];
    mockOutletContext.mockReset();
    mockUseMutation.mockReset();
    mockUseQuery.mockReset();
    mockUseQueryClient.mockReset();

    mockUseQueryClient.mockReturnValue({
      invalidateQueries: vi.fn().mockResolvedValue(undefined),
      setQueryData: vi.fn(),
    });
    mockUseMutation.mockReturnValue({
      isPending: false,
      isError: false,
      mutateAsync: vi.fn(),
      variables: undefined,
      error: null,
    });
    mockUseQuery.mockImplementation((options) => {
      observedQueryOptions.push({
        queryKey: options?.queryKey,
        enabled: options?.enabled,
      });
      return {
        data: null,
        error: null,
        isError: false,
        isLoading: false,
      };
    });
  });

  it("shows access denied when the current user is anonymous", () => {
    mockOutletContext.mockReturnValue({
      projectId: "__global__",
      activeProjectId: null,
      isGlobalProject: true,
      projects: ["default"],
      authMe: null,
      currentUser: { status: "anonymous" },
    });

    render(<ProxiesRoute />);

    expect(latestProxiesPageProps?.mode).toBe("global");
    if (latestProxiesPageProps?.mode !== "global") {
      throw new Error("expected global proxies page");
    }
    expect(latestProxiesPageProps.accessDenied).toBe(true);
  });

  it("surfaces auth failures without mislabeling them as access denied", () => {
    mockOutletContext.mockReturnValue({
      projectId: "__global__",
      activeProjectId: null,
      isGlobalProject: true,
      projects: ["default"],
      authMe: null,
      currentUser: { status: "error", message: "auth_unavailable: upstream timeout" },
    });

    render(<ProxiesRoute />);

    expect(latestProxiesPageProps?.mode).toBe("global");
    if (latestProxiesPageProps?.mode !== "global") {
      throw new Error("expected global proxies page");
    }
    expect(latestProxiesPageProps.accessDenied).toBe(false);
    expect(latestProxiesPageProps.authError).toBe("auth_unavailable: upstream timeout");
  });

  it("passes the current project list through to the global page", () => {
    mockOutletContext.mockReturnValue({
      projectId: "__global__",
      activeProjectId: null,
      isGlobalProject: true,
      projects: ["default", "edge-jp", "lab-us"],
      authMe: { is_admin: true },
      currentUser: { status: "resolved", identity: { is_admin: true } },
    });

    render(<ProxiesRoute />);

    expect(latestProxiesPageProps?.mode).toBe("global");
    if (latestProxiesPageProps?.mode !== "global") {
      throw new Error("expected global proxies page");
    }
    expect(latestProxiesPageProps.projects).toEqual(["default", "edge-jp", "lab-us"]);
  });

  it("switches the proxies page into project mode for a normal project", () => {
    mockOutletContext.mockReturnValue({
      projectId: "edge-jp",
      activeProjectId: "edge-jp",
      isGlobalProject: false,
      projects: ["default", "edge-jp", "lab-us"],
      authMe: { is_admin: true },
      currentUser: { status: "resolved", identity: { is_admin: true } },
    });

    render(<ProxiesRoute />);

    expect(latestProxiesPageProps?.mode).toBe("project");
    expect(latestProxiesPageProps).toMatchObject({
      projectId: "edge-jp",
      showProxyPolicy: true,
    });
  });

  it("loads the project catalog for non-admin users on project projects", () => {
    mockOutletContext.mockReturnValue({
      projectId: "edge-jp",
      activeProjectId: "edge-jp",
      isGlobalProject: false,
      projects: ["default", "edge-jp"],
      authMe: { is_admin: false },
      currentUser: {
        status: "resolved",
        identity: { is_admin: false },
      },
    });

    render(<ProxiesRoute />);

    expect(observedQueryOptions).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          queryKey: ["proxy-catalog", "project", "edge-jp"],
          enabled: true,
        }),
        expect.objectContaining({
          queryKey: ["suggested-port", "edge-jp"],
          enabled: true,
        }),
      ]),
    );
  });
});
