import { Component, ErrorInfo, ReactNode } from 'react';
import { AlertTriangle, RefreshCw, ChevronDown, ChevronRight, Home } from 'lucide-react';

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
                {moduleName} Сбой выполнения (модуль безопасно изолирован)
              </div>
              <div style={{ fontSize: '12px', color: '#94a3b8', marginTop: '2px' }}>
                В этом модуле возникло необработанное исключение; система перехватила и изолировала сбой — остальные функции центра управления не затронуты.
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
            {error?.message || 'Произошла неизвестная ошибка рендеринга (Unknown render exception)'}
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
              <span>{showDetails ? 'Свернуть детали стека' : 'Развернуть детали стека технологий'}</span>
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
                {error?.stack || 'Стек вызовов недоступен'}
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
              <span>Повторить загрузку модуля</span>
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
                <span>Вернуться к обзору состояния</span>
              </button>
            )}
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
