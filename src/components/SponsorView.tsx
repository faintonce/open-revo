import React, { useState } from 'react';
import { Heart, Star, Coffee, ExternalLink, Check, Copy, ShieldAlert } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import alipayQr from '../assets/sponsor/alipay.jpg';
import wechatQr from '../assets/sponsor/wechat.jpg';

export const SponsorView: React.FC = () => {
  const [copied, setCopied] = useState(false);
  const githubUrl = 'https://github.com/faintonce/open-revo'; // 开源项目地址

  const handleCopyLink = async () => {
    let success = false;
    try {
      await invoke('write_clipboard', { text: githubUrl });
      success = true;
    } catch {
      try {
        if (navigator.clipboard && navigator.clipboard.writeText) {
          await navigator.clipboard.writeText(githubUrl);
          success = true;
        }
      } catch (err) {
        console.warn('navigator.clipboard failed:', err);
      }
      if (!success) {
        try {
          const textArea = document.createElement('textarea');
          textArea.value = githubUrl;
          textArea.style.position = 'fixed';
          textArea.style.left = '-999999px';
          document.body.appendChild(textArea);
          textArea.focus();
          textArea.select();
          success = document.execCommand('copy');
          document.body.removeChild(textArea);
        } catch (err) {
          console.error('execCommand failed:', err);
        }
      }
    }

    if (success) {
      setCopied(true);
      setTimeout(() => setCopied(false), 2500);
    }
  };

  const handleOpenGithub = async () => {
    try {
      await invoke('open_external_url', { url: githubUrl });
    } catch {
      window.open(githubUrl, '_blank');
    }
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '16px', maxWidth: '880px', margin: '0 auto' }}>
      {/* Top Banner Card */}
      <div
        className="glass-card"
        style={{
          padding: '18px 24px',
          background: 'linear-gradient(135deg, rgba(244, 63, 94, 0.1) 0%, rgba(99, 102, 241, 0.1) 100%)',
          border: '1px solid rgba(244, 63, 94, 0.25)',
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          position: 'relative',
          overflow: 'hidden',
        }}
      >
        <div style={{ zIndex: 1 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '10px', marginBottom: '4px' }}>
            <div
              style={{
                width: '28px',
                height: '28px',
                borderRadius: '8px',
                background: 'rgba(244, 63, 94, 0.2)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                color: '#f43f5e',
              }}
            >
              <Heart size={16} color="#ff3366" fill="#ff3366" style={{ filter: 'drop-shadow(0 0 6px rgba(255, 51, 102, 0.7))' }} />
            </div>
            <h2 style={{ fontSize: '18px', fontWeight: 800, margin: 0, color: '#fff', letterSpacing: '0.02em' }}>
              支持 OpenRevo 持续开源
            </h2>
            <span
              style={{
                fontSize: '10px',
                padding: '2px 8px',
                borderRadius: '12px',
                background: 'rgba(244, 63, 94, 0.2)',
                color: '#fda4af',
                fontWeight: 700,
                border: '1px solid rgba(244, 63, 94, 0.4)',
              }}
            >
              永久免费 · 无广告 · 纯净逆向
            </span>
          </div>
          <p style={{ fontSize: '12px', color: 'var(--text-muted)', margin: 0, maxWidth: '620px', lineHeight: 1.5 }}>
            OpenRevo 历经数百小时底层逆向、WMI 通信破解与出厂 EC 深度对齐，打造纯净秒开的机械革命全功能控制台。
            软件不设任何付费门槛与收费功能，如果它切实改善了您的电脑体验，欢迎自愿打赏请作者喝杯咖啡 <Coffee size={13} className="inline text-amber-400 align-text-bottom" /> 您的认可就是项目持续迭代的最佳动力！
          </p>
        </div>

        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: '8px',
            alignItems: 'flex-end',
            zIndex: 1,
          }}
        >
          <button
            onClick={handleOpenGithub}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
              background: 'linear-gradient(135deg, #f43f5e, #e11d48)',
              border: 'none',
              color: '#fff',
              padding: '6px 14px',
              borderRadius: '6px',
              fontSize: '11px',
              fontWeight: 700,
              cursor: 'pointer',
              whiteSpace: 'nowrap',
              boxShadow: '0 0 12px rgba(244, 63, 94, 0.4)',
            }}
          >
            <Star className="w-3.5 h-3.5 fill-amber-300 text-amber-300" />
            <span>GitHub Star 点赞</span>
            <ExternalLink className="w-3 h-3 text-rose-200" />
          </button>
          <button
            onClick={handleCopyLink}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
              background: 'rgba(255, 255, 255, 0.08)',
              border: '1px solid rgba(255, 255, 255, 0.15)',
              color: '#e2e8f0',
              padding: '5px 12px',
              borderRadius: '6px',
              fontSize: '11px',
              fontWeight: 600,
              cursor: 'pointer',
              whiteSpace: 'nowrap',
              transition: 'all 0.15s ease',
            }}
          >
            {copied ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
            <span>{copied ? '已复制开源链接' : '复制 GitHub 链接'}</span>
          </button>
        </div>
      </div>

      {/* Anti-Resale & Free Open Source Warning Banner */}
      <div
        className="glass-card"
        style={{
          padding: '12px 18px',
          background: 'linear-gradient(90deg, rgba(239, 68, 68, 0.12) 0%, rgba(220, 38, 38, 0.05) 100%)',
          border: '1px solid rgba(239, 68, 68, 0.3)',
          borderRadius: '8px',
          display: 'flex',
          alignItems: 'center',
          gap: '12px',
        }}
      >
        <ShieldAlert size={22} color="#ef4444" style={{ flexShrink: 0 }} />
        <div style={{ fontSize: '11px', color: '#fca5a5', lineHeight: 1.5 }}>
          <strong style={{ color: '#fff', marginRight: '6px' }}>【反盗版与防倒卖郑重声明】</strong>
          OpenRevo 为 100% 完全免费、公开透明的开源软件，从未授权任何第三方付费分发、加壳改名或二次倒卖。若您通过海鲜市场、贴吧等任何渠道付费购买，请立即向平台申请全额退款并举报不良商家！
        </div>
      </div>

      {/* QR Codes Grid */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: '16px' }}>
        {/* WeChat Pay Card */}
        <div
          className="glass-card"
          style={{
            padding: '20px',
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            background: 'linear-gradient(180deg, rgba(16, 185, 129, 0.08) 0%, rgba(15, 23, 42, 0.4) 100%)',
            border: '1px solid rgba(16, 185, 129, 0.25)',
            position: 'relative',
          }}
        >
          <div className="oem-bracket-tl" />
          <div className="oem-bracket-tr" />
          <div className="oem-bracket-bl" />
          <div className="oem-bracket-br" />

          {/* Header */}
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '14px' }}>
            <div
              style={{
                width: '24px',
                height: '24px',
                borderRadius: '50%',
                background: '#07c160',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                color: '#fff',
                fontSize: '12px',
                fontWeight: 900,
              }}
            >
              微
            </div>
            <span style={{ fontSize: '15px', fontWeight: 700, color: '#10b981', letterSpacing: '0.02em' }}>
              微信支付 (WeChat Pay)
            </span>
          </div>

          {/* QR Container */}
          <div
            style={{
              background: '#ffffff',
              padding: '12px',
              borderRadius: '12px',
              boxShadow: '0 8px 24px rgba(16, 185, 129, 0.25)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              maxWidth: '240px',
              width: '100%',
              aspectRatio: '1/1',
              overflow: 'hidden',
            }}
          >
            <img
              src={wechatQr}
              alt="微信收款码 - 鱼子饭"
              style={{
                width: '100%',
                height: '100%',
                objectFit: 'contain',
                borderRadius: '6px',
                userSelect: 'none',
              }}
            />
          </div>

          {/* Account Label */}
          <div style={{ marginTop: '14px', textAlign: 'center' }}>
            <div style={{ fontSize: '13px', fontWeight: 700, color: '#f1f5f9' }}>
              收款人：鱼子饭 (*昱)
            </div>
            <div style={{ fontSize: '11px', color: 'var(--text-muted)', marginTop: '2px' }}>
              打开微信 [扫一扫] · 任意金额支持
            </div>
          </div>
        </div>

        {/* Alipay Card */}
        <div
          className="glass-card"
          style={{
            padding: '20px',
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            background: 'linear-gradient(180deg, rgba(59, 130, 246, 0.08) 0%, rgba(15, 23, 42, 0.4) 100%)',
            border: '1px solid rgba(59, 130, 246, 0.25)',
            position: 'relative',
          }}
        >
          <div className="oem-bracket-tl" />
          <div className="oem-bracket-tr" />
          <div className="oem-bracket-bl" />
          <div className="oem-bracket-br" />

          {/* Header */}
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '14px' }}>
            <div
              style={{
                width: '24px',
                height: '24px',
                borderRadius: '50%',
                background: '#1677ff',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                color: '#fff',
                fontSize: '12px',
                fontWeight: 900,
              }}
            >
              支
            </div>
            <span style={{ fontSize: '15px', fontWeight: 700, color: '#60a5fa', letterSpacing: '0.02em' }}>
              支付宝 (Alipay)
            </span>
          </div>

          {/* QR Container */}
          <div
            style={{
              background: '#ffffff',
              padding: '12px',
              borderRadius: '12px',
              boxShadow: '0 8px 24px rgba(59, 130, 246, 0.25)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              maxWidth: '240px',
              width: '100%',
              aspectRatio: '1/1',
              overflow: 'hidden',
            }}
          >
            <img
              src={alipayQr}
              alt="支付宝收款码 - 鱼子饭"
              style={{
                width: '100%',
                height: '100%',
                objectFit: 'contain',
                borderRadius: '6px',
                userSelect: 'none',
              }}
            />
          </div>

          {/* Account Label */}
          <div style={{ marginTop: '14px', textAlign: 'center' }}>
            <div style={{ fontSize: '13px', fontWeight: 700, color: '#f1f5f9' }}>
              收款人：鱼子饭 (*昱)
            </div>
            <div style={{ fontSize: '11px', color: 'var(--text-muted)', marginTop: '2px' }}>
              打开支付宝 [扫一扫] · 任意金额支持
            </div>
          </div>
        </div>
      </div>

      {/* Bottom Notes & FAQ */}
      <div
        className="glass-card"
        style={{
          padding: '14px 18px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          background: 'rgba(0, 0, 0, 0.3)',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <Coffee className="w-4 h-4 text-amber-400" />
          <div style={{ fontSize: '11px', color: 'var(--text-dim)' }}>
            <strong>打赏须知：</strong> 赞赏完全基于自愿原则，无论是否赞助均享有软件 100% 完整功能与终身免费更新，感谢每一位关注与支持机械革命开源生态的玩家！
          </div>
        </div>
      </div>
    </div>
  );
};
