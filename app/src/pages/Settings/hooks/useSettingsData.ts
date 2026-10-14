import { useRequest } from 'ahooks';
import { agentsApi } from '@/api/tauri';
import { registerDynamicAgents } from '@/constants/agents';
import type { AgentConfig } from '@/types';

export const useSettingsData = () => {
  const { data: agents = [] as AgentConfig[], run: loadAgents } = useRequest(
    async () => {
      const agentsData = await agentsApi.detect();
      registerDynamicAgents(agentsData);
      return agentsData;
    },
    {
      onError: (error) => console.error('Failed to detect agents:', error),
    },
  );

  return { agents, loadAgents };
};
