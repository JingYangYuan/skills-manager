import { useTranslation } from 'react-i18next';
import { open as openUrl } from '@tauri-apps/plugin-shell';
import { Drawer } from '@/components/Drawer';
import { Icon } from '@/components/Icon';
import type { McpInstallOption, McpMarketplaceDetail, McpMarketplaceItem } from '@/types';
import { McpKindChips, McpMarketIcon } from './McpMarketCard';

interface McpMarketDetailProps {
  item: McpMarketplaceItem | null;
  detail: McpMarketplaceDetail | null;
  loading: boolean;
  error: string | null;
  installed: boolean;
  onClose: () => void;
  onInstall: (detail: McpMarketplaceDetail) => void;
}

function optionCommand(option: McpInstallOption): string {
  if (option.kind === 'remote') return option.url ?? '';
  return [option.command, ...option.args].filter(Boolean).join(' ');
}

function optionKindLabel(option: McpInstallOption): string {
  if (option.kind === 'remote') {
    return option.transport === 'sse' ? 'SSE' : 'HTTP';
  }
  return option.label;
}

export function McpMarketDetail({
  item,
  detail,
  loading,
  error,
  installed,
  onClose,
  onInstall,
}: McpMarketDetailProps) {
  const { t } = useTranslation();
  if (!item) return null;

  const view = detail ?? item;
  const options = view.installOptions;
  const supported = options.filter((option) => option.supported);
  const canInstall = supported.length > 0 && !loading;

  const open = (url: string | null | undefined) => {
    if (!url) return;
    void openUrl(url).catch(() => {});
  };

  return (
    <Drawer isOpen onClose={onClose} title={view.title} width="560px">
      <div className="flex-1 overflow-y-auto px-5 py-4 space-y-5">
        <div className="flex items-start gap-3">
          <McpMarketIcon item={view} size="lg" />
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2">
              <h3 className="text-lg font-bold text-[var(--text-primary)] dark:text-white">
                {view.title}
              </h3>
              {installed && (
                <span className="text-[10px] font-bold px-2 py-0.5 rounded-full bg-emerald-50 dark:bg-emerald-500/10 text-emerald-600 dark:text-emerald-400">
                  {t('mcpMarket.installedBadge')}
                </span>
              )}
            </div>
            <p className="mt-1 text-xs font-mono text-[var(--text-tertiary)] break-all">
              {view.registryName}
            </p>
            <div className="mt-2 flex flex-wrap items-center gap-1.5">
              <span className="text-[10px] font-bold px-2 py-0.5 rounded-full bg-black/5 dark:bg-white/10 text-[var(--text-secondary)]">
                {t('mcpMarket.detail.version')} v{view.version || '—'}
              </span>
              <McpKindChips kinds={view.installKinds} />
              {view.categories.map((category) => (
                <span
                  key={category}
                  className="text-[10px] font-semibold px-2 py-0.5 rounded-full bg-[var(--accent-primary-soft)] text-[var(--accent-primary)]"
                >
                  {t(`mcpMarket.categories.${category}`, category)}
                </span>
              ))}
            </div>
          </div>
        </div>

        {view.description && (
          <p className="text-sm leading-relaxed text-[var(--text-secondary)] dark:text-gray-400 whitespace-pre-wrap">
            {view.description}
          </p>
        )}

        <div className="flex flex-wrap items-center gap-3 text-xs">
          {view.repositoryUrl && (
            <button
              type="button"
              onClick={() => open(view.repositoryUrl)}
              className="inline-flex items-center gap-1 font-semibold text-[var(--accent-primary)] hover:underline"
            >
              <Icon name="code" className="text-base" />
              {t('mcpMarket.detail.repository')}
            </button>
          )}
          {view.websiteUrl && (
            <button
              type="button"
              onClick={() => open(view.websiteUrl)}
              className="inline-flex items-center gap-1 font-semibold text-[var(--accent-primary)] hover:underline"
            >
              <Icon name="open_in_new" className="text-base" />
              {t('mcpMarket.detail.website')}
            </button>
          )}
        </div>

        <div>
          <h4 className="text-xs font-bold uppercase tracking-widest text-[var(--text-tertiary)] mb-2">
            {t('mcpMarket.detail.installWays')}
          </h4>

          {loading && (
            <div className="flex items-center gap-2 text-sm text-[var(--text-tertiary)]">
              <Icon name="autorenew" className="animate-spin text-base" />
              {t('mcpMarket.detail.loadingDetail')}
            </div>
          )}

          {!loading && options.length === 0 && (
            <p className="text-sm text-[var(--text-tertiary)]">
              {t('mcpMarket.detail.noInstallWays')}
            </p>
          )}

          <div className="space-y-2">
            {options.map((option) => (
              <div
                key={option.id}
                className={`rounded-xl border p-3 ${
                  option.supported
                    ? 'border-[var(--border-color)] dark:border-dark-border'
                    : 'border-amber-200 dark:border-amber-500/30 bg-amber-50/60 dark:bg-amber-500/5'
                }`}
              >
                <div className="flex flex-wrap items-center gap-2">
                  <span className="text-xs font-bold text-[var(--text-primary)] dark:text-white">
                    {option.label}
                  </span>
                  <span className="text-[10px] font-bold px-2 py-0.5 rounded-full bg-black/5 dark:bg-white/10 text-[var(--text-secondary)] uppercase">
                    {optionKindLabel(option)}
                  </span>
                  {option.packageType && (
                    <span className="text-[10px] font-mono text-[var(--text-tertiary)]">
                      {option.packageType}
                    </span>
                  )}
                  {!option.supported && (
                    <span className="text-[10px] font-bold px-2 py-0.5 rounded-full bg-amber-100 dark:bg-amber-500/10 text-amber-700 dark:text-amber-400">
                      {t('mcpMarket.detail.unsupportedShort')}
                    </span>
                  )}
                </div>
                {optionCommand(option) && (
                  <p className="mt-1.5 text-[11px] font-mono text-[var(--text-tertiary)] break-all">
                    {optionCommand(option)}
                  </p>
                )}
                {Object.keys(option.env).length > 0 && (
                  <p className="mt-1 text-[10px] font-mono text-[var(--text-tertiary)]">
                    ENV: {Object.keys(option.env).join(', ')}
                  </p>
                )}
                {Object.keys(option.headers).length > 0 && (
                  <p className="mt-1 text-[10px] font-mono text-[var(--text-tertiary)]">
                    Headers: {Object.keys(option.headers).join(', ')}
                  </p>
                )}
                {!option.supported && option.unsupportedReason && (
                  <p className="mt-1.5 text-xs text-amber-700 dark:text-amber-400">
                    {t('mcpMarket.detail.unsupported', { reason: option.unsupportedReason })}
                  </p>
                )}
                {option.warnings.map((warning) => (
                  <p key={warning} className="mt-1.5 text-xs text-amber-700 dark:text-amber-400">
                    {warning}
                  </p>
                ))}
              </div>
            ))}
          </div>
        </div>

        {error && (
          <div className="px-3 py-2 rounded-xl bg-red-50 dark:bg-red-500/10 text-xs text-red-600 dark:text-red-400 break-all">
            {t('mcpMarket.error', { error })}
          </div>
        )}

        <div className="rounded-xl border border-amber-200 dark:border-amber-500/30 bg-amber-50/70 dark:bg-amber-500/5 p-3">
          <div className="flex items-center gap-1.5 text-xs font-bold text-amber-700 dark:text-amber-400">
            <Icon name="lock" className="text-base" />
            {t('mcpMarket.detail.security')}
          </div>
          <p className="mt-1 text-xs leading-relaxed text-amber-800/80 dark:text-amber-300/80">
            {t('mcpMarket.detail.securityDesc')}
          </p>
        </div>
      </div>

      <div className="px-5 py-4 border-t border-[var(--border-color)] dark:border-dark-border flex items-center justify-between gap-3 flex-shrink-0">
        <span className="text-[11px] text-[var(--text-tertiary)]">
          {detail?.publishedAt
            ? `${t('mcpMarket.detail.publishedAt')}: ${detail.publishedAt.slice(0, 10)}`
            : view.updatedAt
              ? `${t('mcpMarket.detail.updatedAt')}: ${view.updatedAt.slice(0, 10)}`
              : ''}
        </span>
        <button
          type="button"
          disabled={!canInstall}
          onClick={() => {
            if (detail) onInstall(detail);
          }}
          className={`flex items-center gap-1.5 px-4 py-2 rounded-xl text-sm font-semibold text-white bg-[var(--accent-primary)] hover:opacity-90 transition-opacity ${
            !canInstall ? 'opacity-50 cursor-not-allowed' : ''
          }`}
        >
          <Icon name="download" className="text-base" />
          {t('mcpMarket.detail.install')}
        </button>
      </div>
    </Drawer>
  );
}
