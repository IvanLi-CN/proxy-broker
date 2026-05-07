import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { Outlet, useLocation, useNavigate } from "react-router-dom";
import { toast } from "sonner";

import { AppShell } from "@/components/AppShell";
import { useProjectPreference } from "@/hooks/use-project-preference";
import { useI18n } from "@/i18n";
import { ApiError, api } from "@/lib/api";
import { resolveCurrentUserState } from "@/lib/current-user";
import { formatApiErrorMessage } from "@/lib/error-messages";
import { isGlobalProjectId } from "@/lib/project-selection";
import type { AuthMeResponse, CurrentUserState } from "@/lib/types";

export interface RootOutletContext {
  projectId: string;
  activeProjectId: string | null;
  isGlobalProject: boolean;
  projects: string[];
  authMe: AuthMeResponse | null;
  currentUser: CurrentUserState;
}

export function RootRoute() {
  const { t } = useI18n();
  const location = useLocation();
  const navigate = useNavigate();
  const [projectId, setProjectId] = useProjectPreference();
  const queryClient = useQueryClient();
  const healthQuery = useQuery({
    queryKey: ["health"],
    queryFn: api.getHealth,
    refetchInterval: 10_000,
  });
  const authMeQuery = useQuery({
    queryKey: ["auth-me"],
    queryFn: api.getAuthMe,
    refetchInterval: 30_000,
  });
  const projectsQuery = useQuery({
    queryKey: ["projects"],
    queryFn: api.listProjects,
  });
  const createProjectMutation = useMutation({
    mutationFn: (nextProjectId: string) => api.createProject({ project_id: nextProjectId }),
  });
  const projects = Array.from(
    new Set(
      [
        ...(projectsQuery.data?.projects ?? []),
        isGlobalProjectId(projectId) ? null : projectId,
      ].filter((value): value is string => Boolean(value)),
    ),
  ).sort((left, right) => left.localeCompare(right));
  const isGlobalProject = isGlobalProjectId(projectId);
  const activeProjectId = isGlobalProject ? null : projectId;
  const currentUser = resolveCurrentUserState({
    identity: authMeQuery.data ?? null,
    isLoading: authMeQuery.isLoading && !authMeQuery.data,
    error: authMeQuery.error ?? null,
  });

  useEffect(() => {
    if (!isGlobalProject || location.pathname === "/proxies") {
      return;
    }
    navigate("/proxies", { replace: true });
  }, [isGlobalProject, location.pathname, navigate]);

  const handleCreateProject = async (nextProjectId: string) => {
    try {
      const created = await createProjectMutation.mutateAsync(nextProjectId);
      setProjectId(created.project_id);
      toast.success(t("Created project {projectId}", { projectId: created.project_id }));
      await queryClient.invalidateQueries({ queryKey: ["projects"] });
      return created.project_id;
    } catch (error) {
      if (error instanceof ApiError && error.code === "project_exists") {
        const existingProjectId = nextProjectId.trim();
        toast.info(
          t("Project {projectId} already exists. Refreshing catalog.", {
            projectId: existingProjectId,
          }),
        );
        await queryClient.invalidateQueries({ queryKey: ["projects"] });
      }
      toast.error(formatApiErrorMessage(error, t));
      throw error;
    }
  };

  return (
    <AppShell
      currentUser={currentUser}
      healthStatus={healthQuery.data?.status ?? "checking"}
      onCreateProject={handleCreateProject}
      onProjectIdChange={setProjectId}
      onRetryProjects={() => {
        void projectsQuery.refetch();
      }}
      projects={projects}
      projectsCreating={createProjectMutation.isPending}
      projectsError={
        projectsQuery.isError && !projectsQuery.data
          ? formatApiErrorMessage(projectsQuery.error, t)
          : null
      }
      projectsLoading={projectsQuery.isLoading}
      projectId={projectId}
    >
      <Outlet
        context={
          {
            projectId,
            activeProjectId,
            isGlobalProject,
            projects,
            authMe: authMeQuery.data ?? null,
            currentUser,
          } satisfies RootOutletContext
        }
      />
    </AppShell>
  );
}
