import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useDebounce } from 'ahooks';
import { Icon } from '@/components/Icon';
import { useToast } from '@/components/Toast';
import { getAgentDisplayName } from '@/constants';
import { appendOperationLog } from '@/pages/Dashboard/hooks/useOperationLog';
import { mcpMarketplaceApi, projectsApi } from '@/api/tauri';
import type {
  McpInstallRecord,
  McpInstallResult,
  McpMarketplaceDetail,
  McpMarketplaceItem,
  ProjectConfig,
} from '@/types';
import { useMcpMarketplace } from './hooks/useMcpMarketplace';
import { McpMarketCard, McpMarketIcon } from './components/McpMarketCard';
import { McpMarketDetail } from './components/McpMarketDetail';
import { McpInstallDialog } from './components/McpInstallDialog';

type MarketTab = 'featured' | 'all' | 'installed';

const TABS: Array<{ id: MarketTab; labelKey: string; icon: string }> = [
  { id: 'featured', labelKey: 'mcpMarket.tabs.featured', icon: 'star' },
  { id: 'all', labelKey: 'mcpMarket.tabs.all', icon: 'apps' },
  { id: 'installed', labelKey: 'mcpMarket.tabs.installed', icon: 'download_for_offline' },
];

const EMPTY_ITEM = (
  record: McpInstallRecord,
): McpMarketplaceItem => ({
  registryName: record.registryName,
  title: record.title || record.localName,
  description: record.description,
  version: record.version,
  iconUrl: null,
  repositoryUrl: null,
  websiteUrl: null,
  categories: [],
  installKinds: [],
  installOptions: [],
  source: 'registry',
  updatedAt: null,
});

