import { useCallback, useEffect, useState } from 'react';
import { projectsApi } from '@/api/tauri';
import type { ProjectCandidate, ProjectConfig } from '@/types';

/** 项目注册表：列表、增删、自动发现候选。 */
export function useProjects() {
  const [projects, setProjects] = useState<ProjectConfig[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setProjects(await projectsApi.list());
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const addProject = useCallback(
    async (path: string, name?: string) => {
      const project = await projectsApi.add(path, name);
      await refresh();
      return project;
    },
    [refresh],
  );

  const removeProject = useCallback(
    async (id: string) => {
      await projectsApi.remove(id);
      await refresh();
    },
    [refresh],
  );

  const detectCandidates = useCallback(async (): Promise<ProjectCandidate[]> => {
    return projectsApi.detectCandidates();
  }, []);

  return { projects, loading, error, refresh, addProject, removeProject, detectCandidates };
}
