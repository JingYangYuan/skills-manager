import { useCallback, useRef, useState } from 'react';
import { mcpMarketplaceApi } from '@/api/tauri';
import type { McpInstallRecord, McpMarketplaceItem } from '@/types';

/**
 * MCP 市场数据：内置精选、Registry 列表/搜索、安装记录。
 *
 * 列表请求用递增的 request id 丢弃过期响应，避免快速输入时旧结果覆盖新结果。
 */
export function useMcpMarketplace() {
  const [featured, setFeatured] = useState<McpMarketplaceItem[]>([]);
  const [featuredLoading, setFeaturedLoading] = useState(false);
  const [featuredError, setFeaturedError] = useState<string | null>(null);

  const [items, setItems] = useState<McpMarketplaceItem[]>([]);
  const [listLoading, setListLoading] = useState(false);
  const [listError, setListError] = useState<string | null>(null);
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [listQuery, setListQuery] = useState('');
  const [loadingMore, setLoadingMore] = useState(false);

  const [installs, setInstalls] = useState<McpInstallRecord[]>([]);
  const [installsLoading, setInstallsLoading] = useState(false);
  const [installsError, setInstallsError] = useState<string | null>(null);

  const featuredRequest = useRef(0);
  const listRequest = useRef(0);
  const installsRequest = useRef(0);

  const loadFeatured = useCallback(async () => {
    const id = ++featuredRequest.current;
    setFeaturedLoading(true);
    setFeaturedError(null);
    try {
      const result = await mcpMarketplaceApi.featured();
      if (id === featuredRequest.current) setFeatured(result);
    } catch (error) {
      if (id === featuredRequest.current) setFeaturedError(String(error));
    } finally {
      if (id === featuredRequest.current) setFeaturedLoading(false);
    }
  }, []);

  const loadInstalls = useCallback(async () => {
    const id = ++installsRequest.current;
    setInstallsLoading(true);
    setInstallsError(null);
    try {
      const result = await mcpMarketplaceApi.installs();
      if (id === installsRequest.current) setInstalls(result);
    } catch (error) {
      if (id === installsRequest.current) setInstallsError(String(error));
    } finally {
      if (id === installsRequest.current) setInstallsLoading(false);
    }
  }, []);

  const loadFirstPage = useCallback(async (query: string) => {
    const id = ++listRequest.current;
    setListLoading(true);
    setListError(null);
    setListQuery(query);
    try {
      const page = await mcpMarketplaceApi.list(query || null, null, 40);
      if (id !== listRequest.current) return;
      setItems(page.items);
      setNextCursor(page.nextCursor ?? null);
    } catch (error) {
      if (id !== listRequest.current) return;
      setItems([]);
      setNextCursor(null);
      setListError(String(error));
    } finally {
      if (id === listRequest.current) {
        setListLoading(false);
        // 新的首屏请求会作废正在进行的“加载更多”，这里一并复位，
        // 避免旧请求的 finally 因 id 不匹配而让按钮永远停在 loading。
        setLoadingMore(false);
      }
    }
  }, []);

  const loadMore = useCallback(async () => {
    if (!nextCursor || listLoading || loadingMore) return;
    const id = listRequest.current;
    setLoadingMore(true);
    setListError(null);
    try {
      const page = await mcpMarketplaceApi.list(listQuery || null, nextCursor, 40);
      if (id !== listRequest.current) return;
      setItems((current) => {
        const seen = new Set(current.map((item) => item.registryName));
        return [...current, ...page.items.filter((item) => !seen.has(item.registryName))];
      });
      setNextCursor(page.nextCursor ?? null);
    } catch (error) {
      if (id === listRequest.current) setListError(String(error));
    } finally {
      if (id === listRequest.current) setLoadingMore(false);
    }
  }, [listQuery, listLoading, loadingMore, nextCursor]);

  return {
    featured,
    featuredLoading,
    featuredError,
    items,
    listLoading,
    listError,
    nextCursor,
    listQuery,
    loadingMore,
    installs,
    installsLoading,
    installsError,
    loadFeatured,
    loadInstalls,
    loadFirstPage,
    loadMore,
  };
}
