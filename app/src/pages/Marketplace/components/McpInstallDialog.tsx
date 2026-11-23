import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Icon } from '@/components/Icon';
import { getAgentDisplayName, getAgentIconUrl } from '@/constants';
import { mcpApi, mcpMarketplaceApi, projectsApi } from '@/api/tauri';
import type {
  McpAgentInfo,
  McpInstallInput,
  McpInstallOption,
  McpInstallResult,
  McpMarketplaceDetail,
  McpProjectTarget,
  ProjectConfig,
} from '@/types';

interface McpInstallDialogProps {
  open: boolean;
  detail: McpMarketplaceDetail | null;
  onClose: () => void;
  onInstalled: (result: McpInstallResult) => void;
}

const defaultsFor = (option: McpInstallOption | null | undefined): Record<string, string> => {
  const out: Record<string, string> = {};
  for (const input of option?.inputs ?? []) {
    out[input.key] = input.defaultValue ?? '';
  }
  return out;
};

const slugify = (raw: string): string =>
  raw
    .toLowerCase()
    .replace(/[^\p{L}\p{N}]+/gu, '-')
    .replace(/^-+|-+$/g, '');

const suggestLocalName = (registryName: string, title: string): string => {
  const fromTitle = slugify(title);
  if (fromTitle) return fromTitle;
  const last = registryName.split('/').pop() ?? registryName;
  return slugify(last) || 'mcp-server';
};

const inputKindLabelKey: Record<McpInstallInput['kind'], string> = {
  env: 'mcpMarket.inputKinds.env',
  header: 'mcpMarket.inputKinds.header',
  argument: 'mcpMarket.inputKinds.argument',
  variable: 'mcpMarket.inputKinds.variable',
};

