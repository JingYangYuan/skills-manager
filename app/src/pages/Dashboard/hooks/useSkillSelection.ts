import { useCallback, useMemo, useState } from 'react';

/**
 * Dashboard 批量管理选择态。
 *
 * 只保存技能 id 列表：技能数据本身由 useSkillData 维护，
 * 批量操作前再从最新 skills 里解析，避免持有过期快照。
 */
export function useSkillSelection() {
  const [selectionMode, setSelectionMode] = useState(false);
  const [selectedIds, setSelectedIds] = useState<string[]>([]);

  const selectedSet = useMemo(() => new Set(selectedIds), [selectedIds]);

  const enterSelection = useCallback(() => {
    setSelectionMode(true);
  }, []);

  const clearSelection = useCallback(() => {
    setSelectedIds([]);
  }, []);

  const exitSelection = useCallback(() => {
    setSelectionMode(false);
    setSelectedIds([]);
  }, []);

  const toggleSelected = useCallback((skillId: string) => {
    setSelectedIds(prev =>
      prev.includes(skillId) ? prev.filter(id => id !== skillId) : [...prev, skillId]
    );
  }, []);

  /** 当前列表已全选时取消全选，否则全选传入的 id */
  const toggleSelectAll = useCallback((ids: string[]) => {
    setSelectedIds(prev => {
      const allSelected = ids.length > 0 && ids.every(id => prev.includes(id));
      if (allSelected) return prev.filter(id => !ids.includes(id));
      const merged = new Set(prev);
      ids.forEach(id => merged.add(id));
      return [...merged];
    });
  }, []);

  /** 退出选择模式并从选择态中移除已不存在的技能 */
  const pruneSelection = useCallback((existingIds: Set<string>) => {
    setSelectedIds(prev => prev.filter(id => existingIds.has(id)));
  }, []);

  return {
    selectionMode,
    selectedIds,
    selectedSet,
    enterSelection,
    exitSelection,
    clearSelection,
    toggleSelected,
    toggleSelectAll,
    pruneSelection,
  };
}
