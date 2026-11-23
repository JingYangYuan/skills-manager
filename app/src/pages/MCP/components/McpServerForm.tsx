import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Icon } from '@/components/Icon';
import type { McpAgentInfo, McpServerAggregate, McpServerDef, McpTransport } from '@/types';

interface McpServerFormProps {
  open: boolean;
  initial: McpServerAggregate | null;
  agents: McpAgentInfo[];
  /** 项目级：目标固定为某个 agent 的配置文件 */
  lockedTarget?: string;
  onClose: () => void;
  onSubmit: (server: McpServerDef, targets: string[]) => Promise<void>;
}

const parsePairs = (text: string): Record<string, string> => {
  const out: Record<string, string> = {};
  for (const line of text.split('\n')) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const index = trimmed.indexOf('=');
    if (index === -1) continue;
    const key = trimmed.slice(0, index).trim();
    if (!key) continue;
    out[key] = trimmed.slice(index + 1).trim();
  }
  return out;
};

const formatPairs = (pairs: Record<string, string>): string =>
  Object.entries(pairs)
    .map(([key, value]) => `${key}=${value}`)
    .join('\n');

/** 新增 / 编辑 MCP server。 */
export function McpServerForm({
  open,
  initial,
  agents,
  lockedTarget,
  onClose,
  onSubmit,
}: McpServerFormProps) {
  const { t } = useTranslation();
  const [name, setName] = useState('');
  const [transport, setTransport] = useState<McpTransport>('stdio');
  const [command, setCommand] = useState('');
  const [args, setArgs] = useState('');
  const [env, setEnv] = useState('');
  const [url, setUrl] = useState('');
  const [headers, setHeaders] = useState('');
  const [enabled, setEnabled] = useState(true);
  const [targets, setTargets] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const targetOptions = useMemo(
    () => (lockedTarget ? agents.filter((agent) => agent.agent === lockedTarget) : agents),
    [agents, lockedTarget],
  );

  useEffect(() => {
    if (!open) return;
    setError(null);
    const def = initial?.definition;
    setName(def?.name ?? '');
    setTransport(def?.transport ?? 'stdio');
    setCommand(def?.command ?? '');
    setArgs((def?.args ?? []).join('\n'));
    setEnv(formatPairs(def?.env ?? {}));
    setUrl(def?.url ?? '');
    setHeaders(formatPairs(def?.headers ?? {}));
    setEnabled(def?.enabled ?? true);
    if (lockedTarget) {
      setTargets([lockedTarget]);
    } else if (initial) {
      setTargets(initial.bindings.map((binding) => binding.agent));
    } else {
      setTargets(targetOptions.slice(0, 1).map((agent) => agent.agent));
    }
  }, [open, initial, lockedTarget, targetOptions]);

  if (!open) return null;

  const toggleTarget = (agent: string) => {
    setTargets((current) =>
      current.includes(agent)
        ? current.filter((item) => item !== agent)
        : [...current, agent],
    );
  };

  const submit = async () => {
    const trimmedName = name.trim();
    if (!trimmedName) {
      setError(t('mcp.nameRequired'));
      return;
    }
    if (transport === 'stdio' && !command.trim()) {
      setError(t('mcp.commandRequired'));
      return;
    }
    if (transport !== 'stdio' && !url.trim()) {
      setError(t('mcp.urlRequired'));
      return;
    }
    if (targets.length === 0) {
      setError(t('mcp.targetRequired'));
      return;
    }
    const def: McpServerDef = {
      name: trimmedName,
      transport,
      command: transport === 'stdio' ? command.trim() : undefined,
      args:
        transport === 'stdio'
          ? args
              .split('\n')
              .map((line) => line.trim())
              .filter(Boolean)
          : [],
      env: transport === 'stdio' ? parsePairs(env) : {},
      url: transport === 'stdio' ? undefined : url.trim(),
      headers: transport === 'stdio' ? {} : parsePairs(headers),
      enabled,
      extra: initial?.definition.extra,
    };
    setBusy(true);
    setError(null);
    try {
      await onSubmit(def, targets);
      onClose();
    } catch (e) {
      setError(String(e));
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
        className="w-full max-w-2xl max-h-[85vh] flex flex-col rounded-2xl bg-[var(--bg-card)] dark:bg-dark-bg-card shadow-2xl overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="px-5 pt-5 pb-3 border-b border-[var(--border-color)] dark:border-dark-border">
          <h3 className="text-lg font-bold text-[var(--text-primary)] dark:text-white">
            {initial ? t('mcp.editTitle', { name: initial.name }) : t('mcp.addTitle')}
          </h3>
        </div>

        <div className="flex-1 overflow-y-auto px-5 py-4 space-y-3">
          <label className="block">
            <span className="text-xs font-bold uppercase tracking-widest text-[var(--text-tertiary)]">
              {t('mcp.name')}
            </span>
            <input
              value={name}
              disabled={!!initial}
              onChange={(e) => setName(e.target.value)}
              placeholder="zotero-agent"
              className="mt-1 w-full px-3 py-2 rounded-xl border border-[var(--border-color)] dark:border-dark-border bg-transparent text-sm font-mono text-[var(--text-primary)] dark:text-white outline-none focus:border-[var(--accent-primary)] disabled:opacity-60"
            />
          </label>

          <div className="flex items-center gap-2">
            {(['stdio', 'sse', 'http'] as McpTransport[]).map((kind) => (
              <button
                key={kind}
                type="button"
                onClick={() => setTransport(kind)}
                className={`px-3 py-1.5 rounded-lg text-xs font-bold uppercase transition-colors ${
                  transport === kind
                    ? 'bg-[var(--accent-primary)] text-white'
                    : 'bg-black/5 dark:bg-white/10 text-[var(--text-secondary)]'
                }`}
              >
                {kind}
              </button>
            ))}
          </div>

          {transport === 'stdio' ? (
            <>
              <label className="block">
                <span className="text-xs font-bold uppercase tracking-widest text-[var(--text-tertiary)]">
                  {t('mcp.command')}
                </span>
                <input
                  value={command}
                  onChange={(e) => setCommand(e.target.value)}
                  placeholder="npx"
                  className="mt-1 w-full px-3 py-2 rounded-xl border border-[var(--border-color)] dark:border-dark-border bg-transparent text-sm font-mono text-[var(--text-primary)] dark:text-white outline-none focus:border-[var(--accent-primary)]"
                />
              </label>
              <label className="block">
                <span className="text-xs font-bold uppercase tracking-widest text-[var(--text-tertiary)]">
                  {t('mcp.args')}
                </span>
                <textarea
                  value={args}
                  onChange={(e) => setArgs(e.target.value)}
                  rows={4}
                  placeholder={'-y\nzotero-mcp'}
                  className="mt-1 w-full px-3 py-2 rounded-xl border border-[var(--border-color)] dark:border-dark-border bg-transparent text-sm font-mono text-[var(--text-primary)] dark:text-white outline-none focus:border-[var(--accent-primary)] resize-y"
                />
              </label>
              <label className="block">
                <span className="text-xs font-bold uppercase tracking-widest text-[var(--text-tertiary)]">
                  {t('mcp.env')}
                </span>
                <textarea
                  value={env}
                  onChange={(e) => setEnv(e.target.value)}
                  rows={3}
                  placeholder={'API_KEY=•••\nBASE_URL=https://...'}
                  className="mt-1 w-full px-3 py-2 rounded-xl border border-[var(--border-color)] dark:border-dark-border bg-transparent text-sm font-mono text-[var(--text-primary)] dark:text-white outline-none focus:border-[var(--accent-primary)] resize-y"
                />
              </label>
            </>
          ) : (
            <>
              <label className="block">
                <span className="text-xs font-bold uppercase tracking-widest text-[var(--text-tertiary)]">
                  URL
                </span>
                <input
                  value={url}
                  onChange={(e) => setUrl(e.target.value)}
                  placeholder="https://example.com/mcp"
                  className="mt-1 w-full px-3 py-2 rounded-xl border border-[var(--border-color)] dark:border-dark-border bg-transparent text-sm font-mono text-[var(--text-primary)] dark:text-white outline-none focus:border-[var(--accent-primary)]"
                />
              </label>
              <label className="block">
                <span className="text-xs font-bold uppercase tracking-widest text-[var(--text-tertiary)]">
                  {t('mcp.headers')}
                </span>
                <textarea
                  value={headers}
                  onChange={(e) => setHeaders(e.target.value)}
                  rows={3}
                  placeholder="Authorization=Bearer •••"
                  className="mt-1 w-full px-3 py-2 rounded-xl border border-[var(--border-color)] dark:border-dark-border bg-transparent text-sm font-mono text-[var(--text-primary)] dark:text-white outline-none focus:border-[var(--accent-primary)] resize-y"
                />
              </label>
            </>
          )}

          <label className="flex items-center gap-2 text-sm text-[var(--text-primary)] dark:text-white">
            <input
              type="checkbox"
              checked={enabled}
              onChange={(e) => setEnabled(e.target.checked)}
              className="accent-[var(--accent-primary)]"
            />
            {t('common.enable')}
          </label>

          <div>
            <span className="text-xs font-bold uppercase tracking-widest text-[var(--text-tertiary)]">
              {t('mcp.targets')}
            </span>
            <div className="mt-1 flex flex-wrap gap-2">
              {targetOptions.map((agent) => {
                const active = targets.includes(agent.agent);
                return (
                  <button
                    key={agent.agent}
                    type="button"
                    onClick={() => toggleTarget(agent.agent)}
                    className={`flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-xs font-semibold border transition-colors ${
                      active
                        ? 'border-[var(--accent-primary)] text-[var(--accent-primary)] bg-[var(--accent-primary-soft)]'
                        : 'border-[var(--border-color)] dark:border-dark-border text-[var(--text-secondary)]'
                    }`}
                  >
                    <Icon
                      name={active ? 'check_circle' : 'radio_button_unchecked'}
                      className="text-sm"
                    />
                    {agent.label}
                  </button>
                );
              })}
            </div>
          </div>
        </div>

        {error && (
          <div className="mx-5 mb-3 px-3 py-2 rounded-xl bg-red-50 dark:bg-red-500/10 text-xs text-red-600 dark:text-red-400">
            {error}
          </div>
        )}

        <div className="px-5 py-4 border-t border-[var(--border-color)] dark:border-dark-border flex justify-end gap-2">
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
            disabled={busy}
            onClick={() => void submit()}
            className={`px-4 py-2 rounded-xl text-sm font-semibold text-white bg-[var(--accent-primary)] hover:opacity-90 transition-opacity ${
              busy ? 'opacity-60 cursor-wait' : ''
            }`}
          >
            {t('common.save')}
          </button>
        </div>
      </div>
    </div>
  );
}