/** 从市场安装 MCP server：选择安装方式、填写配置、选择全局/项目目标。 */
export function McpInstallDialog({ open, detail, onClose, onInstalled }: McpInstallDialogProps) {
  const { t } = useTranslation();

  const [optionId, setOptionId] = useState('');
  const [values, setValues] = useState<Record<string, string>>({});
  const [localName, setLocalName] = useState('');
  const [enabled, setEnabled] = useState(true);
  const [overwrite, setOverwrite] = useState(false);
  const [trusted, setTrusted] = useState(false);

  const [agents, setAgents] = useState<McpAgentInfo[]>([]);
  const [projects, setProjects] = useState<ProjectConfig[]>([]);
  const [targetsLoading, setTargetsLoading] = useState(false);
  const [globalTargets, setGlobalTargets] = useState<string[]>([]);
  const [projectSelection, setProjectSelection] = useState<Record<string, string[]>>({});
  const [projectAgents, setProjectAgents] = useState<Record<string, McpAgentInfo[]>>({});
  const [loadingProjectId, setLoadingProjectId] = useState<string | null>(null);

  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const options = useMemo(() => detail?.installOptions ?? [], [detail]);
  const option = useMemo(
    () =>
      options.find((item) => item.id === optionId) ??
      options.find((item) => item.supported) ??
      options[0] ??
      null,
    [options, optionId],
  );

  // 打开时重置表单
  useEffect(() => {
    if (!open || !detail) return;
    const first = detail.installOptions.find((item) => item.supported) ?? detail.installOptions[0] ?? null;
    setOptionId(first?.id ?? '');
    setValues(defaultsFor(first));
    setLocalName(suggestLocalName(detail.registryName, detail.title));
    setEnabled(true);
    setOverwrite(false);
    setTrusted(false);
    setGlobalTargets([]);
    setProjectSelection({});
    setProjectAgents({});
    setError(null);
    setBusy(false);
  }, [open, detail]);

  // 切换安装方式时重置配置项默认值
  useEffect(() => {
    if (!open || !option) return;
    setValues(defaultsFor(option));
    setError(null);
  }, [open, option?.id]);

  // 读取全局 Agent 与项目列表
  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    setTargetsLoading(true);
    Promise.all([mcpApi.list(), projectsApi.list()])
      .then(([overview, projectList]) => {
        if (cancelled) return;
        setAgents(overview.agents);
        setProjects(projectList);
        setGlobalTargets((current) => {
          if (current.length > 0) return current;
          const detected = overview.agents.filter((agent) => agent.exists);
          const preferred =
            detected.find((agent) => agent.agent === 'claude') ?? detected[0];
          return preferred ? [preferred.agent] : [];
        });
      })
      .catch((reason) => {
        if (!cancelled) setError(String(reason));
      })
      .finally(() => {
        if (!cancelled) setTargetsLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [open]);

  if (!open || !detail) return null;

  const toggleGlobal = (agent: string) => {
    setGlobalTargets((current) =>
      current.includes(agent)
        ? current.filter((item) => item !== agent)
        : [...current, agent],
    );
  };

  const toggleProject = async (projectId: string) => {
    if (projectId in projectSelection) {
      setProjectSelection((current) => {
        const next = { ...current };
        delete next[projectId];
        return next;
      });
      return;
    }

    setProjectSelection((current) => ({ ...current, [projectId]: [] }));
    let list: McpAgentInfo[] | undefined = projectAgents[projectId];
    if (!list) {
      setLoadingProjectId(projectId);
      try {
        const overview = await mcpApi.listProject(projectId);
        list = overview.agents;
        setProjectAgents((current) => ({ ...current, [projectId]: list as McpAgentInfo[] }));
      } catch (reason) {
        setError(String(reason));
        setProjectSelection((current) => {
          const next = { ...current };
          delete next[projectId];
          return next;
        });
        return;
      } finally {
        setLoadingProjectId(null);
      }
    }

    const first = list.find((agent) => agent.exists) ?? list[0];
    if (first) {
      setProjectSelection((current) =>
        current[projectId] ? { ...current, [projectId]: [first.agent] } : current,
      );
    }
  };

  const toggleProjectTarget = (projectId: string, target: string) => {
    setProjectSelection((current) => {
      const selected = current[projectId];
      if (!selected) return current;
      return {
        ...current,
        [projectId]: selected.includes(target)
          ? selected.filter((item) => item !== target)
          : [...selected, target],
      };
    });
  };

  const missingInput = option?.inputs.find(
    (input) => input.required && !(values[input.key] ?? '').trim(),
  );
  const selectedProjectTargets: McpProjectTarget[] = Object.entries(projectSelection).flatMap(
    ([projectId, targets]) => targets.map((target) => ({ projectId, target })),
  );
  const emptyProject = Object.entries(projectSelection).find(([, targets]) => targets.length === 0);
  const canSubmit = !!option && option.supported && !busy;

  const submit = async () => {
    if (!option) return;
    if (!option.supported) {
      setError(t('mcpMarket.detail.unsupportedShort'));
      return;
    }
    if (missingInput) {
      setError(t('mcpMarket.install.valuesRequired', { label: missingInput.label }));
      return;
    }
    if (!localName.trim()) {
      setError(t('mcpMarket.install.localNameRequired'));
      return;
    }
    if (/[\\/]/.test(localName)) {
      setError(t('mcpMarket.install.localNameInvalid'));
      return;
    }
    if (globalTargets.length === 0 && selectedProjectTargets.length === 0) {
      setError(t('mcpMarket.install.targetsRequired'));
      return;
    }
    if (emptyProject) {
      const project = projects.find((item) => item.id === emptyProject[0]);
      setError(
        t('mcpMarket.install.projectTargetsRequired', {
          name: project?.name ?? emptyProject[0],
        }),
      );
      return;
    }
    if (!trusted) {
      setError(t('mcpMarket.install.trustedRequired'));
      return;
    }

    setBusy(true);
    setError(null);
    try {
      const result = await mcpMarketplaceApi.install({
        registryName: detail.registryName,
        version: detail.version,
        optionId: option.id,
        localName: localName.trim(),
        values,
        enabled,
        globalTargets,
        projectTargets: selectedProjectTargets,
        overwrite,
      });
      onInstalled(result);
      onClose();
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-[9999] bg-black/30 backdrop-blur-sm flex items-center justify-center p-6"
      onClick={() => !busy && onClose()}
    >
      <div
        className="w-full max-w-3xl max-h-[88vh] flex flex-col rounded-2xl bg-[var(--bg-card)] dark:bg-dark-bg-card shadow-2xl overflow-hidden"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="px-5 pt-5 pb-3 border-b border-[var(--border-color)] dark:border-dark-border">
          <h3 className="text-lg font-bold text-[var(--text-primary)] dark:text-white">
            {t('mcpMarket.install.title', { name: detail.title })}
          </h3>
          <p className="mt-0.5 text-xs font-mono text-[var(--text-tertiary)] break-all">
            {detail.registryName}
          </p>
        </div>

        <div className="flex-1 overflow-y-auto px-5 py-4 space-y-5">
          <section>
            <h4 className="text-xs font-bold uppercase tracking-widest text-[var(--text-tertiary)] mb-2">
              {t('mcpMarket.install.optionTitle')}
            </h4>
            <div className="space-y-2">
              {options.map((item) => {
                const active = item.id === option?.id;
                return (
                  <button
                    key={item.id}
                    type="button"
                    disabled={!item.supported || busy}
                    onClick={() => setOptionId(item.id)}
                    className={`w-full text-left rounded-xl border p-3 transition-colors ${
                      active
                        ? 'border-[var(--accent-primary)] bg-[var(--accent-primary-soft)]'
                        : 'border-[var(--border-color)] dark:border-dark-border'
                    } ${!item.supported ? 'opacity-60 cursor-not-allowed' : 'hover:border-[var(--accent-primary)]'}`}
                  >
                    <div className="flex items-center gap-2">
                      <Icon
                        name={active ? 'check_circle' : 'radio_button_unchecked'}
                        className={`text-base ${
                          active
                            ? 'text-[var(--accent-primary)]'
                            : 'text-[var(--text-tertiary)]'
                        }`}
                      />
                      <span className="text-sm font-bold text-[var(--text-primary)] dark:text-white">
                        {item.label}
                      </span>
                      <span className="text-[10px] font-bold px-2 py-0.5 rounded-full bg-black/5 dark:bg-white/10 text-[var(--text-secondary)] uppercase">
                        {item.kind === 'remote' ? item.transport : item.packageType ?? 'package'}
                      </span>
                      {!item.supported && (
                        <span className="ml-auto text-[10px] font-bold text-amber-600 dark:text-amber-400">
                          {t('mcpMarket.detail.unsupportedShort')}
                        </span>
                      )}
                    </div>
                    {item.unsupportedReason && (
                      <p className="mt-1 pl-6 text-xs text-amber-700 dark:text-amber-400">
                        {item.unsupportedReason}
                      </p>
                    )}
                    {item.warnings.map((warning) => (
                      <p
                        key={warning}
                        className="mt-1 pl-6 text-xs text-amber-700 dark:text-amber-400"
                      >
                        {warning}
                      </p>
                    ))}
                  </button>
                );
              })}
            </div>
          </section>

          <section>
            <h4 className="text-xs font-bold uppercase tracking-widest text-[var(--text-tertiary)] mb-2">
              {t('mcpMarket.install.inputs')}
            </h4>
            {!option || option.inputs.length === 0 ? (
              <p className="text-xs text-[var(--text-tertiary)]">
                {t('mcpMarket.install.noInputs')}
              </p>
            ) : (
              <div className="space-y-3">
                {option.inputs.map((input) => (
                  <label key={input.key} className="block">
                    <span className="flex items-center gap-1.5 text-xs font-bold text-[var(--text-secondary)] dark:text-gray-300">
                      <span className="font-mono">{input.label}</span>
                      <span className="text-[10px] font-semibold text-[var(--text-tertiary)]">
                        {t(inputKindLabelKey[input.kind])}
                      </span>
                      {input.required && (
                        <span className="text-[10px] font-bold text-red-500">
                          {t('mcpMarket.install.required')}
                        </span>
                      )}
                      {input.secret && (
                        <Icon name="lock" className="text-xs text-[var(--text-tertiary)]" />
                      )}
                    </span>
                    {input.description && (
                      <span className="mt-0.5 block text-[11px] leading-relaxed text-[var(--text-tertiary)]">
                        {input.description}
                      </span>
                    )}
                    {input.choices.length > 0 ? (
                      <select
                        value={values[input.key] ?? ''}
                        disabled={busy}
                        onChange={(event) =>
                          setValues((current) => ({
                            ...current,
                            [input.key]: event.target.value,
                          }))
                        }
                        className="mt-1 w-full px-3 py-2 rounded-xl border border-[var(--border-color)] dark:border-dark-border bg-transparent text-sm text-[var(--text-primary)] dark:text-white outline-none focus:border-[var(--accent-primary)]"
                      >
                        <option value="">—</option>
                        {input.choices.map((choice) => (
                          <option key={choice} value={choice}>
                            {choice}
                          </option>
                        ))}
                      </select>
                    ) : (
                      <input
                        type={input.secret ? 'password' : 'text'}
                        value={values[input.key] ?? ''}
                        disabled={busy}
                        placeholder={input.placeholder ?? ''}
                        onChange={(event) =>
                          setValues((current) => ({
                            ...current,
                            [input.key]: event.target.value,
                          }))
                        }
                        className="mt-1 w-full px-3 py-2 rounded-xl border border-[var(--border-color)] dark:border-dark-border bg-transparent text-sm font-mono text-[var(--text-primary)] dark:text-white outline-none focus:border-[var(--accent-primary)]"
                      />
                    )}
                  </label>
                ))}
              </div>
            )}
          </section>

          <section className="space-y-3">
            <label className="block">
              <span className="text-xs font-bold uppercase tracking-widest text-[var(--text-tertiary)]">
                {t('mcpMarket.install.localName')}
              </span>
              <input
                value={localName}
                disabled={busy}
                onChange={(event) => setLocalName(event.target.value)}
                className="mt-1 w-full px-3 py-2 rounded-xl border border-[var(--border-color)] dark:border-dark-border bg-transparent text-sm font-mono text-[var(--text-primary)] dark:text-white outline-none focus:border-[var(--accent-primary)]"
              />
              <span className="mt-1 block text-[11px] text-[var(--text-tertiary)]">
                {t('mcpMarket.install.localNameHint')}
              </span>
            </label>

            <div className="flex flex-wrap items-center gap-4">
              <label className="flex items-center gap-2 text-sm text-[var(--text-primary)] dark:text-white">
                <input
                  type="checkbox"
                  checked={enabled}
                  disabled={busy}
                  onChange={(event) => setEnabled(event.target.checked)}
                  className="accent-[var(--accent-primary)]"
                />
                {t('mcpMarket.install.enabled')}
              </label>
              <label className="flex items-center gap-2 text-sm text-[var(--text-primary)] dark:text-white">
                <input
                  type="checkbox"
                  checked={overwrite}
                  disabled={busy}
                  onChange={(event) => setOverwrite(event.target.checked)}
                  className="accent-[var(--accent-primary)]"
                />
                {t('mcpMarket.install.overwrite')}
              </label>
            </div>
          </section>

          <section>
            <div className="flex items-center justify-between mb-1.5">
              <h4 className="text-xs font-bold uppercase tracking-widest text-[var(--text-tertiary)]">
                {t('mcpMarket.install.globalTargets')}
              </h4>
              <div className="flex items-center gap-2 text-[11px] font-semibold">
                <button
                  type="button"
                  disabled={busy || targetsLoading}
                  onClick={() =>
                    setGlobalTargets(
                      agents.filter((agent) => agent.exists).map((agent) => agent.agent),
                    )
                  }
                  className="text-[var(--accent-primary)] hover:underline disabled:opacity-50"
                >
                  {t('mcpMarket.install.selectAllDetected')}
                </button>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => setGlobalTargets([])}
                  className="text-[var(--text-tertiary)] hover:underline disabled:opacity-50"
                >
                  {t('mcpMarket.install.clearTargets')}
                </button>
              </div>
            </div>
            {targetsLoading ? (
              <p className="text-xs text-[var(--text-tertiary)]">{t('common.loading')}</p>
            ) : (
              <div className="flex flex-wrap gap-2">
                {agents.map((agent) => {
                  const active = globalTargets.includes(agent.agent);
                  return (
                    <button
                      key={agent.agent}
                      type="button"
                      disabled={busy}
                      onClick={() => toggleGlobal(agent.agent)}
                      className={`flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-xs font-semibold border transition-colors ${
                        active
                          ? 'border-[var(--accent-primary)] text-[var(--accent-primary)] bg-[var(--accent-primary-soft)]'
                          : 'border-[var(--border-color)] dark:border-dark-border text-[var(--text-secondary)]'
                      }`}
                    >
                      <img
                        src={getAgentIconUrl(agent.agent)}
                        alt=""
                        className="w-4 h-4 rounded-sm object-contain"
                      />
                      {getAgentDisplayName(agent.agent)}
                      {!agent.exists && (
                        <span className="opacity-60">· {t('mcpMarket.install.notDetected')}</span>
                      )}
                    </button>
                  );
                })}
              </div>
            )}
          </section>

          <section>
            <h4 className="text-xs font-bold uppercase tracking-widest text-[var(--text-tertiary)] mb-1.5">
              {t('mcpMarket.install.projectTargets')}
            </h4>
            {projects.length === 0 ? (
              <p className="text-xs text-[var(--text-tertiary)]">
                {t('mcpMarket.install.noProjects')}
              </p>
            ) : (
              <div className="space-y-2">
                {projects.map((project) => {
                  const selected = project.id in projectSelection;
                  const targets = projectSelection[project.id] ?? [];
                  const list = projectAgents[project.id] ?? [];
                  return (
                    <div
                      key={project.id}
                      className={`rounded-xl border p-3 ${
                        selected
                          ? 'border-[var(--accent-primary)]/50'
                          : 'border-[var(--border-color)] dark:border-dark-border'
                      }`}
                    >
                      <button
                        type="button"
                        disabled={busy}
                        onClick={() => void toggleProject(project.id)}
                        className="flex w-full items-center gap-2 text-left"
                      >
                        <Icon
                          name={selected ? 'check_circle' : 'radio_button_unchecked'}
                          className={`text-base ${
                            selected
                              ? 'text-[var(--accent-primary)]'
                              : 'text-[var(--text-tertiary)]'
                          }`}
                        />
                        <span className="text-sm font-semibold text-[var(--text-primary)] dark:text-white">
                          {project.name}
                        </span>
                        <span className="truncate text-[10px] font-mono text-[var(--text-tertiary)]">
                          {project.path}
                        </span>
                        {loadingProjectId === project.id && (
                          <Icon
                            name="autorenew"
                            className="ml-auto animate-spin text-sm text-[var(--text-tertiary)]"
                          />
                        )}
                      </button>
                      {selected && (
                        <div className="mt-2 flex flex-wrap gap-1.5 pl-6">
                          {list.length === 0 && loadingProjectId !== project.id && (
                            <span className="text-[11px] text-[var(--text-tertiary)]">
                              {t('common.loading')}
                            </span>
                          )}
                          {list.map((agent) => {
                            const active = targets.includes(agent.agent);
                            return (
                              <button
                                key={agent.agent}
                                type="button"
                                disabled={busy}
                                onClick={() => toggleProjectTarget(project.id, agent.agent)}
                                className={`flex items-center gap-1 px-2 py-1 rounded-md text-[11px] font-semibold border transition-colors ${
                                  active
                                    ? 'border-[var(--accent-primary)] text-[var(--accent-primary)] bg-[var(--accent-primary-soft)]'
                                    : 'border-[var(--border-color)] dark:border-dark-border text-[var(--text-secondary)]'
                                }`}
                              >
                                <img
                                  src={getAgentIconUrl(agent.agent)}
                                  alt=""
                                  className="w-3.5 h-3.5 rounded-sm object-contain"
                                />
                                {getAgentDisplayName(agent.agent)}
                              </button>
                            );
                          })}
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            )}
          </section>

          <label className="flex items-start gap-2 rounded-xl border border-amber-200 dark:border-amber-500/30 bg-amber-50/70 dark:bg-amber-500/5 p-3 text-xs leading-relaxed text-amber-800 dark:text-amber-300">
            <input
              type="checkbox"
              checked={trusted}
              disabled={busy}
              onChange={(event) => setTrusted(event.target.checked)}
              className="mt-0.5 accent-[var(--accent-primary)]"
            />
            <span>{t('mcpMarket.install.trusted')}</span>
          </label>
        </div>

        {error && (
          <div className="mx-5 mb-3 px-3 py-2 rounded-xl bg-red-50 dark:bg-red-500/10 text-xs text-red-600 dark:text-red-400 break-all">
            {error}
          </div>
        )}

        <div className="px-5 py-4 border-t border-[var(--border-color)] dark:border-dark-border flex items-center justify-between gap-3">
          <span className="text-[11px] text-[var(--text-tertiary)]">
            {t('mcpMarket.install.targetSummary', {
              global: globalTargets.length,
              project: selectedProjectTargets.length,
            })}
          </span>
          <div className="flex items-center gap-2">
            <button
              type="button"
              disabled={busy}
              onClick={onClose}
              className="px-3 py-2 rounded-xl text-sm font-semibold text-[var(--text-secondary)] hover:bg-black/5 dark:hover:bg-white/5 transition-colors"
            >
              {t('common.cancel')}
            </button>
            <button
              type="button"
              disabled={!canSubmit}
              onClick={() => void submit()}
              className={`flex items-center gap-1.5 px-4 py-2 rounded-xl text-sm font-semibold text-white bg-[var(--accent-primary)] hover:opacity-90 transition-opacity ${
                !canSubmit ? 'opacity-50 cursor-not-allowed' : ''
              }`}
            >
              {busy ? (
                <Icon name="autorenew" className="animate-spin text-base" />
              ) : (
                <Icon name="download" className="text-base" />
              )}
              {busy ? t('mcpMarket.install.installing') : t('mcpMarket.install.install')}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
