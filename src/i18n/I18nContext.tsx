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
  setLocale: (code: string) => Promise<void>;
  openLocalesDir: () => Promise<void>;
  exportTemplate: () => Promise<void>;
}

const DEFAULT_LOCALES: LocaleMetaOption[] = [
  { code: 'zh-CN', name: '简体中文', font_priority: 'cjk', is_builtin: true, is_external: false },
  { code: 'en-US', name: 'English', font_priority: 'segoe', is_builtin: true, is_external: false },
];

const I18nContext = createContext<I18nContextValue>({
  locale: 'zh-CN',
  availableLocales: DEFAULT_LOCALES,
  i18n: zhCN,
  localeFeedback: '',
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
      setBundle(payload.bundle as LocaleBundle);
      applyFontPriority(payload.bundle);
    } else if (payload.locale === 'en-US') {
      setBundle(enUS);
      applyFontPriority(enUS);
    } else if (payload.locale === 'zh-CN') {
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
