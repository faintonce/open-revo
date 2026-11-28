import React, { createContext, useContext, useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import zhCN from '../../locales/zh-CN.json';
import enUS from '../../locales/en-US.json';

export type LocaleBundle = typeof zhCN;

export interface LocaleMetaOption {
  code: string;
  name: string;
  font_priority?: string;
  is_builtin?: boolean;
  is_external?: boolean;
}

interface I18nContextValue {
  locale: string;
  availableLocales: LocaleMetaOption[];
  i18n: LocaleBundle;
  localeFeedback: string;
  tr: (keyOrZh: string) => string;
  setLocale: (code: string) => Promise<void>;
  openLocalesDir: () => Promise<void>;
  exportTemplate: () => Promise<void>;
}

const DEFAULT_LOCALES: LocaleMetaOption[] = [
  { code: 'zh-CN', name: '简体中文', font_priority: 'cjk', is_builtin: true, is_external: false },
  { code: 'en-US', name: 'English', font_priority: 'segoe', is_builtin: true, is_external: false },
];

const ZH_TO_KEY_MAP = new Map<string, string>();
const PREFERRED_FIRST = ['overview', 'drawer', 'tuning', 'lighting', 'gpu', 'power', 'water', 'plugins', 'system', 'sponsor', 'shell', 'common', 'web'];
for (const mod of PREFERRED_FIRST) {
  const sec = (zhCN as Record<string, any>)[mod];
  if (!sec || typeof sec !== 'object') continue;
  for (const [k, v] of Object.entries(sec)) {
    if (typeof v === 'string' && !ZH_TO_KEY_MAP.has(v)) {
      ZH_TO_KEY_MAP.set(v, `${mod}.${k}`);
    }
  }
}
for (const [mod, sec] of Object.entries(zhCN as Record<string, any>)) {
  if (mod === '_meta' || !sec || typeof sec !== 'object') continue;
  for (const [k, v] of Object.entries(sec)) {
    if (typeof v === 'string' && !ZH_TO_KEY_MAP.has(v)) {
      ZH_TO_KEY_MAP.set(v, `${mod}.${k}`);
    }
  }
}

let activeBundleRef: Record<string, any> = zhCN as Record<string, any>;

function lookupDotPath(bundle: Record<string, any>, dotPath: string): string | undefined {
  const idx = dotPath.indexOf('.');
  if (idx <= 0) return undefined;
  const mod = dotPath.slice(0, idx);
  const key = dotPath.slice(idx + 1);
  const val = bundle?.[mod]?.[key];
  return typeof val === 'string' ? val : undefined;
}

export function trText(keyOrZh: string): string {
  if (!keyOrZh) return keyOrZh;
  const trimmed = keyOrZh.trim();
  const byKey = lookupDotPath(activeBundleRef, trimmed);
  if (byKey !== undefined) return byKey;

  const mappedKey = ZH_TO_KEY_MAP.get(trimmed) ?? ZH_TO_KEY_MAP.get(keyOrZh);
  if (mappedKey) {
    const resolved = lookupDotPath(activeBundleRef, mappedKey);
    if (resolved !== undefined) {
      if (trimmed !== keyOrZh) {
        return keyOrZh.replace(trimmed, resolved);
      }
      return resolved;
    }
  }
  return keyOrZh;
}

const I18nContext = createContext<I18nContextValue>({
  locale: 'zh-CN',
  availableLocales: DEFAULT_LOCALES,
  i18n: zhCN,
  localeFeedback: '',
  tr: trText,
  setLocale: async () => {},
  openLocalesDir: async () => {},
  exportTemplate: async () => {},
});

function applyFontPriority(bundle: any) {
  const priority = bundle?._meta?.font_priority;
  if (priority === 'segoe') {
    document.documentElement.style.setProperty(
      '--font-mech',
      "'Segoe UI', 'Microsoft YaHei UI', system-ui, sans-serif",
    );
  } else {
    document.documentElement.style.setProperty(
      '--font-mech',
      "'Microsoft YaHei UI', 'Segoe UI', system-ui, sans-serif",
    );
  }
}

export const I18nProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [locale, setLocaleState] = useState<string>('zh-CN');
  const [availableLocales, setAvailableLocales] = useState<LocaleMetaOption[]>(DEFAULT_LOCALES);
  const [bundle, setBundle] = useState<LocaleBundle>(zhCN);
  const [localeFeedback, setLocaleFeedback] = useState<string>('');

  const applyPayload = useCallback((payload: any) => {
    if (!payload) return;
    if (typeof payload.locale === 'string') {
      setLocaleState(payload.locale);
    }
    if (Array.isArray(payload.available_locales) && payload.available_locales.length > 0) {
      setAvailableLocales(payload.available_locales);
    }
    if (payload.bundle && typeof payload.bundle === 'object') {
      activeBundleRef = payload.bundle as Record<string, any>;
      setBundle(payload.bundle as LocaleBundle);
      applyFontPriority(payload.bundle);
    } else if (payload.locale === 'en-US') {
      activeBundleRef = enUS as Record<string, any>;
      setBundle(enUS);
      applyFontPriority(enUS);
    } else if (payload.locale === 'zh-CN') {
      activeBundleRef = zhCN as Record<string, any>;
      setBundle(zhCN);
      applyFontPriority(zhCN);
    }
  }, []);

  useEffect(() => {
    invoke<any>('get_i18n_state')
      .then(applyPayload)
      .catch(() => {});

    const unlistenPromise = listen<any>('locale-changed', (ev) => {
      applyPayload(ev.payload);
    });
    return () => {
      unlistenPromise.then((fn) => fn()).catch(() => {});
    };
  }, [applyPayload]);

  const tr = useCallback(
    (keyOrZh: string) => {
      void bundle;
      return trText(keyOrZh);
    },
    [bundle],
  );

  const setLocale = useCallback(async (code: string) => {
    try {
      const res = await invoke<any>('set_locale', { code });
      applyPayload(res);
    } catch (e) {
      console.error('Failed to switch locale:', e);
    }
  }, [applyPayload]);

  const openLocalesDir = useCallback(async () => {
    try {
      await invoke('open_locales_dir');
    } catch (e) {
      console.error('Failed to open locales dir:', e);
    }
  }, []);

  const exportTemplate = useCallback(async () => {
    try {
      await invoke('export_i18n_template');
      setLocaleFeedback(bundle.system.template_exported);
      setTimeout(() => setLocaleFeedback(''), 4000);
    } catch (e) {
      console.error('Failed to export i18n template:', e);
    }
  }, [bundle.system.template_exported]);

  return (
    <I18nContext.Provider
      value={{
        locale,
        availableLocales,
        i18n: bundle,
        localeFeedback,
        tr,
        setLocale,
        openLocalesDir,
        exportTemplate,
      }}
    >
      {children}
    </I18nContext.Provider>
  );
};

export function useI18n() {
  return useContext(I18nContext);
}
