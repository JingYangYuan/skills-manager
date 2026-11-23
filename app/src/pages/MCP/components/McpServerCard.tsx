import { useTranslation } from 'react-i18next';
import { Icon } from '@/components/Icon';
import { getAgentDisplayName, getAgentIconUrl } from '@/constants';
import type { McpServerAggregate } from '@/types';

interface McpServerCardProps {
  server: McpServerAggregate;
  onSelect: (server: McpServerAggregate) => void;
}

/** MCP server 卡片：名称、传输方式、绑定 agent、冲突提示。 */
export function McpServerCard({ server, onSelect }: McpServerCardProps) {
  const { t } = useTranslation();
  const { definition } = server;
  const summary =
    definition.transport === 'stdio'
      ? [definition.command, ...definition.args].filter(Boolean).join(' ')
      : definition.url ?? '';

  return (
    <button
      type="button"
      onClick={() => onSelect(server)}
      className="w-full text-left p-4 rounded-2xl border border-[var(--border-color)] dark:border-dark-border bg-[var(--bg-card)] dark:bg-dark-bg-card hover:shadow-[0_18px_40px_-30px_rgba(39,50,72,0.7)] transition-shadow"
    >
      <div className="flex items-start gap-3">
        <div className="w-9 h-9 rounded-xl bg-[var(--accent-primary-soft)] flex items-center justify-center shrink-0">
          <Icon name="hub" className="text-lg text-[var(--accent-primary)]" />
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <span className="font-bold text-[var(--text-primary)] dark:text-white truncate">
              {server.name}
            </span>
            {server.conflict && (
              <span className="text-[10px] font-bold px-2 py-0.5 rounded-full bg-amber-50 dark:bg-amber-500/10 text-amber-600 dark:text-amber-400 whitespace-nowrap">
                {t('mcp.conflict')}
              </span>
            )}
            {!definition.enabled && (
              <span className="text-[10px] font-bold px-2 py-0.5 rounded-full bg-gray-100 dark:bg-gray-500/10 text-gray-500 dark:text-gray-400 whitespace-nowrap">
                {t('common.disabled')}
              </span>
            )}
          </div>
          <p className="mt-0.5 text-xs font-mono text-[var(--text-tertiary)] truncate" title={summary}>
            {summary || '—'}
          </p>
        </div>
        <span className="text-[10px] font-bold px-2 py-0.5 rounded-full bg-black/5 dark:bg-white/10 text-[var(--text-secondary)] uppercase whitespace-nowrap">
          {definition.transport}
        </span>
      </div>

      <div className="mt-3 flex flex-wrap items-center gap-1.5">
        {server.bindings.map((binding) => (
          <span
            key={binding.agent}
            className="inline-flex items-center gap-1 text-[10px] font-semibold px-2 py-0.5 rounded-full bg-black/5 dark:bg-white/10 text-[var(--text-secondary)]"
            title={binding.config_path}
          >
            <img
              src={getAgentIconUrl(binding.agent)}
              alt=""
              className="w-3.5 h-3.5 rounded-sm object-contain"
            />
            {getAgentDisplayName(binding.agent)}
          </span>
        ))}
      </div>
    </button>
  );
}
