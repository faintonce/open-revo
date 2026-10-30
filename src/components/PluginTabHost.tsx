import React, { useState, useEffect, useRef, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, emit } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { getCurrentWindow, currentMonitor, LogicalSize, LogicalPosition } from '@tauri-apps/api/window';
import { HardwareState, PowerMode } from '../types/hardware';
import { CustomPluginItem } from './PluginsView';
import { AlertCircle, FolderOpen, RotateCcw, Loader2, ExternalLink, Minimize2, Play, Square } from 'lucide-react';

export interface PluginServiceStatus {
  plugin_id: string;
  status: 'running' | 'starting' | 'stopped' | 'error';
  pid?: number;
  port?: number;
  code?: string;
  message?: string;
  detail?: string;
}

interface PluginTabHostProps {
  plugin: CustomPluginItem;
  status: HardwareState;
}

export const PluginTabHost: React.FC<PluginTabHostProps> = ({ plugin, status }) => {
  const [htmlContent, setHtmlContent] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [serviceStatus, setServiceStatus] = useState<PluginServiceStatus | null>(null);
  const [serviceActionLoading, setServiceActionLoading] = useState(false);
  const [isPoppedOut, setIsPoppedOut] = useState(false);
  const iframeRef = useRef<HTMLIFrameElement>(null);

  // Sync running state of independent pop-out window
  useEffect(() => {
    let unlistenClosed: (() => void) | undefined;
    let unlistenStatus: (() => void) | undefined;

    const checkRunning = async () => {
      try {
        const running = await invoke<string[]>('get_running_widgets');
        setIsPoppedOut(running.includes(plugin.id));
      } catch (_) {}
    };

    checkRunning();

    listen<string>('widget-closed', (event) => {
      if (event.payload === plugin.id) {
        setIsPoppedOut(false);
      }
    }).then((un) => {
      unlistenClosed = un;
    });

    listen<{ plugin_id: string; active: boolean }>('widget-status-changed', (event) => {
      if (event.payload?.plugin_id === plugin.id) {
        setIsPoppedOut(!!event.payload.active);
      }
    }).then((un) => {
      unlistenStatus = un;
    });

    return () => {
      if (unlistenClosed) unlistenClosed();
      if (unlistenStatus) unlistenStatus();
    };
  }, [plugin.id]);

  // Expose OpenRevo Bridge to iframe parent context
  useEffect(() => {
    (window as any).__OPENREVO_BRIDGE__ = {
      invoke,
      listen: (event: string, handler: any) => listen(event, handler),
      emit: (event: string, payload: any) => emit(event, payload),
      getCurrentWebviewWindow: () => getCurrentWebviewWindow(),
      getCurrentWindow: () => getCurrentWindow(),
      currentMonitor: () => currentMonitor(),
      LogicalSize,
      LogicalPosition,
      pluginId: plugin.id,
      storage: {
        get: () => invoke('get_plugin_storage', { pluginId: plugin.id }),
        set: (data: any) => invoke('set_plugin_storage', { pluginId: plugin.id, data }),
        getItem: async (key: string) => {
          const all: any = await invoke('get_plugin_storage', { pluginId: plugin.id });
          return all ? all[key] : undefined;
        },
        setItem: async (key: string, val: any) => {
          const all: any = (await invoke('get_plugin_storage', { pluginId: plugin.id })) || {};
          all[key] = val;
          return invoke('set_plugin_storage', { pluginId: plugin.id, data: all });
        },
        readFile: (path: string) => invoke('read_plugin_file', { pluginId: plugin.id, path }),
        writeFile: (path: string, content: string) => invoke('write_plugin_file', { pluginId: plugin.id, path, content }),
        deleteFile: (path: string) => invoke('delete_plugin_file', { pluginId: plugin.id, path }),
        listFiles: (subDir?: string) => invoke('list_plugin_files', { pluginId: plugin.id, subDir }),
      },
    };
  }, [plugin.id]);

  // Injected runtime bridge scripts for sandboxed iframe
  const preparedHtml = useMemo(() => {
    if (!htmlContent) return '';
    const bridgeScript = `
<script>
(function() {
  var b = window.parent.__OPENREVO_BRIDGE__ || {};
  window.__OPENREVO_PLUGIN_ID__ = "${plugin.id}";
  window.OpenRevo = {
    invoke: b.invoke || function() { return Promise.reject("Bridge unavailable"); },
    listen: b.listen || function() { return Promise.resolve(function() {}); },
    emit: b.emit || function() { return Promise.resolve(); },
    pluginId: "${plugin.id}",
    storage: b.storage || {
      get: function() { return b.invoke('get_plugin_storage', { pluginId: "${plugin.id}" }); },
      set: function(data) { return b.invoke('set_plugin_storage', { pluginId: "${plugin.id}", data: data }); },
      getItem: async function(k) { var all = await b.invoke('get_plugin_storage', { pluginId: "${plugin.id}" }); return all ? all[k] : undefined; },
      setItem: async function(k, v) { var all = (await b.invoke('get_plugin_storage', { pluginId: "${plugin.id}" })) || {}; all[k] = v; return b.invoke('set_plugin_storage', { pluginId: "${plugin.id}", data: all }); },
      readFile: function(p) { return b.invoke('read_plugin_file', { pluginId: "${plugin.id}", path: p }); },
      writeFile: function(p, c) { return b.invoke('write_plugin_file', { pluginId: "${plugin.id}", path: p, content: c }); },
      deleteFile: function(p) { return b.invoke('delete_plugin_file', { pluginId: "${plugin.id}", path: p }); },
      listFiles: function(d) { return b.invoke('list_plugin_files', { pluginId: "${plugin.id}", subDir: d }); }
    }
  };
  window.__TAURI__ = {
    core: { invoke: b.invoke },
    event: { listen: b.listen, emit: b.emit },
    webviewWindow: { getCurrentWebviewWindow: b.getCurrentWebviewWindow },
    window: { getCurrentWindow: b.getCurrentWindow, currentMonitor: b.currentMonitor },
    dpi: { LogicalSize: b.LogicalSize, LogicalPosition: b.LogicalPosition }
  };
})();
</script>
`;
    if (htmlContent.includes('<head>')) {
      return htmlContent.replace('<head>', `<head>${bridgeScript}`);
    } else if (htmlContent.includes('<html>')) {
      return htmlContent.replace('<html>', `<html><head>${bridgeScript}</head>`);
    } else {
      return `${bridgeScript}${htmlContent}`;
    }
  }, [htmlContent, plugin.id]);

  const fetchHtml = async () => {
    setLoading(true);
    setError(null);
    try {
      const html = await invoke<string>('load_plugin_html', { pluginId: plugin.id });
      setHtmlContent(html);
    } catch (err: any) {
      console.error('Failed to load plugin html:', err);
      setError(typeof err === 'string' ? err : err?.message || '无法加载插件 index.html');
    } finally {
      setLoading(false);
    }
  };

  const fetchServiceStatus = async () => {
    if (!plugin.service) return;
    try {
      const res = await invoke<PluginServiceStatus>('get_plugin_service_status', { pluginId: plugin.id });
      setServiceStatus(res);
      iframeRef.current?.contentWindow?.postMessage(
        {
          type: 'OPENREVO_PLUGIN_SERVICE_STATUS',
          payload: res,
        },
        '*'
      );
    } catch (e) {
      console.warn('[PluginTabHost] Failed to query service status:', e);
    }
  };

  const handleStartService = async () => {
    if (serviceActionLoading) return;
    setServiceActionLoading(true);
    setServiceStatus((prev) => (prev ? { ...prev, status: 'starting' } : { plugin_id: plugin.id, status: 'starting' }));
    try {
      const res = await invoke<PluginServiceStatus>('start_plugin_service', { pluginId: plugin.id });
      setServiceStatus(res);
      iframeRef.current?.contentWindow?.postMessage(
        {
          type: 'OPENREVO_PLUGIN_SERVICE_STATUS',
          payload: res,
        },
        '*'
      );
    } catch (e: any) {
      const errStatus: PluginServiceStatus = {
        plugin_id: plugin.id,
        status: 'error',
        code: 'SPAWN_ERROR',
        message: typeof e === 'string' ? e : e?.message || '启动服务失败',
      };
      setServiceStatus(errStatus);
      iframeRef.current?.contentWindow?.postMessage(
        {
          type: 'OPENREVO_PLUGIN_SERVICE_STATUS',
          payload: errStatus,
        },
        '*'
      );
    } finally {
      setServiceActionLoading(false);
    }
  };

  const handleStopService = async () => {
    if (serviceActionLoading) return;
    setServiceActionLoading(true);
    try {
      const res = await invoke<PluginServiceStatus>('stop_plugin_service', { pluginId: plugin.id });
      setServiceStatus(res);
      iframeRef.current?.contentWindow?.postMessage(
        {
          type: 'OPENREVO_PLUGIN_SERVICE_STATUS',
          payload: res,
        },
        '*'
      );
    } catch (e: any) {
      console.error('[PluginTabHost] Failed to stop service:', e);
    } finally {
      setServiceActionLoading(false);
    }
  };

  useEffect(() => {
    fetchHtml();
    if (plugin.service) {
      fetchServiceStatus();
      if (plugin.service.auto_start) {
        handleStartService();
      }
    }
  }, [plugin.id]);

  // Push telemetry snapshot to iframe whenever status updates
  useEffect(() => {
    if (!iframeRef.current || !iframeRef.current.contentWindow) return;
    try {
      iframeRef.current.contentWindow.postMessage(
        {
          type: 'OPENREVO_TELEMETRY',
          payload: status,
        },
        '*'
      );
    } catch (_) {}
  }, [status]);

  // Handle postMessage from sandboxed plugin iframe
  useEffect(() => {
    const handleMessage = async (e: MessageEvent) => {
      // Security: Only accept messages intended for OpenRevo
      if (!e.data || typeof e.data !== 'object' || !e.data.action) return;

      const { action, payload } = e.data;
      try {
        switch (action) {
          case 'SET_POWER_MODE':
            if (payload?.mode) {
              await invoke('set_power_mode', { mode: payload.mode as PowerMode });
            }
            break;
          case 'SET_FAN_BOOST':
            if (typeof payload?.enable === 'boolean') {
              await invoke('set_fan_boost', { enable: payload.enable });
            }
            break;
          case 'SET_BATTERY_LIMIT':
            if (typeof payload?.limit === 'number') {
              await invoke('set_battery_limit', { limit: payload.limit });
            }
            break;
          case 'START_PLUGIN_SERVICE':
            await handleStartService();
            break;
          case 'STOP_PLUGIN_SERVICE':
            await handleStopService();
            break;
          case 'GET_PLUGIN_SERVICE_STATUS':
            await fetchServiceStatus();
            break;
          case 'GET_SHORTCUTS':
            {
              const list = await invoke('get_registered_shortcuts');
              iframeRef.current?.contentWindow?.postMessage({ type: 'OPENREVO_SHORTCUTS', payload: list }, '*');
            }
            break;
          case 'REGISTER_SHORTCUT':
            if (payload) {
              const res = await invoke('register_custom_shortcut', { binding: payload });
              iframeRef.current?.contentWindow?.postMessage({ type: 'OPENREVO_SHORTCUT_REGISTERED', success: res }, '*');
            }
            break;
          case 'UNREGISTER_SHORTCUT':
            if (payload?.id) {
              const res = await invoke('unregister_custom_shortcut', { id: payload.id });
              iframeRef.current?.contentWindow?.postMessage({ type: 'OPENREVO_SHORTCUT_UNREGISTERED', success: res }, '*');
            }
            break;
          case 'TRIGGER_SHORTCUT_ACTION':
            if (payload?.action) {
              await invoke('trigger_shortcut_action', { action: payload.action, osd: payload.osd });
            }
            break;
          case 'GET_RAW_EC_CAPABILITIES':
            {
              const caps = await invoke('get_raw_ec_capabilities');
              iframeRef.current?.contentWindow?.postMessage({ type: 'OPENREVO_RAW_EC_CAPABILITIES', payload: caps }, '*');
            }
            break;
          case 'RAW_EC_READ':
            if (payload) {
              try {
                const val = await invoke('raw_ec_read', { request: payload.request || payload });
                iframeRef.current?.contentWindow?.postMessage({ type: 'OPENREVO_RAW_EC_READ_RESULT', success: true, payload: val, reqId: payload.reqId }, '*');
              } catch (e: any) {
                iframeRef.current?.contentWindow?.postMessage({ type: 'OPENREVO_RAW_EC_READ_RESULT', success: false, error: String(e), reqId: payload.reqId }, '*');
              }
            }
            break;
          case 'RAW_EC_WRITE':
            if (payload) {
              try {
                const ok = await invoke('raw_ec_write', { request: payload.request || payload });
                iframeRef.current?.contentWindow?.postMessage({ type: 'OPENREVO_RAW_EC_WRITE_RESULT', success: true, payload: ok, reqId: payload.reqId }, '*');
              } catch (e: any) {
                iframeRef.current?.contentWindow?.postMessage({ type: 'OPENREVO_RAW_EC_WRITE_RESULT', success: false, error: String(e), reqId: payload.reqId }, '*');
              }
            }
            break;
          case 'RAW_MAILBOX_INVOKE':
            if (payload) {
              try {
                const out = await invoke('raw_mailbox_invoke', { request: payload.request || payload });
                iframeRef.current?.contentWindow?.postMessage({ type: 'OPENREVO_RAW_MAILBOX_RESULT', success: true, payload: out, reqId: payload.reqId }, '*');
              } catch (e: any) {
                iframeRef.current?.contentWindow?.postMessage({ type: 'OPENREVO_RAW_MAILBOX_RESULT', success: false, error: String(e), reqId: payload.reqId }, '*');
              }
            }
            break;
          case 'GET_PLUGIN_STORAGE':
            try {
              const res = await invoke('get_plugin_storage', { pluginId: plugin.id });
              iframeRef.current?.contentWindow?.postMessage({ type: 'OPENREVO_PLUGIN_STORAGE_RESULT', success: true, payload: res, reqId: payload?.reqId }, '*');
            } catch (e: any) {
              iframeRef.current?.contentWindow?.postMessage({ type: 'OPENREVO_PLUGIN_STORAGE_RESULT', success: false, error: String(e), reqId: payload?.reqId }, '*');
            }
            break;
          case 'SET_PLUGIN_STORAGE':
            try {
              await invoke('set_plugin_storage', { pluginId: plugin.id, data: payload?.data ?? payload });
              iframeRef.current?.contentWindow?.postMessage({ type: 'OPENREVO_PLUGIN_STORAGE_SAVED', success: true, reqId: payload?.reqId }, '*');
            } catch (e: any) {
              iframeRef.current?.contentWindow?.postMessage({ type: 'OPENREVO_PLUGIN_STORAGE_SAVED', success: false, error: String(e), reqId: payload?.reqId }, '*');
            }
            break;
          case 'READ_PLUGIN_FILE':
            try {
              const content = await invoke('read_plugin_file', { pluginId: plugin.id, path: payload?.path || payload });
              iframeRef.current?.contentWindow?.postMessage({ type: 'OPENREVO_PLUGIN_FILE_READ', success: true, payload: content, reqId: payload?.reqId }, '*');
            } catch (e: any) {
              iframeRef.current?.contentWindow?.postMessage({ type: 'OPENREVO_PLUGIN_FILE_READ', success: false, error: String(e), reqId: payload?.reqId }, '*');
            }
            break;
          case 'WRITE_PLUGIN_FILE':
            try {
              await invoke('write_plugin_file', { pluginId: plugin.id, path: payload?.path, content: payload?.content });
              iframeRef.current?.contentWindow?.postMessage({ type: 'OPENREVO_PLUGIN_FILE_WRITTEN', success: true, reqId: payload?.reqId }, '*');
            } catch (e: any) {
              iframeRef.current?.contentWindow?.postMessage({ type: 'OPENREVO_PLUGIN_FILE_WRITTEN', success: false, error: String(e), reqId: payload?.reqId }, '*');
            }
            break;
          case 'DELETE_PLUGIN_FILE':
            try {
              const deleted = await invoke('delete_plugin_file', { pluginId: plugin.id, path: payload?.path || payload });
              iframeRef.current?.contentWindow?.postMessage({ type: 'OPENREVO_PLUGIN_FILE_DELETED', success: true, payload: deleted, reqId: payload?.reqId }, '*');
            } catch (e: any) {
              iframeRef.current?.contentWindow?.postMessage({ type: 'OPENREVO_PLUGIN_FILE_DELETED', success: false, error: String(e), reqId: payload?.reqId }, '*');
            }
            break;
          case 'LIST_PLUGIN_FILES':
            try {
              const files = await invoke('list_plugin_files', { pluginId: plugin.id, subDir: payload?.subDir });
              iframeRef.current?.contentWindow?.postMessage({ type: 'OPENREVO_PLUGIN_FILES_LIST', success: true, payload: files, reqId: payload?.reqId }, '*');
            } catch (e: any) {
              iframeRef.current?.contentWindow?.postMessage({ type: 'OPENREVO_PLUGIN_FILES_LIST', success: false, error: String(e), reqId: payload?.reqId }, '*');
            }
            break;
          default:
            break;
        }
      } catch (err) {
        console.warn('[PluginTabHost] Sandbox message dispatch failed:', err);
      }
    };

    window.addEventListener('message', handleMessage);
    return () => window.removeEventListener('message', handleMessage);
  }, [plugin.id, serviceActionLoading]);

  const handleOpenPluginsDir = async () => {
    try {
      await invoke('open_plugins_dir');
    } catch (_) {}
  };

  const handlePopOut = async () => {
    try {
      await invoke('open_custom_widget', { pluginId: plugin.id });
      setIsPoppedOut(true);
    } catch (err) {
      console.error('Failed to open detached widget window:', err);
    }
  };

  const handleClosePopOut = async () => {
    try {
      await invoke('close_custom_widget', { pluginId: plugin.id });
      setIsPoppedOut(false);
    } catch (err) {
      console.error('Failed to close detached widget window:', err);
    }
  };

  if (loading) {
    return (
      <div
        style={{
          width: '100%',
          height: '100%',
          minHeight: '400px',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: '12px',
          color: 'var(--text-muted)',
        }}
      >
        <Loader2 size={28} className="animate-spin" />
        <span style={{ fontSize: '13px' }}>正在加载插件面板 [{plugin.name}]...</span>
      </div>
    );
  }

  if (error || !htmlContent) {
    return (
      <div className="sys-page">
        <div className="glass-card" style={{ maxWidth: '640px', margin: '40px auto', textAlign: 'center', padding: '32px 24px' }}>
          <div style={{ display: 'inline-flex', padding: '12px', borderRadius: '50%', background: 'rgba(239, 68, 68, 0.15)', color: 'var(--status-error)', marginBottom: '16px' }}>
            <AlertCircle size={32} />
          </div>
          <h3 style={{ margin: '0 0 8px 0', fontSize: '16px', color: 'var(--text-primary)' }}>插件加载失败</h3>
          <p style={{ margin: '0 0 20px 0', fontSize: '13px', color: 'var(--text-muted)', lineHeight: '1.5' }}>
            {error || '未在插件目录中找到 index.html 入口文件。请检查插件目录结构或查阅《OpenRevo 第三方插件开发手册》。'}
          </p>
          <div style={{ display: 'flex', justifyContent: 'center', gap: '10px' }}>
            <button onClick={fetchHtml} className="toggle-btn">
              <RotateCcw size={13} />
              <span>重试</span>
            </button>
            <button onClick={handleOpenPluginsDir} className="toggle-btn active-indigo">
              <FolderOpen size={13} />
              <span>打开插件目录</span>
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div
      style={{
        width: '100%',
        height: '100%',
        minHeight: '600px',
        display: 'flex',
        flexDirection: 'column',
        borderRadius: 'var(--radius-md)',
        overflow: 'hidden',
        background: 'var(--surface-overlay)',
        border: '1px solid var(--border-medium)',
      }}
    >
      {/* Tab Header Toolstrip with Pop-out Action */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '8px 14px',
          background: 'var(--surface-card)',
          borderBottom: '1px solid var(--border-subtle)',
          flexShrink: 0,
        }}
      >
        <span style={{ fontSize: '13px', fontWeight: 600, color: 'var(--text-primary)' }}>
          {plugin.name}
        </span>
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          {plugin.service && (
            <div className="plugin-service-group">
              <div
                className="plugin-service-pill"
                title={
                  serviceStatus?.status === 'error'
                    ? `${serviceStatus.message || '启动失败'}${serviceStatus.detail ? ` (${serviceStatus.detail})` : ''}`
                    : undefined
                }
              >
                <span className={`plugin-service-dot ${serviceStatus?.status || 'stopped'}`} />
                <span>
                  {serviceActionLoading
                    ? '处理中...'
                    : serviceStatus?.status === 'running'
                    ? `服务在线${serviceStatus.port ? ` :${serviceStatus.port}` : ''}`
                    : serviceStatus?.status === 'error'
                    ? '服务异常'
                    : '服务未运行'}
                </span>
              </div>

              {serviceStatus?.status === 'running' ? (
                <button
                  onClick={handleStopService}
                  disabled={serviceActionLoading}
                  className="toggle-btn"
                  title="停止此插件的本地伴随服务"
                  style={{ height: '22px', padding: '0 8px', fontSize: '11px' }}
                >
                  <Square size={10} />
                  <span>停止</span>
                </button>
              ) : (
                <button
                  onClick={handleStartService}
                  disabled={serviceActionLoading}
                  className="toggle-btn active-emerald"
                  title="拉起此插件的本地提权伴随服务"
                  style={{ height: '22px', padding: '0 8px', fontSize: '11px' }}
                >
                  {serviceActionLoading ? (
                    <Loader2 size={10} className="animate-spin" />
                  ) : (
                    <Play size={10} />
                  )}
                  <span>启动服务</span>
                </button>
              )}
            </div>
          )}

          <button
            onClick={fetchHtml}
            className="toggle-btn"
            title="重新加载插件"
            style={{ height: '26px', padding: '0 10px', fontSize: '11px' }}
          >
            <RotateCcw size={12} />
            <span>刷新</span>
          </button>
          {isPoppedOut ? (
            <button
              onClick={handleClosePopOut}
              className="toggle-btn active-cyan"
              title="当前正以独立原生视窗运行，点击收回并关闭独立视窗"
              style={{ height: '26px', padding: '0 10px', fontSize: '11px' }}
            >
              <Minimize2 size={12} />
              <span>收回视窗</span>
            </button>
          ) : (
            <button
              onClick={handlePopOut}
              className="toggle-btn active-indigo"
              title="以独立原生窗口弹出运行，支持在玩游戏或脱离主控制台时随时调优"
              style={{ height: '26px', padding: '0 10px', fontSize: '11px' }}
            >
              <ExternalLink size={12} />
              <span>独立视窗运行</span>
            </button>
          )}
        </div>
      </div>

      <iframe
        ref={iframeRef}
        srcDoc={preparedHtml}
        title={plugin.name}
        sandbox="allow-scripts allow-forms allow-same-origin"
        style={{
          width: '100%',
          height: '100%',
          border: 'none',
          background: 'transparent',
          flex: 1,
        }}
        onLoad={() => {
          // Push initial telemetry after iframe DOM has loaded
          try {
            iframeRef.current?.contentWindow?.postMessage(
              {
                type: 'OPENREVO_TELEMETRY',
                payload: status,
              },
              '*'
            );
            if (serviceStatus) {
              iframeRef.current?.contentWindow?.postMessage(
                {
                  type: 'OPENREVO_PLUGIN_SERVICE_STATUS',
                  payload: serviceStatus,
                },
                '*'
              );
            }
          } catch (_) {}
        }}
      />
    </div>
  );
};
