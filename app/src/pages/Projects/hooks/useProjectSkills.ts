import { useCallback, useEffect, useState } from 'react';
import { projectsApi } from '@/api/tauri';
import type { ProjectSkill } from '@/types';

/** 单个项目内的 skill 列表与写操作。 */
export function useProjectSkills(projectId: string | null) {
  const [skills, setSkills] = useState<ProjectSkill[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!projectId) {
      setSkills([]);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      setSkills(await projectsApi.listSkills(projectId));
    } catch (e) {
      setError(String(e));
      setSkills([]);
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const installSkill = useCallback(
    async (skillId: string, targetDir?: string) => {
      if (!projectId) return '';
      const path = await projectsApi.installSkill(projectId, skillId, targetDir);
      await refresh();
      return path;
    },
    [projectId, refresh],
  );

  const importSkill = useCallback(
    async (skillPath: string) => {
      if (!projectId) return '';
      const name = await projectsApi.importSkill(projectId, skillPath);
      return name;
    },
    [projectId],
  );

  const deleteSkill = useCallback(
    async (skillPath: string, mode: 'link' | 'trash') => {
      if (!projectId) return;
      await projectsApi.deleteSkill(projectId, skillPath, mode);
      await refresh();
    },
    [projectId, refresh],
  );

  return { skills, loading, error, refresh, installSkill, importSkill, deleteSkill };
}
