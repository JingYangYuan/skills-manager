import { useEffect, useState } from 'react';
import { LOCAL_STORAGE_KEYS } from '@/constants/storageKeys';

const read = (): boolean => {
  try {
    return localStorage.getItem(LOCAL_STORAGE_KEYS.advancedMode) === 'true';
  } catch {
    return false;
  }
};

/**
 * 读取「高阶模式」开关。
 *
 * 该开关由设置页写入 localStorage，并派发一个 storage 事件做同窗口广播，
 * 这里订阅同一个事件，保证在设置页切换后 Dashboard 立即生效
 * （此前 Dashboard 侧完全没有读这个开关，导致 Agent 目录下的技能永远删不掉）。
 */
/**
 * 写入「高阶模式」开关并广播 storage 事件，让所有订阅方立即生效。
 *
 * 删除确认弹窗等危险操作入口会用它做「开启后立即继续」的解锁，
 * 避免出现「设置里打开了高阶模式，删除却依旧被挡住」的割裂体验。
 */
export function setAdvancedModeEnabled(enabled: boolean): void {
  try {
    localStorage.setItem(LOCAL_STORAGE_KEYS.advancedMode, String(enabled));
    window.dispatchEvent(
      new StorageEvent('storage', {
        key: LOCAL_STORAGE_KEYS.advancedMode,
        newValue: String(enabled),
      })
    );
  } catch {
    // localStorage 不可用（隐私模式）时静默失败，删除入口仍可用本身的安全校验兜底
  }
}

export function useAdvancedMode(): boolean {
  const [enabled, setEnabled] = useState(read);

  useEffect(() => {
    const sync = () => setEnabled(read());
    window.addEventListener('storage', sync);
    return () => window.removeEventListener('storage', sync);
  }, []);

  return enabled;
}
