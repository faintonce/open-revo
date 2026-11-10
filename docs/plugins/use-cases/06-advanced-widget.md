> 文档首页：[docs/README.md](../../README.md) · 插件与扩展：[plugins/README.md](../README.md) · [用例索引](README.md)

# 用例 6: 进阶机甲桌宠工程规范 —— 边缘吸附灵动小球、动态换肤与跨插件事件总线 (Advanced Widget)

如果你不仅想实现一个简单的状态展示板，而是希望开发一个**具备多套动态皮肤、动画序列、拖拽至屏幕边缘自动折叠为半露微光小球、并且能与其他插件联动呼应**的完整级桌面桌宠挂件，推荐采用以下标准工程解构：

## 1. 进阶工业级工程目录树 (Standard Layout)
位于 `%APPDATA%\OpenRevo\plugins\eva01-companion/`：
```text
eva01-companion/
│
├── manifest.json              # [核心清单] 插件元数据、窗口物理尺寸、权限声明与遥测传感器列表
├── index.html                 # [视窗容器] Webview 视窗入口（纯 HTML5，透明无边框）
│
├── css/
│   ├── main.css               # 主布局、毛玻璃亚克力与交互微动效
│   ├── dock.css               # 边缘停靠、折叠小球形态与展开动画关键帧 (@keyframes)
│   └── themes/                # 动态换肤样式包 (CSS 变量设计令牌)
│       ├── eva01-purple.css   # 初号机经典紫绿配色
│       ├── eva02-asuka.css    # 二号机暴走红橙配色
│       └── cyber-dark.css     # 极客透明单色微光配色
│
├── js/
│   ├── app.js                 # 主逻辑调度入口、生命周期与 DOM 初始化
│   ├── dock.js                # 屏幕边缘碰撞检测、吸附停靠与小球折叠状态机
│   ├── bus.js                 # 跨插件事件总线广播与监听器 (Cross-Plugin Bus)
│   └── skin.js                # 动态无感热换肤引擎与本地偏好持久化
│
└── assets/                    # 静态资产仓库
    ├── animations/            # 动态动画序列（Lottie JSON / APNG / WebM 透明动图 / SVG 矢量）
    │   ├── idle.svg           # 待机平稳呼吸形态
    │   ├── boost.svg          # 强冷暴走喷射形态
    │   └── ball-orbit.svg     # 边缘贴边小球微动特效
    ├── sounds/                # 拟物音效（可选，如点击、暴走提示音）
    │   └── switch.mp3
    └── preview.png            # 插件在 OpenRevo 插件管理面板中展示的缩略封面
```

## 2. 清单权威声明 (`manifest.json`)
```json
{
  "id": "eva01-companion",
  "name": "EVA-01 初号机灵动桌宠",
  "version": "1.2.0",
  "author": "OpenRevo Community",
  "description": "异形拟态机甲桌宠，支持边缘吸附小球、初号机/二号机动态换肤与跨插件事件联动。",
  "type": "widget",
  "entry": "index.html",
  "window": {
    "width": 240,
    "height": 180,
    "transparent": true,
    "alwaysOnTop": true,
    "resizable": false
  },
  "telemetry": {
    "enabled": true,
    "desiredIntervalMs": 1500,
    "sensors": [
      "cpu_temp",
      "cpu_fan_rpm",
      "fan_boost"
    ]
  },
  "permissions": [
    "telemetry:read",
    "fan:control",
    "power:control"
  ]
}
```

## 3. 边缘吸附与折叠小球机制 (`dock.js` + `dock.css`)
利用 Tauri 的 `getCurrentWebviewWindow()` 动态嗅探窗口物理外框与当前屏幕边界。当挂件被拖拽至屏幕右侧边界（阈值 30px）时，窗口物理尺寸无缝收缩为 `48×48`，DOM 自动切换为吸附小球样式；当鼠标移入小球判定区时，自动弹开恢复完整面板：

