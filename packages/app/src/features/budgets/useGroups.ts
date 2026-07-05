import { useState, useEffect, useCallback } from "react";
import type { GroupWithMembers, RawGroupWithMembers, Group, RawGroup, BudgetBasis } from "./types";
import { normalizeGroupWithMembers, normalizeGroup } from "./types";

/**
 * Safely invoke a Tauri command. Returns null in non-Tauri (e.g. jsdom/test) environments.
 */
async function safeTauriInvoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<T>(command, args);
  } catch {
    return null;
  }
}

interface UseGroupsResult {
  groups: GroupWithMembers[];
  projectNames: string[];
  loading: boolean;
  error: string | null;
  refresh: () => void;
  createGroup: (
    name: string,
    budgetBasis: BudgetBasis | null,
    budgetValue: number | null,
  ) => Promise<Group | null>;
  updateGroup: (
    id: number,
    name: string,
    budgetBasis: BudgetBasis | null,
    budgetValue: number | null,
  ) => Promise<void>;
  deleteGroup: (id: number) => Promise<void>;
  assignProject: (projectName: string, groupId: number) => Promise<void>;
  unassignProject: (projectName: string) => Promise<void>;
}

/**
 * Hook that manages CRUD operations on project groups.
 * Automatically refreshes state after each mutation.
 */
export function useGroups(): UseGroupsResult {
  const [groups, setGroups] = useState<GroupWithMembers[]>([]);
  const [projectNames, setProjectNames] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchAll = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [rawGroups, rawProjectNames] = await Promise.all([
        safeTauriInvoke<RawGroupWithMembers[]>("list_groups"),
        safeTauriInvoke<string[]>("list_project_names"),
      ]);

      if (rawGroups !== null) {
        setGroups(rawGroups.map(normalizeGroupWithMembers));
      }
      if (rawProjectNames !== null) {
        setProjectNames(rawProjectNames);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unknown error");
    } finally {
      setLoading(false);
    }
  }, []);

  // Fetch on mount
  useEffect(() => {
    void fetchAll();
  }, [fetchAll]);

  const createGroup = useCallback(
    async (
      name: string,
      budgetBasis: BudgetBasis | null,
      budgetValue: number | null,
    ): Promise<Group | null> => {
      const raw = await safeTauriInvoke<RawGroup>("create_group", {
        name,
        budgetBasis,
        budgetValue,
      });
      await fetchAll();
      return raw !== null ? normalizeGroup(raw) : null;
    },
    [fetchAll],
  );

  const updateGroup = useCallback(
    async (
      id: number,
      name: string,
      budgetBasis: BudgetBasis | null,
      budgetValue: number | null,
    ): Promise<void> => {
      await safeTauriInvoke<void>("update_group", {
        id,
        name,
        budgetBasis,
        budgetValue,
      });
      await fetchAll();
    },
    [fetchAll],
  );

  const deleteGroup = useCallback(
    async (id: number): Promise<void> => {
      await safeTauriInvoke<void>("delete_group", { id });
      await fetchAll();
    },
    [fetchAll],
  );

  const assignProject = useCallback(
    async (projectName: string, groupId: number): Promise<void> => {
      await safeTauriInvoke<void>("assign_project", {
        projectName,
        groupId,
      });
      await fetchAll();
    },
    [fetchAll],
  );

  const unassignProject = useCallback(
    async (projectName: string): Promise<void> => {
      await safeTauriInvoke<void>("unassign_project", {
        projectName,
      });
      await fetchAll();
    },
    [fetchAll],
  );

  return {
    groups,
    projectNames,
    loading,
    error,
    refresh: fetchAll,
    createGroup,
    updateGroup,
    deleteGroup,
    assignProject,
    unassignProject,
  };
}