/** MCP 市场：内置精选 + 官方 Registry 搜索/全量列表 + 已安装记录。 */
export default function McpMarket() {
  const { t } = useTranslation();
  const { showToast } = useToast();
  const {
    featured,
    featuredLoading,
    featuredError,
    items,
    listLoading,
    listError,
    nextCursor,
    loadingMore,
    installs,
    installsLoading,
    installsError,
    loadFeatured,
    loadInstalls,
    loadFirstPage,
    loadMore,
  } = useMcpMarketplace();

  const [tab, setTab] = useState<MarketTab>('featured');
  const [search, setSearch] = useState('');
  const debounced = useDebounce(search, { wait: 400 });
  const searchLength = search.trim().length;
  const searchTerm = searchLength >= 3 ? debounced.trim() : '';
  const effectiveQuery = searchTerm.length >= 3 ? searchTerm : '';
  const isSearching = effectiveQuery.length >= 3;
  const [category, setCategory] = useState('all');

  const [selected, setSelected] = useState<McpMarketplaceItem | null>(null);
  const [detail, setDetail] = useState<McpMarketplaceDetail | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [detailError, setDetailError] = useState<string | null>(null);
  const detailRequest = useRef(0);
  const [installOpen, setInstallOpen] = useState(false);
  const [warnings, setWarnings] = useState<string[]>([]);
  const [projects, setProjects] = useState<ProjectConfig[]>([]);

  const projectName = (projectId: string) =>
    projects.find((project) => project.id === projectId)?.name ?? projectId;

  useEffect(() => {
    void loadFeatured();
    void loadInstalls();
    void projectsApi
      .list()
      .then(setProjects)
      .catch(() => {
        /* 项目名只用于展示，失败时回退到 projectId */
      });
  }, [loadFeatured, loadInstalls]);

  useEffect(() => {
    if (effectiveQuery && tab !== 'all') {
      setTab('all');
      return;
    }
    if (tab === 'all') {
      void loadFirstPage(effectiveQuery);
    }
  }, [tab, effectiveQuery, loadFirstPage]);

  useEffect(() => {
    if (tab === 'installed') void loadInstalls();
  }, [tab, loadInstalls]);

  const installedKeys = useMemo(
    () => new Set(installs.map((record) => record.registryName)),
    [installs],
  );

  const categories = useMemo(() => {
    const set = new Set<string>();
    for (const item of featured) {
      for (const itemCategory of item.categories) set.add(itemCategory);
    }
    return Array.from(set).sort();
  }, [featured]);

  const visibleFeatured = useMemo(
    () =>
      category === 'all'
        ? featured
        : featured.filter((item) => item.categories.includes(category)),
    [featured, category],
  );

  const switchTab = (next: MarketTab) => {
    setSearch('');
    setTab(next);
    if (next === 'featured') void loadFeatured();
    if (next === 'installed') void loadInstalls();
  };

  const refresh = () => {
    if (tab === 'featured') void loadFeatured();
    else if (tab === 'all') void loadFirstPage(effectiveQuery);
    else void loadInstalls();
  };

  const openDetail = async (item: McpMarketplaceItem) => {
    setSelected(item);
    setDetail(
      item.installOptions.length > 0
        ? { ...item, publishedAt: null, status: null }
        : null,
    );
    setDetailError(null);

    if (item.installOptions.length > 0) return;

    const id = ++detailRequest.current;
    setDetailLoading(true);
    try {
      const result = await mcpMarketplaceApi.detail(item.registryName);
      if (id === detailRequest.current) setDetail(result);
    } catch (error) {
      if (id === detailRequest.current) setDetailError(String(error));
    } finally {
      if (id === detailRequest.current) setDetailLoading(false);
    }
  };

  const closeDetail = () => {
    detailRequest.current += 1;
    setSelected(null);
    setDetail(null);
    setDetailError(null);
    setDetailLoading(false);
    setInstallOpen(false);
  };

  const handleInstalled = (result: McpInstallResult) => {
    const targets = [
      ...result.installedGlobal.map((agent) => getAgentDisplayName(agent)),
      ...result.installedProjects.map(
        (entry) => `${projectName(entry.projectId)}/${getAgentDisplayName(entry.target)}`,
      ),
    ];
    appendOperationLog({
      type: 'download',
      skillName: result.server.name,
      downloadTarget: targets.join(', ') || '—',
    });
    showToast(
      'success',
      result.warnings.length > 0
        ? t('mcpMarket.install.successWithWarnings', {
            name: result.server.name,
            count: result.warnings.length,
          })
        : t('mcpMarket.install.success', { name: result.server.name }),
    );
    setWarnings(result.warnings);
    setInstallOpen(false);
    closeDetail();
    void loadInstalls();
  };

  const forgetRecord = async (record: McpInstallRecord) => {
    try {
      await mcpMarketplaceApi.forget(record.registryName, record.localName);
      await loadInstalls();
      showToast('success', t('mcpMarket.installed.forgotten'));
    } catch (error) {
      showToast('error', t('mcpMarket.actionFailed'), String(error));
    }
  };

  const error = tab === 'featured' ? featuredError : tab === 'all' ? listError : installsError;
  const loading =
    tab === 'featured' ? featuredLoading : tab === 'all' ? listLoading : installsLoading;

  return (
    <div className="h-full flex flex-col bg-[var(--bg-primary)] dark:bg-dark-bg-secondary">
      <div className="flex-shrink-0 px-5 pt-5 pb-3" data-tauri-drag-region>
        <div className="flex items-center gap-3 h-11">
          <div className="relative grid grid-cols-3 items-center rounded-lg border border-[var(--border-color)] dark:border-dark-border bg-[var(--shell-surface-muted)] dark:bg-dark-bg-tertiary p-0.5 h-9 shrink-0">
            <div
              className="absolute top-0.5 bottom-0.5 rounded-md bg-[var(--bg-card)] dark:bg-dark-bg-card shadow-sm transition-all duration-200 ease-in-out"
              style={{
                left: `calc(${TABS.findIndex((item) => item.id === tab)} * (100% - 4px) / 3 + 2px)`,
                width: 'calc((100% - 4px) / 3)',
              }}
            />
            {TABS.map((item) => {
              const active = item.id === tab;
              return (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => switchTab(item.id)}
                  aria-pressed={active}
                  className="relative z-10 flex items-center justify-center gap-1.5 px-3 h-full rounded-md transition-colors"
                >
                  <Icon
                    name={item.icon}
                    className={`text-base transition-colors ${
                      active
                        ? 'text-[var(--accent-primary)]'
                        : 'text-slate-400 dark:text-gray-500'
                    }`}
                  />
                  <span
                    className={`text-xs font-bold whitespace-nowrap transition-colors ${
                      active
                        ? 'text-slate-700 dark:text-white'
                        : 'text-slate-400 dark:text-gray-500'
                    }`}
                  >
                    {t(item.labelKey)}
                  </span>
                </button>
              );
            })}
          </div>

          <div className="flex-1 relative">
            <div
              className={`relative flex items-center h-9 rounded-lg border bg-[var(--bg-card)] dark:bg-dark-bg-card overflow-hidden transition-all duration-300 ${
                search
                  ? 'border-[var(--accent-primary)]/40'
                  : 'border-[var(--border-color)] dark:border-dark-border focus-within:border-[var(--accent-primary)]'
              }`}
            >
              <Icon name="search" className="flex-shrink-0 ml-3 text-slate-400 dark:text-gray-400" />
              <input
                type="text"
                value={search}
                onChange={(event) => setSearch(event.target.value)}
                placeholder={t('mcpMarket.searchPlaceholder')}
                className="flex-1 h-full w-full bg-transparent border-0 outline-none ring-0 focus:ring-0 text-sm text-slate-900 dark:text-white placeholder:text-slate-400 dark:placeholder:text-gray-500 pl-2 pr-8"
              />
              {search && (
                <button
                  type="button"
                  onClick={() => setSearch('')}
                  title={t('skillDownload.clear')}
                  className="absolute top-1/2 -translate-y-1/2 right-2 w-5 h-5 flex items-center justify-center rounded hover:bg-[var(--shell-surface-soft)] dark:hover:bg-dark-bg-tertiary"
                >
                  <Icon name="close" className="text-sm text-slate-400 dark:text-gray-500" />
                </button>
              )}
            </div>
          </div>

          <button
            type="button"
            onClick={refresh}
            title={t('common.refresh')}
            className="p-2 rounded-xl border border-[var(--border-color)] dark:border-dark-border text-[var(--text-tertiary)] hover:text-[var(--text-primary)] transition-colors shrink-0"
          >
            <Icon name="refresh" className="text-base" />
          </button>
        </div>

        <div className="mt-3 flex flex-wrap items-center gap-2 min-h-6">
          {tab === 'featured' && !isSearching ? (
            <>
              <button
                type="button"
                onClick={() => setCategory('all')}
                className={`px-2.5 py-1 rounded-full text-[11px] font-bold transition-colors ${
                  category === 'all'
                    ? 'bg-[var(--accent-primary)] text-white'
                    : 'bg-black/5 dark:bg-white/10 text-[var(--text-secondary)]'
                }`}
              >
                {t('mcpMarket.categoryAll')}
              </button>
              {categories.map((item) => (
                <button
                  key={item}
                  type="button"
                  onClick={() => setCategory(item)}
                  className={`px-2.5 py-1 rounded-full text-[11px] font-bold transition-colors ${
                    category === item
                      ? 'bg-[var(--accent-primary)] text-white'
                      : 'bg-black/5 dark:bg-white/10 text-[var(--text-secondary)]'
                  }`}
                >
                  {t(`mcpMarket.categories.${item}`, item)}
                </button>
              ))}
              <span className="ml-auto text-[11px] text-[var(--text-tertiary)]">
                {t('mcpMarket.count', { count: visibleFeatured.length })}
              </span>
            </>
          ) : tab === 'installed' ? (
            <span className="text-[11px] text-[var(--text-tertiary)]">
              {t('mcpMarket.installedCount', { count: installs.length })}
            </span>
          ) : (
            <>
              <span className="text-[11px] text-[var(--text-tertiary)]">
                {isSearching
                  ? t('mcpMarket.searchResultCount', { count: items.length })
                  : t('mcpMarket.count', { count: items.length })}
              </span>
              {isSearching && (
                <span className="text-[11px] text-amber-600 dark:text-amber-400">
                  {t('mcpMarket.searchHint')}
                </span>
              )}
              {!isSearching && (
                <span className="ml-auto text-[11px] text-[var(--text-tertiary)]">
                  {t('mcpMarket.sourceRegistryHint')}
                </span>
              )}
            </>
          )}
          {searchLength > 0 && searchLength < 3 && (
            <span className="text-[11px] text-amber-600 dark:text-amber-400">
              {t('mcpMarket.searchTooShort')}
            </span>
          )}
        </div>
      </div>

      {warnings.length > 0 && (
        <div className="mx-5 mb-2 rounded-xl bg-amber-50 dark:bg-amber-500/10 px-3 py-2 text-xs text-amber-700 dark:text-amber-400">
          <div className="flex items-center justify-between gap-2">
            <span className="font-bold">{t('mcpMarket.install.warningList')}</span>
            <button type="button" onClick={() => setWarnings([])}>
              <Icon name="close" className="text-sm" />
            </button>
          </div>
          <ul className="mt-1 list-disc pl-4 space-y-0.5">
            {warnings.map((warning) => (
              <li key={warning}>{warning}</li>
            ))}
          </ul>
        </div>
      )}

      <div className="flex-1 overflow-y-auto px-5 pb-6">
        {loading && (
          <p className="text-sm text-[var(--text-tertiary)]">{t('mcpMarket.loading')}</p>
        )}

        {!loading && error && (
          <div className="mb-3 px-3 py-2 rounded-xl bg-red-50 dark:bg-red-500/10 text-xs text-red-600 dark:text-red-400 break-all">
            {t('mcpMarket.error', { error })}
          </div>
        )}

        {!loading && !error && tab === 'featured' && !isSearching && (
          visibleFeatured.length === 0 ? (
            <EmptyState icon="star" title={t('mcpMarket.emptyFeatured')} />
          ) : (
            <div className="grid grid-cols-1 xl:grid-cols-2 2xl:grid-cols-3 gap-3">
              {visibleFeatured.map((item) => (
                <McpMarketCard
                  key={item.registryName}
                  item={item}
                  installed={installedKeys.has(item.registryName)}
                  onSelect={(value) => void openDetail(value)}
                />
              ))}
            </div>
          )
        )}

        {!loading && !error && tab === 'all' && (
          items.length === 0 ? (
            <EmptyState
              icon="search_off"
              title={isSearching ? t('mcpMarket.emptyAll') : t('mcpMarket.emptyAll')}
            />
          ) : (
            <>
              <div className="grid grid-cols-1 xl:grid-cols-2 2xl:grid-cols-3 gap-3">
                {items.map((item) => (
                  <McpMarketCard
                    key={item.registryName}
                    item={item}
                    installed={installedKeys.has(item.registryName)}
                    onSelect={(value) => void openDetail(value)}
                  />
                ))}
              </div>
              {nextCursor && (
                <div className="mt-4 flex justify-center">
                  <button
                    type="button"
                    disabled={loadingMore}
                    onClick={() => void loadMore()}
                    className={`px-4 py-2 rounded-xl text-xs font-bold border border-[var(--border-color)] dark:border-dark-border text-[var(--text-secondary)] hover:border-[var(--accent-primary)] transition-colors ${
                      loadingMore ? 'opacity-60 cursor-wait' : ''
                    }`}
                  >
                    {loadingMore ? t('mcpMarket.loading') : t('mcpMarket.loadMore')}
                  </button>
                </div>
              )}
            </>
          )
        )}

        {!loading && !error && tab === 'installed' && (
          installs.length === 0 ? (
            <EmptyState icon="download_for_offline" title={t('mcpMarket.emptyInstalled')} />
          ) : (
            <div className="grid grid-cols-1 xl:grid-cols-2 2xl:grid-cols-3 gap-3">
              {installs.map((record) => (
                <div
                  key={`${record.registryName}::${record.localName}`}
                  className="p-4 rounded-2xl border border-[var(--border-color)] dark:border-dark-border bg-[var(--bg-card)] dark:bg-dark-bg-card"
                >
                  <div className="flex items-start gap-3">
                    <McpMarketIcon
                      item={{ title: record.title || record.localName, iconUrl: null }}
                    />
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2">
                        <span className="font-bold text-[var(--text-primary)] dark:text-white truncate">
                          {record.title || record.localName}
                        </span>
                        <span className="text-[10px] font-bold px-2 py-0.5 rounded-full bg-black/5 dark:bg-white/10 text-[var(--text-secondary)]">
                          v{record.version || '—'}
                        </span>
                      </div>
                      <p className="mt-0.5 text-[11px] font-mono text-[var(--text-tertiary)] truncate">
                        {record.registryName}
                      </p>
                      <p className="mt-0.5 text-[11px] text-[var(--text-tertiary)]">
                        {t('mcpMarket.installed.localName')}:{' '}
                        <span className="font-mono">{record.localName}</span>
                      </p>
                    </div>
                  </div>

                  <div className="mt-3 flex flex-wrap gap-1.5">
                    {record.globalTargets.map((agent) => (
                      <span
                        key={`g-${agent}`}
                        className="text-[10px] font-semibold px-2 py-0.5 rounded-full bg-black/5 dark:bg-white/10 text-[var(--text-secondary)]"
                      >
                        {t('mcpMarket.installed.globalTarget', {
                          agent: getAgentDisplayName(agent),
                        })}
                      </span>
                    ))}
                    {record.projectTargets.map((entry) => (
                      <span
                        key={`p-${entry.projectId}-${entry.target}`}
                        className="text-[10px] font-semibold px-2 py-0.5 rounded-full bg-black/5 dark:bg-white/10 text-[var(--text-secondary)]"
                      >
                        {t('mcpMarket.installed.projectTarget', {
                          project: projectName(entry.projectId),
                          target: getAgentDisplayName(entry.target),
                        })}
                      </span>
                    ))}
                  </div>

                  <div className="mt-2 text-[10px] text-[var(--text-tertiary)]">
                    {t('mcpMarket.installed.installedAt')}: {record.installedAt.slice(0, 10)}
                  </div>

                  <div className="mt-3 flex items-center justify-end gap-2">
                    <button
                      type="button"
                      onClick={() => void forgetRecord(record)}
                      className="px-3 py-1.5 rounded-lg text-xs font-semibold text-[var(--text-tertiary)] hover:bg-black/5 dark:hover:bg-white/5 transition-colors"
                    >
                      {t('mcpMarket.installed.forget')}
                    </button>
                    <button
                      type="button"
                      onClick={() => void openDetail(EMPTY_ITEM(record))}
                      className="px-3 py-1.5 rounded-lg text-xs font-semibold bg-[var(--accent-primary)] text-white hover:opacity-90 transition-opacity"
                    >
                      {t('mcpMarket.installed.reinstall')}
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )
        )}
      </div>

      <McpMarketDetail
        item={selected}
        detail={detail}
        loading={detailLoading}
        error={detailError}
        installed={selected ? installedKeys.has(selected.registryName) : false}
        onClose={closeDetail}
        onInstall={(value) => {
          setDetail(value);
          setInstallOpen(true);
        }}
      />

      <McpInstallDialog
        open={installOpen}
        detail={detail}
        onClose={() => setInstallOpen(false)}
        onInstalled={handleInstalled}
      />
    </div>
  );
}

function EmptyState({ icon, title }: { icon: string; title: string }) {
  return (
    <div className="flex flex-col items-center justify-center text-center py-20 rounded-3xl border border-dashed border-[var(--border-color)] dark:border-dark-border">
      <Icon name={icon} className="text-4xl text-[var(--text-tertiary)] mb-3" />
      <h4 className="font-bold text-[var(--text-primary)] dark:text-white">{title}</h4>
    </div>
  );
}