```javascript
// js/dock.js
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { currentMonitor } from '@tauri-apps/api/window';
import { LogicalSize, LogicalPosition } from '@tauri-apps/api/dpi';

const appWindow = getCurrentWebviewWindow();
let isDocked = false;

// 拖拽释放后触发贴边校验
export async function checkEdgeDock() {
  const monitor = await currentMonitor();
  if (!monitor) return;

  const scale = monitor.scaleFactor;
  const winPos = await appWindow.outerPosition();
  const screenW = monitor.size.width / scale;
  const curX = winPos.x / scale;

  // 靠近右侧边缘（小于 30px）触发吸附折叠
  if (screenW - (curX + 240) < 30) {
    isDocked = true;
    document.body.classList.add('docked-bubble');
    // 窗口平滑收缩为 48x48 悬浮微球，贴紧屏幕最右侧
    await appWindow.setSize(new LogicalSize(48, 48));
    await appWindow.setPosition(new LogicalPosition(screenW - 48, winPos.y / scale));
  }
}

// 悬停展开完整看板
export function initDockEvents() {
  const bubble = document.getElementById('bubble-hitbox');
  bubble.addEventListener('pointerenter', async () => {
    if (isDocked) {
      document.body.classList.remove('docked-bubble');
      await appWindow.setSize(new LogicalSize(240, 180));
    }
  });

  document.getElementById('core-panel').addEventListener('pointerleave', async () => {
    if (isDocked) {
      document.body.classList.add('docked-bubble');
      await appWindow.setSize(new LogicalSize(48, 48));
    }
  });
}
```

```css
/* css/dock.css */
:root {
  --core-accent: #6366f1;
  --core-glow: rgba(99, 102, 241, 0.4);
}

/* 正常展开看板 */
#core-panel {
  width: 240px;
  height: 180px;
  border-radius: 16px;
  background: rgba(15, 23, 42, 0.85);
  backdrop-filter: blur(20px);
  border: 1px solid rgba(255, 255, 255, 0.1);
  transition: all 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
}

/* 边缘折叠小球形态 */
body.docked-bubble #core-panel {
  display: none;
}
body.docked-bubble #bubble-hitbox {
  display: flex;
  width: 48px;
  height: 48px;
  border-radius: 50%;
  background: rgba(15, 23, 42, 0.9);
  border: 2px solid var(--core-accent);
  box-shadow: 0 0 16px var(--core-glow);
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transform: translateX(10px); /* 贴边半露小球胶囊 */
}
```

## 4. 多套皮肤无感热切换与资产组织 (`skin.js`)
皮肤切换采用 CSS Design Tokens 隔离，绝不在 JS 中写死任何样式数值：
```javascript
// js/skin.js
const THEMES = ['eva01-purple', 'eva02-asuka', 'cyber-dark'];

export function applyTheme(themeName) {
  // 1. 动态挂载主题样式表
  const link = document.getElementById('theme-stylesheet');
  link.href = `css/themes/${themeName}.css`;

  // 2. 动态切换动画形象
  const animImg = document.getElementById('companion-avatar');
  animImg.src = `assets/animations/${themeName}-idle.svg`;

  // 3. 持久化到插件本地独立存储
  localStorage.setItem('companion_theme', themeName);
}
```

## 5. 跨插件与主视窗公频事件联动 (`bus.js`)
利用 OpenRevo 底层统一的 IPC 事件总线，插件之间可以跨视窗自由互通：

- **插件 A（桌宠挂件）在硬件剧烈过热时向公频广播自定义事件**：
  ```javascript
  import { emit } from '@tauri-apps/api/event';

  // 结温突破 85℃ 时广播“初号机暴走”联动信号
  export async function broadcastBerserk(temp) {
    await emit('plugin:eva01:berserk', {
      source: 'eva01-companion',
      temp,
      action: 'fan_boost_suggested'
    });
  }
  ```

- **插件 B（键盘流光插件 / AI 语音助手插件）监听并同步响应**：
  ```javascript
  import { listen } from '@tauri-apps/api/event';

  // 监听插件 A 的广播
  await listen('plugin:eva01:berserk', (event) => {
    const { temp } = event.payload;
    console.log(`收到初号机暴走联动: 当前结温 ${temp}℃`);

    // 联动 1: 键盘立即闪烁猩红警报流光
    // 联动 2: AI 看板娘切换为惊恐动画帧
  });
  ```


---

## 相关页面

- 上一页：[用例 5：完全重写前端！自建独立替代控制台 (Alternative Shell)](05-alternative-shell.md)
- 下一页：[用例 7：主窗口主题皮肤与插槽化布局重排 (Skin Plugin —— 一切皆插件)](07-skin-plugin.md)
- 返回用例索引：[经典实战用例集](README.md)
