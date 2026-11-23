import { useTranslation } from 'react-i18next';
import { Icon } from '@/components/Icon';

export type MarketplaceKind = 'skill' | 'mcp';

interface MarketplaceKindSwitchProps {
  kind: MarketplaceKind;
  onChange: (kind: MarketplaceKind) => void;
}

const OPTIONS: Array<{ id: MarketplaceKind; icon: string; labelKey: string }> = [
  { id: 'skill', icon: 'extension', labelKey: 'marketplace.kind.skill' },
  { id: 'mcp', icon: 'hub', labelKey: 'marketplace.kind.mcp' },
];

/** 市场类型切换：技能 / MCP。 */
export function MarketplaceKindSwitch({ kind, onChange }: MarketplaceKindSwitchProps) {
  const { t } = useTranslation();
  const index = Math.max(0, OPTIONS.findIndex((option) => option.id === kind));

  return (
    <div
      role="tablist"
      aria-label={t('marketplace.title')}
      className="relative grid grid-cols-2 items-center rounded-lg border border-[var(--border-color)] dark:border-dark-border bg-[var(--shell-surface-muted)] dark:bg-dark-bg-tertiary p-0.5 h-9 shrink-0"
    >
      <div
        className="absolute top-0.5 bottom-0.5 rounded-md bg-[var(--bg-card)] dark:bg-dark-bg-card shadow-sm transition-all duration-200 ease-in-out"
        style={{
          left: `calc(${index} * (100% - 4px) / 2 + 2px)`,
          width: 'calc((100% - 4px) / 2)',
        }}
      />
      {OPTIONS.map((option) => {
        const active = option.id === kind;
        return (
          <button
            key={option.id}
            type="button"
            role="tab"
            aria-selected={active}
            onClick={() => onChange(option.id)}
            className="relative z-10 flex items-center justify-center gap-1.5 px-3 h-full rounded-md transition-colors"
          >
            <Icon
              name={option.icon}
              className={`text-base transition-colors ${
                active ? 'text-[var(--accent-primary)]' : 'text-slate-400 dark:text-gray-500'
              }`}
            />
            <span
              className={`text-xs font-bold whitespace-nowrap transition-colors ${
                active ? 'text-slate-700 dark:text-white' : 'text-slate-400 dark:text-gray-500'
              }`}
            >
              {t(option.labelKey)}
            </span>
          </button>
        );
      })}
    </div>
  );
}
