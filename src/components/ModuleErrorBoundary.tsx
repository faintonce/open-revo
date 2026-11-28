import { Component, ErrorInfo, ReactNode } from 'react';
import { AlertTriangle, RefreshCw, ChevronDown, ChevronRight, Home } from 'lucide-react';
import { trText } from '../i18n/I18nContext';

interface Props {
  moduleName: string;
  children: ReactNode;
  onNavigateToOverview?: () => void;
}

interface State {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
  showDetails: boolean;
}

export class ModuleErrorBoundary extends Component<Props, State> {
  public state: State = {
    hasError: false,
    error: null,
    errorInfo: null,
    showDetails: false,
  };

  public static getDerivedStateFromError(error: Error): Partial<State> {
    return { hasError: true, error };
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error(`[ModuleErrorBoundary] Error in module "${this.props.moduleName}":`, error, errorInfo);
    this.setState({ errorInfo });
  }

  private handleReset = () => {
    this.setState({ hasError: false, error: null, errorInfo: null });
  };

  private toggleDetails = () => {
    this.setState((prev) => ({ showDetails: !prev.showDetails }));
  };

  public render() {
    if (this.state.hasError) {
      const { moduleName, onNavigateToOverview } = this.props;
      const { error, showDetails } = this.state;

      return (
        <div
          style={{
            margin: '20px auto',
            maxWidth: '620px',
            backgroundColor: 'rgba(23, 27, 38, 0.85)',
            backdropFilter: 'blur(12px)',
            border: '1px solid rgba(239, 68, 68, 0.35)',
            borderRadius: '12px',
            padding: '24px',
            boxShadow: '0 12px 30px rgba(0, 0, 0, 0.4), 0 0 20px rgba(239, 68, 68, 0.1)',
            color: '#e2e8f0',
            fontFamily: 'inherit',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '12px', marginBottom: '14px' }}>
            <div
              style={{
                width: '40px',
                height: '40px',
                borderRadius: '8px',
                backgroundColor: 'rgba(239, 68, 68, 0.15)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                color: '#ef4444',
                flexShrink: 0,
              }}
            >
              <AlertTriangle size={22} />
            </div>
            <div>
              <div style={{ fontSize: '15px', fontWeight: 600, color: '#f87171' }}>
                {moduleName} 运行异常 (模块已安全隔离)
              </div>
              <div style={{ fontSize: '12px', color: '#94a3b8', marginTop: '2px' }}>
                该模块发生未捕获异常，系统已拦截并隔离故障，未影响控制中心其余功能。
              </div>
            </div>
          </div>

          <div
            style={{
              fontSize: '12px',
              color: '#cbd5e1',
              backgroundColor: 'rgba(0, 0, 0, 0.3)',
              padding: '10px 12px',
              borderRadius: '6px',
              border: '1px solid rgba(255, 255, 255, 0.05)',
              marginBottom: '16px',
              wordBreak: 'break-word',
            }}
          >
            {error?.message || trText('发生未知渲染错误 (Unknown render exception)')}
          </div>

          {/* Collapsible Details */}
          <div style={{ marginBottom: '16px' }}>
            <button
              onClick={this.toggleDetails}
              type="button"
              style={{
                background: 'none',
                border: 'none',
                color: '#64748b',
                cursor: 'pointer',
                display: 'flex',
                alignItems: 'center',
                gap: '4px',
                fontSize: '11px',
                padding: '4px 0',
              }}
            >
              {showDetails ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
              <span>{showDetails ? '收起技术堆栈详情' : trText('展开技术堆栈详情')}</span>
            </button>

            {showDetails && (
              <pre
                style={{
                  marginTop: '8px',
                  maxHeight: '160px',
                  overflowY: 'auto',
                  fontSize: '10px',
                  lineHeight: '1.4',
                  backgroundColor: 'rgba(0, 0, 0, 0.5)',
                  padding: '10px',
                  borderRadius: '6px',
                  color: '#94a3b8',
                  border: '1px solid rgba(255, 255, 255, 0.05)',
                }}
              >
                {error?.stack || trText('无可用调用堆栈')}
              </pre>
            )}
          </div>

          {/* Action buttons */}
          <div style={{ display: 'flex', gap: '10px', flexWrap: 'wrap' }}>
            <button
              onClick={this.handleReset}
              type="button"
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: '6px',
                padding: '7px 14px',
                fontSize: '12px',
                fontWeight: 500,
                color: '#ffffff',
                backgroundColor: 'rgba(239, 68, 68, 0.8)',
                border: '1px solid rgba(239, 68, 68, 0.9)',
                borderRadius: '6px',
                cursor: 'pointer',
                transition: 'all 0.15s ease',
              }}
            >
              <RefreshCw size={14} />
              <span>{trText('重试加载该模块')}</span>
            </button>

            {onNavigateToOverview && (
              <button
                onClick={onNavigateToOverview}
                type="button"
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: '6px',
                  padding: '7px 14px',
                  fontSize: '12px',
                  fontWeight: 500,
                  color: '#cbd5e1',
                  backgroundColor: 'rgba(255, 255, 255, 0.08)',
                  border: '1px solid rgba(255, 255, 255, 0.12)',
                  borderRadius: '6px',
                  cursor: 'pointer',
                  transition: 'all 0.15s ease',
                }}
              >
                <Home size={14} />
                <span>{trText('返回状态概览')}</span>
              </button>
            )}
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
