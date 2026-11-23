import { useCallback, useEffect, useState } from 'react';
import { mcpApi } from '@/api/tauri';
import type { McpOverview, McpServerDef } from '@/types';

/** 全局 MCP 总览与写操作。 */
export function useMcpServers() {
  const [overview, setOverview] = useState<McpOverview | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setOverview(await mcpApi.list());
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const save = useCallback(
    async (server: McpServerDef, targets: string[]) => {
      await mcpApi.save(server, targets);
      await refresh();
    },
    [refresh],
  );

  const remove = useCallback(
    async (name: string, targets: string[]) => {
      await mcpApi.remove(name, targets);
      await refresh();
    },
    [refresh],
  );

  const setEnabled = useCallback(
    async (name: string, enabled: boolean, targets: string[]) => {
      await mcpApi.setEnabled(name, enabled, targets);
      await refresh();
    },
    [refresh],
  );

  const sync = useCallback(
    async (name: string, fromAgent: string, toAgents: string[]) => {
      await mcpApi.sync(name, fromAgent, toAgents);
      await refresh();
    },
    [refresh],
  );

  return { overview, loading, error, refresh, save, remove, setEnabled, sync };
}
