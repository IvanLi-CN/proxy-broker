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

vi.mock("@tanstack/react-query", () => ({
  useMutation: () => mockUseMutation(),
  useQuery: () => mockUseQuery(),
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
    mockUseQuery.mockReturnValue({
      data: null,
      error: null,
      isError: false,
      isLoading: false,
    });
  });

  it("shows access denied when the current user is anonymous", () => {
    mockOutletContext.mockReturnValue({
      profileId: "__global__",
      activeProfileId: null,
      isGlobalConfig: true,
      profiles: ["default"],
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
      profileId: "__global__",
      activeProfileId: null,
      isGlobalConfig: true,
      profiles: ["default"],
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

  it("passes the current profile list through to the global page", () => {
    mockOutletContext.mockReturnValue({
      profileId: "__global__",
      activeProfileId: null,
      isGlobalConfig: true,
      profiles: ["default", "edge-jp", "lab-us"],
      authMe: { is_admin: true },
      currentUser: { status: "resolved", identity: { is_admin: true } },
    });

    render(<ProxiesRoute />);

    expect(latestProxiesPageProps?.mode).toBe("global");
    if (latestProxiesPageProps?.mode !== "global") {
      throw new Error("expected global proxies page");
    }
    expect(latestProxiesPageProps.profiles).toEqual(["default", "edge-jp", "lab-us"]);
  });

  it("switches the proxies page into profile mode for a normal config", () => {
    mockOutletContext.mockReturnValue({
      profileId: "edge-jp",
      activeProfileId: "edge-jp",
      isGlobalConfig: false,
      profiles: ["default", "edge-jp", "lab-us"],
      authMe: { is_admin: true },
      currentUser: { status: "resolved", identity: { is_admin: true } },
    });

    render(<ProxiesRoute />);

    expect(latestProxiesPageProps?.mode).toBe("profile");
    expect(latestProxiesPageProps).toMatchObject({
      profileId: "edge-jp",
      showProxyPolicy: true,
    });
  });
});
