import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Icon } from '@/components/Icon';
import type { McpMarketplaceItem } from '@/types';

interface McpMarketIconProps {
  item: Pick<McpMarketplaceItem, 'title' | 'iconUrl'>;
  size?: 'sm' | 'md' | 'lg';
}

/** Registry 图标可能是远程 URL；加载失败时退化为首字母。 */
export function McpMarketIcon({ item, size = 'md' }: McpMarketIconProps) {
  const [failed, setFailed] = useState(false);
  const sizeClass =
    size === 'lg'
      ? 'w-14 h-14 rounded-2xl text-xl'
      : size === 'sm'
        ? 'w-8 h-8 rounded-lg text-sm'
        : 'w-10 h-10 rounded-xl text-base';

  if (item.iconUrl && !failed) {
    return (
      <span
        className={`${sizeClass} flex items-center justify-center overflow-hidden bg-white dark:bg-dark-bg-card border border-[var(--border-color)] dark:border-dark-border shrink-0`}
      >
        <img
          src={item.iconUrl}
          alt=""
          loading="lazy"
          referrerPolicy="no-referrer"
          onError={() => setFailed(true)}
          className="w-full h-full object-contain"
        />
      </span>
    );
  }

  const initial = item.title.trim().slice(0, 1).toUpperCase() || 'M';
  return (
    <span
      className={`${sizeClass} flex items-center justify-center shrink-0 bg-[var(--accent-primary-soft)] text-[var(--accent-primary)] font-black`}
    >
      {initial}
    </span>
  );
}

export function McpKindChips({ kinds }: { kinds: string[] }) {
  const { t } = useTranslation();
  if (kinds.length === 0) return null;
  return (
    <>
      {kinds.map((kind) => (
        <span
          key={kind}
          className="text-[10px] font-bold px-2 py-0.5 rounded-full bg-black/5 dark:bg-white/10 text-[var(--text-secondary)] uppercase"
        >
          {t(`mcpMarket.installKinds.${kind}`, kind)}
        </span>
      ))}
    </>
  );
}

interface McpMarketCardProps {
  item: McpMarketplaceItem;
  installed: boolean;
  onSelect: (item: McpMarketplaceItem) => void;
}

/** 市场列表卡片。 */
export function McpMarketCard({ item, installed, onSelect }: McpMarketCardProps) {
  const { t } = useTranslation();

  return (
    <button
      type="button"
      onClick={() => onSelect(item)}
      className="w-full text-left p-4 rounded-2xl border border-[var(--border-color)] dark:border-dark-border bg-[var(--bg-card)] dark:bg-dark-bg-card hover:shadow-[0_18px_40px_-30px_rgba(39,50,72,0.7)] hover:border-[var(--shell-line-strong)] dark:hover:border-gray-600 transition-all"
    >
      <div className="flex items-start gap-3">
        <McpMarketIcon item={item} />
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <span className="font-bold text-[var(--text-primary)] dark:text-white truncate">
              {item.title}
            </span>
            {installed && (
              <span className="text-[10px] font-bold px-2 py-0.5 rounded-full bg-emerald-50 dark:bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 whitespace-nowrap">
                {t('mcpMarket.installedBadge')}
              </span>
            )}
          </div>
          <p className="mt-0.5 text-[11px] font-mono text-[var(--text-tertiary)] truncate" title={item.registryName}>
            {item.registryName}
          </p>
        </div>
        <span className="text-[10px] font-bold px-2 py-0.5 rounded-full bg-black/5 dark:bg-white/10 text-[var(--text-secondary)] whitespace-nowrap">
          v{item.version || '—'}
        </span>
      </div>

      <p className="mt-2.5 text-xs leading-relaxed text-[var(--text-secondary)] dark:text-gray-400 line-clamp-2 min-h-[2.5rem]">
        {item.description || t('mcpMarket.noDescription')}
      </p>

      <div className="mt-3 flex flex-wrap items-center gap-1.5">
        <McpKindChips kinds={item.installKinds} />
        <span className="text-[10px] font-semibold px-2 py-0.5 rounded-full bg-[var(--accent-primary-soft)] text-[var(--accent-primary)]">
          {item.source === 'featured'
            ? t('mcpMarket.tabs.featured')
            : t('mcpMarket.sourceRegistry')}
        </span>
        <Icon name="chevron_right" className="ml-auto text-base text-[var(--text-tertiary)]" />
      </div>
    </button>
  );
}
