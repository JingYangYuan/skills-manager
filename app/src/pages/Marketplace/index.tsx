import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import PageHeader from '@/components/PageHeader';
import SkillDownload from './SkillDownload';
import McpMarket from './McpMarket';
import {
  MarketplaceKindSwitch,
  type MarketplaceKind,
} from './components/MarketplaceKindSwitch';

const KIND_STORAGE_KEY = 'skills-manager:marketplace-kind';

function readStoredKind(): MarketplaceKind {
  try {
    return sessionStorage.getItem(KIND_STORAGE_KEY) === 'mcp' ? 'mcp' : 'skill';
  } catch {
    return 'skill';
  }
}

/**
 * 市场外壳：技能 / MCP 两种类型共用同一个入口。
 *
 * SkillDownload 始终保持挂载，避免切到 MCP 再切回来时丢失下载进度；
 * McpMarket 首次进入后保持挂载，同样保留搜索与滚动状态。
 */
export default function Marketplace() {
  const { t } = useTranslation();
  const [kind, setKind] = useState<MarketplaceKind>(readStoredKind);
  const [mcpMounted, setMcpMounted] = useState(kind === 'mcp');

  useEffect(() => {
    if (kind === 'mcp') setMcpMounted(true);
    try {
      sessionStorage.setItem(KIND_STORAGE_KEY, kind);
    } catch {
      /* ignore quota / private mode */
    }
  }, [kind]);

  return (
    <div className="h-full flex flex-col">
      <PageHeader
        icon="storefront"
        title={t('marketplace.title')}
        leftAccessory={<MarketplaceKindSwitch kind={kind} onChange={setKind} />}
      />
      <div className="flex-1 min-h-0 relative">
        <div
          className="h-full"
          style={{
            display: kind === 'skill' ? 'block' : 'none',
            contentVisibility: kind === 'skill' ? 'auto' : 'hidden',
          }}
        >
          <SkillDownload />
        </div>
        {mcpMounted && (
          <div
            className="h-full"
            style={{
              display: kind === 'mcp' ? 'block' : 'none',
              contentVisibility: kind === 'mcp' ? 'auto' : 'hidden',
            }}
          >
            <McpMarket />
          </div>
        )}
      </div>
    </div>
  );
}
