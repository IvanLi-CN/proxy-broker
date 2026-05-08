import { render } from "@testing-library/react";
import type { ComponentProps } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { SessionsRoute } from "@/routes/SessionsRoute";

const {
  mockApi,
  mockQueryClient,
  mockToast,
  mockUseMutation,
  mockUseOutletContext,
  mockUseProxyOperationEvents,
  mockUseQuery,
} = vi.hoisted(() => ({
  mockApi: {
    closeSession: vi.fn(),
    getSuggestedPort: vi.fn(),
    listSessions: vi.fn(),
    openBatchByIp: vi.fn(),
    openSessionByIp: vi.fn(),
    probeProxyCatalogLatency: vi.fn(),
    searchSessionIpNodeOptions: vi.fn(),
    searchSessionNodeOptions: vi.fn(),
    updateSessionNode: vi.fn(),
  },
  mockQueryClient: {
    invalidateQueries: vi.fn(),
  },
  mockToast: {
    error: vi.fn(),
    success: vi.fn(),
  },
  mockUseMutation: vi.fn(),
  mockUseOutletContext: vi.fn(),
  mockUseProxyOperationEvents: vi.fn(),
  mockUseQuery: vi.fn(),
}));

let latestSessionsPageProps: ComponentProps<
  typeof import("@/pages/SessionsPage").SessionsPage
> | null = null;

vi.mock("@tanstack/react-query", () => ({
  useMutation: (options: unknown) => mockUseMutation(options),
  useQuery: (options: unknown) => mockUseQuery(options),
  useQueryClient: () => mockQueryClient,
}));

vi.mock("sonner", () => ({
  toast: mockToast,
}));

vi.mock("react-router-dom", () => ({
  Navigate: ({ to }: { to: string }) => <div data-testid="navigate">{to}</div>,
  useOutletContext: () => mockUseOutletContext(),
}));

vi.mock("@/hooks/use-proxy-operation-events", () => ({
  useProxyOperationEvents: (options: unknown) => mockUseProxyOperationEvents(options),
}));

vi.mock("@/i18n", () => ({
  useI18n: () => ({
    t: (message: string, values?: Record<string, string | number>) =>
      values
        ? Object.entries(values).reduce(
            (current, [key, value]) => current.replace(`{${key}}`, String(value)),
            message,
          )
        : message,
  }),
}));

vi.mock("@/lib/api", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/api")>();
  return {
    ...actual,
    api: mockApi,
  };
});

vi.mock("@/pages/SessionsPage", () => ({
  SessionsPage: (props: ComponentProps<typeof import("@/pages/SessionsPage").SessionsPage>) => {
    latestSessionsPageProps = props;
    return null;
  },
}));

describe("SessionsRoute", () => {
  beforeEach(() => {
    latestSessionsPageProps = null;
    mockQueryClient.invalidateQueries.mockReset();
    mockToast.error.mockReset();
    mockToast.success.mockReset();
    mockUseMutation.mockReset();
    mockUseOutletContext.mockReset();
    mockUseProxyOperationEvents.mockReset();
    mockUseQuery.mockReset();
    for (const mock of Object.values(mockApi)) {
      mock.mockReset();
    }

    mockUseOutletContext.mockReturnValue({
      projectId: "browser",
      activeProjectId: "browser",
      isGlobalProject: false,
    });
    mockUseMutation.mockReturnValue({
      data: null,
      error: null,
      isError: false,
      isPending: false,
      mutateAsync: vi.fn().mockResolvedValue(undefined),
      reset: vi.fn(),
      variables: null,
    });
    mockUseQuery.mockImplementation((options: { queryKey: [string, ...unknown[]] }) => {
      if (options.queryKey[0] === "sessions") {
        return {
          data: { sessions: [] },
          isLoading: false,
        };
      }
      return {
        data: { port: 20000 },
        isLoading: false,
      };
    });
    mockUseProxyOperationEvents.mockReturnValue({
      activeRunByNodeId: {},
      runByNodeId: {},
      runsById: {},
    });
    mockApi.searchSessionIpNodeOptions.mockResolvedValue({ groups: ["ip-group"] });
    mockApi.searchSessionNodeOptions.mockResolvedValue({ items: ["node-option"] });
  });

  it("keeps session option loaders stable across route rerenders", async () => {
    const { rerender } = render(<SessionsRoute />);
    const firstProps = latestSessionsPageProps;

    rerender(<SessionsRoute />);
    const secondProps = latestSessionsPageProps;

    expect(secondProps?.searchSessionIpNodeOptions).toBe(firstProps?.searchSessionIpNodeOptions);
    expect(secondProps?.searchSessionNodeOptions).toBe(firstProps?.searchSessionNodeOptions);

    await expect(
      secondProps?.searchSessionNodeOptions("sess-egC6gRmBLS0rFffF", {
        sort_mode: "session_recent",
      }),
    ).resolves.toEqual(["node-option"]);
    expect(mockApi.searchSessionNodeOptions).toHaveBeenCalledWith(
      "browser",
      "sess-egC6gRmBLS0rFffF",
      {
        sort_mode: "session_recent",
      },
    );
  });
});
