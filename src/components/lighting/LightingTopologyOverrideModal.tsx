import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Cpu, Check, RotateCcw, Loader2 } from 'lucide-react';

export interface LightingHardwareOverride {
  enabled: boolean;
  keyboard: string;   // "auto" | "per_key" | "four_zone" | "single_zone" | "single_white" | "none"
  front_bar: string;  // "auto" | "present" | "none"
  ambient: string;    // "auto" | "vent" | "shoulder" | "none"
  logo: string;       // "auto" | "letter" | "shield" | "none"
}

interface LightingRuntimeStatus {
  keyboard_engine: string;
  keyboard_online: boolean;
  keyboard_active: boolean;
  keyboard_fps: number;
  lightbar_online: boolean;
  lightbar_active: boolean;
  hinge_online: boolean;
  hinge_active: boolean;
  logo_online: boolean;
  logo_active: boolean;
  is_sleeping: boolean;
  hardware_override: LightingHardwareOverride;
  detected_baseline: LightingHardwareOverride;
}

interface LightingTopologyOverrideModalProps {
  isOpen: boolean;
  onClose: () => void;
  onApplied?: () => void;
}

export const LightingTopologyOverrideModal: React.FC<LightingTopologyOverrideModalProps> = ({
  isOpen,
  onClose,
  onApplied,
}) => {
  const [config, setConfig] = useState<LightingHardwareOverride>({
    enabled: false,
    keyboard: 'auto',
    front_bar: 'auto',
    ambient: 'auto',
    logo: 'auto',
  });

  const [baseline, setBaseline] = useState<LightingHardwareOverride>({
    enabled: false,
    keyboard: 'auto',
    front_bar: 'auto',
    ambient: 'auto',
    logo: 'auto',
  });

  const [isApplying, setIsApplying] = useState(false);
  const [applyStep, setApplyStep] = useState('');
  const [errorMessage, setErrorMessage] = useState('');

  useEffect(() => {
    if (isOpen) {
      setErrorMessage('');
      invoke<LightingRuntimeStatus>('get_lighting_runtime_status')
        .then((status) => {
          if (status.hardware_override) {
            setConfig(status.hardware_override);
          }
          if (status.detected_baseline) {
            setBaseline(status.detected_baseline);
          }
        })
        .catch((err) => {
          console.error('[LightingTopology] Failed to fetch runtime status:', err);
        });
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const handleSelect = (category: keyof LightingHardwareOverride, value: string) => {
    setErrorMessage('');
    setConfig((prev) => ({
      ...prev,
      enabled: true,
      [category]: value,
    }));
  };

  const handleResetToAuto = () => {
    setErrorMessage('');
    setConfig({
      enabled: false,
      keyboard: 'auto',
      front_bar: 'auto',
      ambient: 'auto',
      logo: 'auto',
    });
  };

  const handleApply = async () => {
    setIsApplying(true);
    setErrorMessage('');
    setApplyStep('Выполняется безопасный «отжиг» драйвера и сброс оборудования...');

    try {
      // 保证用户纠偏时如果全是 auto 则将 enabled 置为 false
      const isAllAuto =
        config.keyboard === 'auto' &&
        config.front_bar === 'auto' &&
        config.ambient === 'auto' &&
        config.logo === 'auto';

      const finalCfg: LightingHardwareOverride = {
        ...config,
        enabled: !isAllAuto,
      };

      await invoke('apply_lighting_hardware_override', {
        overrideCfg: finalCfg,
        override_cfg: finalCfg,
      });

      setConfig(finalCfg);
      setApplyStep('Аппаратный канал успешно привязан!');
      setTimeout(() => {
        setIsApplying(false);
        onClose();
        if (onApplied) onApplied();
      }, 500);
    } catch (err) {
      console.error('[LightingTopology] Failed to apply override:', err);
      setIsApplying(false);
      setApplyStep('');
      setErrorMessage(String(err));
    }
  };

  // 计算差异项
  const effectiveKb = config.keyboard === 'auto' ? baseline.keyboard : config.keyboard;
  const effectiveFront = config.front_bar === 'auto' ? baseline.front_bar : config.front_bar;
  const effectiveAmb = config.ambient === 'auto' ? baseline.ambient : config.ambient;
  const effectiveLogo = config.logo === 'auto' ? baseline.logo : config.logo;

  let diffCount = 0;
  if (config.keyboard !== 'auto' && config.keyboard !== baseline.keyboard) diffCount++;
  if (config.front_bar !== 'auto' && config.front_bar !== baseline.front_bar) diffCount++;
  if (config.ambient !== 'auto' && config.ambient !== baseline.ambient) diffCount++;
  if (config.logo !== 'auto' && config.logo !== baseline.logo) diffCount++;

  return (
    <div className="sys-modal-backdrop" onClick={onClose}>
      <div
        className="sys-modal-card"
        style={{ maxWidth: '840px', width: '94%', overflow: 'visible' }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="sys-modal-header">
          <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
            <div
              className="sys-modal-icon-badge"
              style={{ background: 'var(--status-info-dim)', color: 'var(--status-info)' }}
            >
              <Cpu size={18} />
            </div>
            <div>
              <div className="sys-modal-title" style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <span>Исправление топологии оборудования подсветки</span>
                <span
                  style={{
                    fontSize: '10.5px',
                    fontWeight: 600,
                    color: 'var(--status-info)',
                    background: 'var(--status-info-dim)',
                    padding: '1px 7px',
                    borderRadius: 'var(--radius-sm)',
                    border: '1px solid var(--status-info)',
                  }}
                >
                  Адаптация для ультратонких
                </span>
              </div>
              <div className="sys-modal-subtitle">
                Если автоопределение не совпало с физическим оборудованием, укажите устройство вручную здесь. Наведите курсор на вариант или <strong>ⓘ</strong> выше можно посмотреть аппаратный протокол и совместимые модели.
              </div>
            </div>
          </div>
        </div>

        {/* Body: 紧凑行 4 维度拓扑 */}
        <div className="sys-modal-body" style={{ overflow: 'visible', padding: '14px 20px' }}>
          {errorMessage && (
            <div
              style={{
                marginBottom: '12px',
                padding: '8px 12px',
                borderRadius: 'var(--radius-sm)',
                background: 'var(--status-error-dim)',
                color: 'var(--status-error)',
                fontSize: '12px',
                border: '1px solid var(--status-error)',
              }}
            >
              Не удалось применить: {errorMessage}
            </div>
          )}
          <div className="topology-container">

            {/* 1. 键盘背光 */}
            <div className="topology-row">
              <div className="topology-row-left">
                <span className="topology-row-title">⌨️ 1. Оборудование подсветки клавиатуры</span>
                <span className="topology-row-hint">
                  Автоопределение: <strong>{baseline.keyboard || 'Определение'}</strong>
                </span>
              </div>
              <div className="topology-radio-group">
                {/* 全彩逐键 */}
                <div
                  className={`topology-pill ${effectiveKb === 'per_key' ? 'selected' : ''}`}
                  onClick={() => handleSelect('keyboard', 'per_key')}
                >
                  <div className="topology-dot" />
                  <span className="topology-pill-label">Полноцветный по-клавишный RGB</span>
                  {baseline.keyboard === 'per_key' && <span className="topology-badge-auto">Авто</span>}
                  <div className="topology-tip-icon">ⓘ</div>
                  <div className="topology-tooltip">
                    <div className="topology-tip-title">
                      <span>Полноцветный по-клавишный RGB</span>
                      <span className="topology-tip-chip">048D:600B / CE00</span>
                    </div>
                    <p className="topology-tip-desc">Отдельный цвет для каждой клавиши; поддержка потока BetterRGB 60 FPS, ряби на воде и пользовательских шейдеров.</p>
                    <div className="topology-tip-example">Типично: Yaoshi 16 Pro/Ultra, Canglong 16, Jiaolong GM7ZG7S(CE00) и другие топовые</div>
                  </div>
                </div>

                {/* 4区 RGB */}
                <div
                  className={`topology-pill ${effectiveKb === 'four_zone' ? 'selected' : ''}`}
                  onClick={() => handleSelect('keyboard', 'four_zone')}
                >
                  <div className="topology-dot" />
                  <span className="topology-pill-label">4-зонный RGB</span>
                  {baseline.keyboard === 'four_zone' && <span className="topology-badge-auto">Авто</span>}
                  <div className="topology-tip-icon">ⓘ</div>
                  <div className="topology-tooltip">
                    <div className="topology-tip-title">
                      <span>4-зонное аппаратное разделение RGB</span>
                      <span className="topology-tip-chip">048D:CE00</span>
                    </div>
                    <p className="topology-tip-desc">Клавиатура по горизонтали разделена на 4 физические зоны с отдельными цветами; аппаратные волны, бегущий огонь и дыхание.</p>
                    <div className="topology-tip-example">Типично: Jiaolong 16, X10 Pro, ранние версии Jiguang</div>
                  </div>
                </div>

                {/* 单区 RGB */}
                <div
                  className={`topology-pill ${effectiveKb === 'single_zone' ? 'selected' : ''}`}
                  onClick={() => handleSelect('keyboard', 'single_zone')}
                >
                  <div className="topology-dot" />
                  <span className="topology-pill-label">Однозонный RGB</span>
                  {baseline.keyboard === 'single_zone' && <span className="topology-badge-auto">Авто</span>}
                  <div className="topology-tip-icon">ⓘ</div>
                  <div className="topology-tooltip">
                    <div className="topology-tip-title">
                      <span>Однозонный RGB (вся клавиатура одним цветом)</span>
                      <span className="topology-tip-chip">EC RAM PWM</span>
                    </div>
                    <p className="topology-tip-desc">Прямое управление от EC платы: вся клавиатура меняет цвет как единое целое; одноцветное свечение, дыхание и общий радужный градиент.</p>
                    <div className="topology-tip-example">Типично: Jiguang X, Yaoshi 15 (однозонная), Kuangshi G16</div>
                  </div>
                </div>

                {/* 单色白光 */}
                <div
                  className={`topology-pill ${effectiveKb === 'single_white' ? 'selected' : ''}`}
                  onClick={() => handleSelect('keyboard', 'single_white')}
                >
                  <div className="topology-dot" />
                  <span className="topology-pill-label">Одноцветный белый</span>
                  {baseline.keyboard === 'single_white' && <span className="topology-badge-auto">Авто</span>}
                  <div className="topology-tip-icon">ⓘ</div>
                  <div className="topology-tooltip">
                    <div className="topology-tip-title">
                      <span>Одноцветная белая подсветка</span>
                      <span className="topology-tip-chip">Белая подсветка EC</span>
                    </div>
                    <p className="topology-tip-desc">Чисто белые LED: три ступени диммирования без мерцания — выкл / слабая (50%) / яркая (100%).</p>
                    <div className="topology-tip-example">Типично: Wujie 14X, Wujie 15X и другие ультрабуки</div>
                  </div>
                </div>

                {/* 禁用 */}
                <div
                  className={`topology-pill ${effectiveKb === 'none' ? 'selected' : ''}`}
                  onClick={() => handleSelect('keyboard', 'none')}
                >
                  <div className="topology-dot" />
                  <span className="topology-pill-label">Без подсветки / отключено</span>
                  <div className="topology-tip-icon">ⓘ</div>
                  <div className="topology-tooltip">
                    <div className="topology-tip-title">
                      <span>Без подсветки клавиатуры</span>
                      <span className="topology-tip-chip">Disabled</span>
                    </div>
                    <p className="topology-tip-desc">У платформы нет подсветки, либо карточка управления клавиатурой полностью скрыта в программе.</p>
                  </div>
                </div>
              </div>
            </div>

            {/* 2. 前置海岸灯条 */}
            <div className="topology-row">
              <div className="topology-row-left">
                <span className="topology-row-title">🌊 2. Передняя световая полоса</span>
                <span className="topology-row-hint">
                  Автоопределение: <strong>{baseline.front_bar === 'present' ? 'Есть световая полоса' : 'Не обнаружено'}</strong>
                </span>
              </div>
              <div className="topology-radio-group">
                <div
                  className={`topology-pill ${effectiveFront === 'present' ? 'selected' : ''}`}
                  onClick={() => handleSelect('front_bar', 'present')}
                >
                  <div className="topology-dot" />
                  <span className="topology-pill-label">Передняя световая полоса (есть)</span>
                  {baseline.front_bar === 'present' && <span className="topology-badge-auto">Авто</span>}
                  <div className="topology-tip-icon">ⓘ</div>
                  <div className="topology-tooltip">
                    <div className="topology-tip-title">
                      <span>Световая полоса Coastline на передней кромке</span>
                      <span className="topology-tip-chip">048D:7001 / EC</span>
                    </div>
                    <p className="topology-tip-desc">Тонкая многосегментная световая полоса у передней кромки прямо под тачпадом; столкновения, бегущий огонь и полноцветные волны.</p>
                    <div className="topology-tip-example">Типично: Yaoshi 16, Jiaolong 16 и другие модели с подсветкой передней кромки</div>
                  </div>
                </div>

                <div
                  className={`topology-pill ${effectiveFront === 'none' ? 'selected' : ''}`}
                  onClick={() => handleSelect('front_bar', 'none')}
                >
                  <div className="topology-dot" />
                  <span className="topology-pill-label">Нет передней полосы (скрыть)</span>
                  {baseline.front_bar === 'none' && <span className="topology-badge-auto">Авто</span>}
                  <div className="topology-tip-icon">ⓘ</div>
                  <div className="topology-tooltip">
                    <div className="topology-tip-title">
                      <span>Нет передней полосы Coastline</span>
                      <span className="topology-tip-chip">None</span>
                    </div>
                    <p className="topology-tip-desc">У платформы нет физической световой полосы на передней кромке: выбор пункта полностью удаляет «призрачную» карточку Coastline и прекращает отправку пакетов.</p>
                  </div>
                </div>
              </div>
            </div>

            {/* 3. 特色氛围灯 */}
            <div className="topology-row">
              <div className="topology-row-left">
                <span className="topology-row-title">💨 3. Особая фоновая подсветка</span>
                <span className="topology-row-hint">
                  Автоопределение: <strong>{baseline.ambient || 'Не обнаружено'}</strong>
                </span>
              </div>
              <div className="topology-radio-group">
                <div
                  className={`topology-pill ${effectiveAmb === 'vent' ? 'selected' : ''}`}
                  onClick={() => handleSelect('ambient', 'vent')}
                >
                  <div className="topology-dot" />
                  <span className="topology-pill-label">«Выхлопная» подсветка (фоновая подсветка задних решёток)</span>
                  {baseline.ambient === 'vent' && <span className="topology-badge-auto">Авто</span>}
                  <div className="topology-tip-icon">ⓘ</div>
                  <div className="topology-tooltip">
                    <div className="topology-tip-title">
                      <span>Фоновая подсветка задних вентиляционных решёток</span>
                      <span className="topology-tip-chip">048D:7000 / Hinge</span>
                    </div>
                    <p className="topology-tip-desc">Симметричные световые полосы выхлопа в стиле спорткара на задних решётках — классический хардкорный визуальный маркер платформ с СЖО.</p>
                    <div className="topology-tip-example">Типично: Kuangshi 16 Super (версия с СЖО) и другие флагманы</div>
                  </div>
                </div>

                <div
                  className={`topology-pill ${effectiveAmb === 'shoulder' ? 'selected' : ''}`}
                  onClick={() => handleSelect('ambient', 'shoulder')}
                >
                  <div className="topology-dot" />
                  <span className="topology-pill-label">Плечевые полосы «восьмёркой» (синхронные «крылья»)</span>
                  {baseline.ambient === 'shoulder' && <span className="topology-badge-auto">Авто</span>}
                  <div className="topology-tip-icon">ⓘ</div>
                  <div className="topology-tooltip">
                    <div className="topology-tip-title">
                      <span>Плечевые полосы «восьмёркой» (двойные крылья)</span>
                      <span className="topology-tip-chip">IDZ Sync</span>
                    </div>
                    <p className="topology-tip-desc">Две световые полосы «крылья», расходящиеся диагонально над клавиатурой слева и справа, пульсируют в такт подсветке клавиш.</p>
                    <div className="topology-tip-example">Типично: серия шасси IDZ (Deep Sea Ghost)</div>
                  </div>
                </div>

                <div
                  className={`topology-pill ${effectiveAmb === 'none' ? 'selected' : ''}`}
                  onClick={() => handleSelect('ambient', 'none')}
                >
                  <div className="topology-dot" />
                  <span className="topology-pill-label">Без фоновой подсветки (по умолчанию)</span>
                  {baseline.ambient === 'none' && <span className="topology-badge-auto">Авто</span>}
                  <div className="topology-tip-icon">ⓘ</div>
                  <div className="topology-tooltip">
                    <div className="topology-tip-title">
                      <span>Без дополнительной фоновой подсветки</span>
                      <span className="topology-tip-chip">None</span>
                    </div>
                    <p className="topology-tip-desc">У обычных моделей нет подсветки задних решёток или плечевых полос — карточки управления фоновой подсветкой в консоли не подгружаются.</p>
                  </div>
                </div>
              </div>
            </div>

            {/* 4. A面 LOGO */}
            <div className="topology-row">
              <div className="topology-row-left">
                <span className="topology-row-title">✨ 4. Подсветка логотипа на крышке A</span>
                <span className="topology-row-hint">
                  Автоопределение: <strong>{baseline.logo || 'Не обнаружено'}</strong>
                </span>
              </div>
              <div className="topology-radio-group">
                <div
                  className={`topology-pill ${effectiveLogo === 'letter' ? 'selected' : ''}`}
                  onClick={() => handleSelect('logo', 'letter')}
                >
                  <div className="topology-dot" />
                  <span className="topology-pill-label">Буквенный логотип (MECHREVO)</span>
                  {baseline.logo === 'letter' && <span className="topology-badge-auto">Авто</span>}
                  <div className="topology-tip-icon">ⓘ</div>
                  <div className="topology-tooltip">
                    <div className="topology-tip-title">
                      <span>Светящийся латинский логотип</span>
                      <span className="topology-tip-chip">USB NewLogo (35)</span>
                    </div>
                    <p className="topology-tip-desc">Полная подсветка надписи MECHREVO целиком, управление через отдельный USB-чип; поддерживает цветные волны и бегущую подсветку.</p>
                    <div className="topology-tip-example">Типично: GM6IX0B, ранние спецверсии Yaoshi</div>
                  </div>
                </div>

                <div
                  className={`topology-pill ${effectiveLogo === 'shield' ? 'selected' : ''}`}
                  onClick={() => handleSelect('logo', 'shield')}
                >
                  <div className="topology-dot" />
                  <span className="topology-pill-label">Щит-логотип (верхняя и нижняя «гайки»)</span>
                  {baseline.logo === 'shield' && <span className="topology-badge-auto">Авто</span>}
                  <div className="topology-tip-icon">ⓘ</div>
                  <div className="topology-tooltip">
                    <div className="topology-tip-title">
                      <span>Светящийся щит-«гайка» логотип</span>
                      <span className="topology-tip-chip">Эффекты подсветки через EC платы</span>
                    </div>
                    <p className="topology-tip-desc">Светящийся щит/гайка из двух половин (верх и низ), прямое управление EC платы; одноцветное свечение, дыхание и смешение трёх цветов.</p>
                    <div className="topology-tip-example">Типично: Canglong, вся серия Yaoshi, Jiguang X, Jiaolong</div>
                  </div>
                </div>

                <div
                  className={`topology-pill ${effectiveLogo === 'none' ? 'selected' : ''}`}
                  onClick={() => handleSelect('logo', 'none')}
                >
                  <div className="topology-dot" />
                  <span className="topology-pill-label">Нет подсветки логотипа на крышке A</span>
                  {baseline.logo === 'none' && <span className="topology-badge-auto">Авто</span>}
                  <div className="topology-tip-icon">ⓘ</div>
                  <div className="topology-tooltip">
                    <div className="topology-tip-title">
                      <span>Нет подсветки логотипа на крышке A</span>
                      <span className="topology-tip-chip">None</span>
                    </div>
                    <p className="topology-tip-desc">Крышка A — только металлическая наклейка, глянцевая печать или без подсветки; карточка эффектов крышки A скрыта во избежание ошибочных действий.</p>
                  </div>
                </div>
              </div>
            </div>

          </div>
        </div>

        {/* Footer Actions */}
        <div className="sys-modal-actions" style={{ justifyContent: 'space-between', padding: '12px 20px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
            <button
              type="button"
              className="toggle-btn"
              onClick={handleResetToAuto}
              style={{ fontSize: '11px', display: 'flex', alignItems: 'center', gap: '5px' }}
            >
              <RotateCcw size={12} />
              <span>Вернуть полностью автоматическое определение</span>
            </button>
            <span style={{ fontSize: '11px', color: 'var(--text-dim)' }}>
              {diffCount === 0 ? (
                'Совпадает с автоопределением'
              ) : (
                <span style={{ color: 'var(--status-warning)', fontWeight: 600 }}>
                  {diffCount}  исправлено вручную
                </span>
              )}
            </span>
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <button
              type="button"
              className="toggle-btn"
              onClick={onClose}
              disabled={isApplying}
            >
              Отмена
            </button>
            <button
              type="button"
              className="toggle-btn active-indigo"
              onClick={handleApply}
              disabled={isApplying}
              style={{ fontWeight: 700 }}
            >
              {isApplying ? (
                <>
                  <Loader2 size={13} className="animate-spin" />
                  <span>{applyStep || 'Применение...'}</span>
                </>
              ) : (
                <>
                  <Check size={13} />
                  <span>Сохранить и заново инициализировать оборудование</span>
                </>
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
