> 文档首页：[docs/README.md](../../README.md) · 插件与扩展：[plugins/README.md](../README.md) · [用例索引](README.md)

# 用例 13: 正交混搭架构实战 —— WebView2 富交互大控制台与 Slint 原生极速常驻桌面挂件 (Hybrid Architecture)

## 1. 业务场景与架构优势
在开发高级电池健康管理插件时，我们面临典型需求：
1. **控制台大面板**：需要展示复杂的电池循环历史趋势、充放电统计、各电芯压差表格与策略开关，使用前端 HTML/JS/CSS 或 ECharts 生态开发极为高效直观；
2. **桌面常驻微挂件**：需要随时吸附在 Windows 桌面边缘，实时显示电量百分比、充电状态与一键养护模式开关。若使用常规 Webview 视窗，即使挂起也要常驻数十兆 Chromium 内存；而采用 **Slint 声明式组件**，由内置 `slint-interpreter` 动态编译光栅化，**0MB Chromium 内存、毫秒级即时启闭**！

通过 **正交混搭架构（Orthogonal Hybrid Architecture）**，我们只需一个插件包即可完美兼得两者！

## 2. 工程目录结构
```text
plugins/hybrid-battery/
├── manifest.json         # 混搭能力声明清单
├── widget.slint          # 原生 Slint 声明式桌面常驻挂件
└── dist/
    └── index.html        # WebView2 驱动的富交互大控制台页面
```

## 3. 清单声明 (`manifest.json`)
```json
{
  "id": "hybrid-battery",
  "name": "电池卫士 (混搭示范)",
  "version": "1.0.0",
  "author": "OpenRevo Official",
  "description": "正交混搭架构示范：WebView2 富生态控制台与 Slint 原生极速常驻桌面挂件完美共存",
  "type": "app",
  "tab": {
    "type": "webview2",
    "entry": "dist/index.html",
    "title": "电池管理控制台"
  },
  "widget": {
    "type": "slint",
    "entry": "widget.slint",
    "title": "电池常驻挂件",
    "window": {
      "width": 260,
      "height": 80,
      "resizable": false,
      "decorations": true,
      "transparent": false,
      "always_on_top": true
    }
  }
}
```

## 4. 原生挂件代码 (`widget.slint`)
```slint
export component MainWindow inherits Window {
    title: "电池常驻挂件 (Slint 极速光栅化)";
    width: 260px;
    height: 80px;
    background: #0f131a;
    default-font-size: 12px;

    in-out property <int> battery_percent: 78;
    in-out property <bool> is_charging: true;
    in-out property <bool> eco_mode: true;

    Rectangle {
        border-radius: 8px;
        border-width: 1px;
        border-color: #1e293b;
        background: #0f131a;

        VerticalLayout {
            padding: 10px;
            spacing: 6px;

            HorizontalLayout {
                alignment: space-between;

                HorizontalLayout {
                    spacing: 8px;
                    // Battery outline
                    Rectangle {
                        width: 24px;
                        height: 14px;
                        border-radius: 3px;
                        border-width: 1.5px;
                        border-color: root.battery_percent > 20 ? #10b981 : #ef4444;
                        y: (parent.height - self.height) / 2;

                        Rectangle {
                            x: 2px;
                            y: 2px;
                            width: (parent.width - 4px) * (root.battery_percent / 100);
                            height: parent.height - 4px;
                            border-radius: 1px;
                            background: root.battery_percent > 20 ? #10b981 : #ef4444;
                        }
                    }

                    Text {
                        text: root.battery_percent + "%";
                        font-size: 13px;
                        font-weight: 700;
                        color: #f1f5f9;
                        vertical-alignment: center;
                    }

                    Text {
                        text: root.is_charging ? "⚡ 充电中" : "🔋 电池供电";
                        font-size: 11px;
                        color: root.is_charging ? #38bdf8 : #94a3b8;
                        vertical-alignment: center;
                    }
                }

                // Slint Tag Badge
                Rectangle {
                    height: 18px;
                    border-radius: 4px;
                    background: #1e293b;
                    HorizontalLayout {
                        padding-left: 6px;
                        padding-right: 6px;
                        Text {
                            text: "Slint 原生";
                            font-size: 10px;
                            color: #38bdf8;
                            vertical-alignment: center;
                        }
                    }
                }
            }

            // Bottom Actions / Status
            HorizontalLayout {
                alignment: space-between;

                Text {
                    text: root.eco_mode ? "养护上限: 80% (健康保护)" : "养护上限: 100% (满电模式)";
                    font-size: 11px;
                    color: #64748b;
                    vertical-alignment: center;
                }

                Rectangle {
                    width: 72px;
                    height: 22px;
                    border-radius: 4px;
                    background: root.eco_mode ? #0284c7 : #334155;

                    TouchArea {
                        mouse-cursor: pointer;
                        clicked => {
                            root.eco_mode = !root.eco_mode;
                        }
                    }

                    Text {
                        text: root.eco_mode ? "养护中" : "常规模式";
                        font-size: 10.5px;
                        font-weight: 600;
                        color: #ffffff;
                        horizontal-alignment: center;
                        vertical-alignment: center;
                    }
                }
            }
        }
    }
}
```

## 5. 运行与验证效果
- **主工作区面板**：当用户在 OpenRevo 侧边栏/导航栏点击该插件时，微内核自动将嵌入式 WebView2 视口对齐贴合至主窗口，完整呈现富前端页面；
- **桌面挂件弹出**：当用户在插件中心开启桌面小组件或点击右上角【独立视窗】时，宿主即刻通过 `--slint-widget hybrid-battery` 拉起独立的轻量级原生窗口，无需启动任何 Chromium 实例！
- **生命周期互不干扰**：关闭控制台不会影响常驻挂件的运行，开关挂件亦不会影响控制台的配置状态。

---

## 相关页面

- 上一页：[用例 12：声明式自动化监听与规则看门狗 (Universal Watcher & Trigger Hub —— 彻底根除隐藏桌宠偷跑轮询)](12-watcher-automation.md)
- 下一页：[UI 设计语言与 Design Tokens](../ui-design.md)
- 返回用例索引：[经典实战用例集](README.md)
