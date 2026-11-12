import React, { useState, useEffect } from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import OsdOverlay from './components/OsdOverlay';
import { WidgetHost } from './components/WidgetHost';
import './index.css';

function getRouteState() {
  const params = new URLSearchParams(window.location.search);
  const hash = window.location.hash;
  const hashParts = hash.split('/');
  // Hash takes precedence on dynamic client routing (e.g. #/widget/:id)
  const hashWidgetId = hash.startsWith('#/widget/') ? hashParts[2] : undefined;
  const injectedWidgetId = (window as any).__OPENREVO_WIDGET_ID__ as string | undefined;
  const isOsd = !!(window as any).__OPENREVO_OSD_WINDOW__ || params.get('window') === 'osd' || hash === '#/osd';
  const isWidget = !!hashWidgetId || !!injectedWidgetId || params.get('window') === 'widget' || hash.startsWith('#/widget');
  const widgetId = hashWidgetId || injectedWidgetId || params.get('id') || 'core';
  return { isOsd, isWidget, widgetId };
}

function RootRouter() {
  const [route, setRoute] = useState(getRouteState);

  useEffect(() => {
    const onHashChange = () => {
      setRoute(getRouteState());
    };
    const onSwitchWidget = (e: any) => {
      if (e?.detail?.pluginId) {
        (window as any).__OPENREVO_WIDGET_ID__ = e.detail.pluginId;
      }
      setRoute(getRouteState());
    };

    window.addEventListener('hashchange', onHashChange);
    window.addEventListener('openrevo-switch-widget', onSwitchWidget);
    return () => {
      window.removeEventListener('hashchange', onHashChange);
      window.removeEventListener('openrevo-switch-widget', onSwitchWidget);
    };
  }, []);

  useEffect(() => {
    if (route.isWidget) {
      document.documentElement.style.background = 'transparent';
      document.documentElement.style.backgroundColor = 'transparent';
      document.body.style.background = 'transparent';
      document.body.style.backgroundColor = 'transparent';
    }
  }, [route.isWidget]);

  if (route.isOsd) {
    return <OsdOverlay />;
  }
  if (route.isWidget) {
    return <WidgetHost key={route.widgetId} pluginId={route.widgetId} />;
  }
  return <App />;
}

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <RootRouter />
  </React.StrictMode>,
);
