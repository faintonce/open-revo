# OpenRevo Slint 移植踩坑总结与标准规范手册

> **编制日期**：2026-11-03  
> **适用范围**：OpenRevo 全端（Slint UI 伴生应用 + Tauri/Rust 原生系统后端）  
> **核心宗旨**：记录从 WebView2 / React 前端向 Slint 原生 GUI 迁移过程中的典型陷阱、血泪教训与最佳工程范式，杜绝同类问题在后续模块（电源管理、RGB灯控、水冷等）开发中反复重蹈覆辙。

---

## 一、UI 布局与控件体系陷阱

### 1. 鼠标拉动滑块时导致整个页面被拖拽滑动（Flickable 陷阱）
- **现象**：在长页面中拖动 Slider（例如调色板或亮度滑块）时，整个界面跟着上下乱晃，无法精确控制。
- **根因**：使用了 Slint 内置的 `Flickable`。`Flickable` 默认支持触摸/鼠标左键抓取平移（Touch Drag），但桌面端管理控制软件**只应允许鼠标滚轮滚动（Mouse Wheel Scroll），绝对不能允许鼠标左键抓取拖动**。
- **解决方案**：
  在绝大多数非移动端视图中，**完全移除外层的 `Flickable`**，改用纯 `VerticalLayout`，并严格限制卡片尺寸与垂直间距（`spacing: 12px`），使主内容自然契合在窗口框架内（1120x760 桌面规格）。

---

### 2. 滑块拖动时高亮条错位、宽度与数值异常绑定（Slider 轨道约束陷阱）
- **现象**：拉动滑块时，底下的色条或滑块自身宽度跟着数值莫名放大缩小，甚至整根轨道随手柄一起移动。
- **根因**：Slint 中子元素若未显式指定 `x`、`width` 坐标约束，其布局尺寸容易被子元素内容或手柄计算反向篡改。
- **黄金布局规范**：所有自制 Slider 轨道必须显式锁定全宽与原点：
  ```slint
  // 1. 底槽轨道：必须显式锁死 x: 0 和 width: parent.width
  Rectangle {
      x: 0;
      y: (parent.height - self.height) / 2;
      width: parent.width;
      height: 4px;
      background: AppTheme.surface_pill;
      border-radius: 2px;

      // 2. 激活高亮填充条：从 0 开始，宽度由百分比比例计算
      Rectangle {
          x: 0;
          y: 0;
          width: Math.max(0px, (root.value - root.min_val) / (root.max_val - root.min_val) * parent.width);
          height: parent.height;
          background: #f59e0b; // 或渐变色
          border-radius: 2px;
      }
  }

  // 3. 拖拽手柄：居中对齐到激活条末端
  Rectangle {
      x: Math.max(0px, Math.min(parent.width - self.width,
          (root.value - root.min_val) / (root.max_val - root.min_val) * parent.width - self.width / 2));
      y: (parent.height - self.height) / 2;
      width: 14px;
      height: 14px;
      border-radius: 7px;
      background: #ffffff;
  }
  ```

---

### 3. CSS 属性妄想症（Slint 仅支持其自身的属性集）
- **现象**：使用 `border-bottom-width: 1px`、`cursor: pointer` 等 Web CSS 语法报错或被静默忽略。
- **根因**：Slint 不是浏览器引擎！它**不支持单边边框**（如 `border-bottom`），也不支持 CSS 光标写法。
- **规范**：
  - 单边分割线：通过独立的 1px 实体 `Rectangle { height: 1px; background: AppTheme.border_subtle; }` 实现。
  - 鼠标手势：必须嵌套 `TouchArea { mouse-cursor: pointer; }`。

---

### 4. 刷新率 / 按钮矩阵动态折行布局崩溃
- **现象**：外接高刷屏支持很多档位（如 59, 60, 100, 120, 144, 165, 180, 200Hz），直接写在一行时按钮被挤扁或右侧被裁切。
- **根因**：Slint 缺少 Web CSS 的 `flex-wrap: wrap` 能力。
- **规范**：在后端（`slint_window.rs`）对可用刷新率数组进行分流切片，划分为 `rates_row1` 和 `rates_row2` 两组，前端用两个紧凑的 `HorizontalLayout` 双行展示；外层容器高度不要写死，使用条件自适应。

---

## 二、状态绑定与组件通信陷阱

### 1. 弹窗与主视图之间的数据单向只读隔离（`:` vs `<=>`）
- **现象**：在二级调色弹窗里拖动滑块或切换预设，弹窗里的数值完全不跟着变，滑块跳回原位。
- **根因**：在 `gpu_view.slint` 挂载 `DisplayTuningModal` 时，误用了冒号 `:`（单向只读绑定）：
  ```slint
  // 错误写法：只读单向赋值，弹窗内部修改无法回传
  cur_temp_k: root.tuning_temp_k;
  ```
  在 Slint 中，当子组件内部需要双向同步修改父组件的属性时，**必须使用双向绑定操作符 `<=>`**：
  ```slint
  // 正确写法：双向双通道实时绑定
  cur_temp_k <=> root.tuning_temp_k;
  cur_saturation <=> root.tuning_saturation;
  ```

---

### 2. 开关状态反转与“一击双发”时序冲突
- **现象**：色彩增强开关点一下可能变成关，再点一下可能没反应，状态时常是反的。
- **根因**：在同一个 `TouchArea.clicked` 中，既在前端翻转了布尔值并派发了带有新状态的 `set_tuning_enabled(!root.tuning_enabled)`，又同时派发了一个无参数的 `toggle_tuning_enabled()`。宿主收到两次动作，发生 Race Condition。
- **规范**：
  - **开关类操作一律采用“状态驱动（State-driven）”，严禁无参 Toggle**。
  - 前端负责更新状态并派发具体的目标布尔值：
    ```slint
    TouchArea {
        clicked => {
            root.tuning_enabled = !root.tuning_enabled;
            root.set_tuning_enabled(root.tuning_enabled);
        }
    }
    ```

---

### 3. 多层透传断裂（漏接漏转）
- **链路图示**：
  ```mermaid
  flowchart LR
    A["Modal / Leaf Component"] --> B["View (gpu_view.slint)"]
    B --> C["FullDashboard (full_dashboard.slint)"]
    C --> D["MainWindow (main_window.slint)"]
    D --> E["Rust Companion (lib.rs)"]
    E --> F["IPC Stdin/Stdout (ipc.rs)"]
    F --> G["Host Tauri (slint_window.rs)"]
  ```
- **常见失误**：
  在 `DisplayTuningModal.slint` 定义了 `callback change_temp_k(int)`，在 `gpu_view.slint` 里接了，但在 `full_dashboard.slint` 或 `main_window.slint` 中遗漏了回调声明或未连线转发。Slint 编译器不会报错，但在运行时回调**在中间层直接被静默吞噬**。
- **排查口诀**：从叶子节点到 Host 后端，必须严格经过 6 级对齐：
  1. Leaf Callback 声明与触发
  2. Subview 声明与绑定
  3. Dashboard 声明与透传
  4. MainWindow 声明与桥接
  5. `lib.rs` 的 `window.on_xxx(...)` 绑定并 `ipc::send_action_to_host`
  6. `slint_window.rs` 的 `match action` 处理分支

---

### 4. 属性命名重名覆盖冲突
- **现象**：编译报错 `Cannot override property 'active-preset-name'`。
- **根因**：Slint 组件中属性名字具有作用域穿透性。若自定义功耗调优已占用 `active_preset_name: "均衡电竞"`，而在显卡色彩增强也声明 `active_preset_name: "多彩鲜艳"`，顶层汇总时会发生命名冲突。
- **规范**：针对具体子系统添加领域命名空间前缀，如 `active_display_preset_name`、`active_tuning_preset_id`、`active_color_profile_id`。

---

## 三、硬件驱动与 IPC 通信陷阱

### 1. 中文方案名与底层硬件唯一 ID 错配
- **现象**：校色下拉框选中“sRGB 色彩标准 (出厂校色)”，点击应用毫无反应。
- **根因**：前端直接把下拉框展示的中文标题发给了后端，后端在 `apply_color_profile` 检索时要求的是唯一英文标识（如 `std_srgb`、`default_native`、`std_p3`），导致在底层配置文件中检索返回 `ProfileNotFound`。
- **标准设计**：
  - 前端同时维护双数组：
    - `color_profiles_list: [string]`（用于 UI 渲染友好中文名称）
    - `color_profiles_ids: [string]`（用于与后端通信的固定 ID）
  - 点击选择时通过下标映射：
    ```slint
    clicked => {
        root.active_color_profile = p_name;
        root.active_color_profile_id = root.color_profiles_ids[idx];
    }
    ```
  - 应用时严格传 `active_color_profile_id`。

---

### 2. 后端硬件生效门禁死锁（`if cfg.enabled` 拦截微调）
- **现象**：用户在二级调色页面拉动色温滑块，没有任何变化。
- **根因**：后端为了性能加了防御判断：
  ```rust
  // 危险：如果配置中 enabled 为 false，硬件写操作被直接跳过！
  if cfg.enabled {
      display_tuning::apply_color_preset(&preset)?;
  }
  ```
  如果用户在主界面未主动开开关，但直接进入调色弹窗拖拽微调，后端的硬件下发就会被整层拦截。
- **纠正准则**：
  **“用户拖动滑块的操作本身，即代表显式的开启意图”**。在处理滑块微调与预设选择时，自动置 `cfg.enabled = true`，并无条件直接触发底层 `apply_color_preset`，实现 0-latency 毫秒级反馈。

---

## 四、平台与 Windows 原生调用陷阱

### 1. 彻底根绝 PowerShell 铁律与纯 Win32 API 替换模式
OpenRevo 恪守绝对原则：**严禁在运行时代码中调用 PowerShell 或 CMD 子进程，全量走 Win32 API / 原生 C ABI**。

| 场景 | 历史遗留错误写法 | 纯 Win32 API 正确替换方案 | 耗时对比 |
| :--- | :--- | :--- | :--- |
| **文件选择对话框** | `powershell [System.Windows.Forms.OpenFileDialog]` | Win32 `comdlg32::GetOpenFileNameW` / `GetSaveFileNameW` | 1500ms ➡️ **0.5ms** |
| **物理网卡 MAC 查询** | `powershell Get-NetAdapter -Physical` | Win32 IP Helper `iphlpapi::GetAdaptersAddresses` | 2000ms ➡️ **0.1ms** |
| **BIOS 序列号查询** | `powershell (Get-CimInstance Win32_BIOS).SerialNumber` | Win32 注册表 `HKLM\HARDWARE\DESCRIPTION\System\BIOS\SystemSerialNumber` | 1800ms ➡️ **0.01ms** |
| **屏幕面板 PnP ID** | `powershell Get-CimInstance WmiMonitorID` | Win32 GDI `EnumDisplayDevicesA` 解析 `DeviceID` | 1500ms ➡️ **0.02ms** |
| **面板独立出厂 SN** | `powershell WmiMonitorID.SerialNumberID` | Win32 注册表直读 `Enum\DISPLAY\<PnP>\...\EDID` 描述符 | 1200ms ➡️ **0.05ms** |
| **MiniLED 调光控制** | `powershell Invoke-CimMethod AcpiTest_MULong` | 原生 In-Process COM Worker (`IWbemServices::ExecMethod`) | 1200ms ➡️ **1ms** |
| **ZIP 压缩包解压** | `powershell Expand-Archive` | Windows 内置 `tar.exe -xf` 或 Rust 原生 `zip` 库 | 3000ms ➡️ **20ms** |

---

### 2. 外部插件收拢重构成原生 Slint 组件（以同方云端校色助手为例）
- **背景**：早期版本在 `plugins_sample/tongfang-cloud-icc` 中通过外部 HTML/JS iframe 演示了出厂校色文件的云端拉取，导致主应用内无法使用且依赖慢速 PowerShell。
- **重构工程闭环**：
  1. **后端引擎收拢**：将硬件特征探测全部替换为 Win32 原生 API（`GetAdaptersAddresses` + 注册表 EDID 提取），输出标准化 `CloudIccIdentity`；
  2. **云端节点自适应**：
     - **50系机型 (Aistone)**：基于 `<PnP_ID>_<BIOS_SN>` 或 `<PnP_ID>_<Panel_SN>` 并发拉取 `iccprofile.aistoneglobal.com:8001`；
     - **40系及更早机型 (Uniwill)**：基于 `<PnP_ID>_<MAC>` 并发拉取 `iccprofile.uniwill.com.tw`；
  3. **UI 原生弹窗呈现**：在 Slint 内部设计专用的 `CloudIccModal.slint`，提供硬件指纹看板、一键检索进度指示与多档位（sRGB/DCI-P3）立即应用与批量入库能力。

---

### 3. 字符串转义导致文件写入出现 Null 字节损坏
- **现象**：Rust 文件编译时被当作二进制文件报错，无法读取文本。
- **根因**：脚本在自动写入带有 Win32 Filter 结尾的过滤串（如 `\0*.json\0\0`）时，未对反斜杠做双重转义（写成了物理 `b'\x00'`），导致源代码文件被写入 Null Byte 变成损坏的二进制文件。
- **规范**：代码生成器与修补工具中，所有涉及 C 风格字符串终止符的字面量，必须严格转义为 `\\0`。

---

## 五、Slint 模块移植标准开发 Checklist

在将任何新模块（如电源管理、RGB灯效、系统设置）从 Web 迁移到 Slint 时，请按顺序检查：

- [ ] **1. 布局约束检查**
  - [ ] 是否彻底移除了非必要的 `Flickable`？
  - [ ] 所有自制 Slider 的底槽是否具备 `x: 0; width: parent.width;` 显式绝对约束？
  - [ ] 是否杜绝了 `border-bottom-width` 等无效伪 CSS 属性？
  - [ ] 多行列表容器高度是否有自适应判断？
- [ ] **2. 状态绑定检查**
  - [ ] 视图层与弹窗层之间的数据交互是否全部使用 `<=>` 双向绑定？
  - [ ] 属性命名是否具备领域前缀，避免与全局属性冲突？
- [ ] **3. 事件流一致性检查**
  - [ ] 开关类控件是否**仅派发**带明确布尔值的 `set_xxx(bool)`，杜绝无参 `toggle_xxx`？
  - [ ] 每一个 callback 是否完整贯通了 6 级管道（UI -> View -> Dash -> Main -> lib.rs -> slint_window.rs）？
- [ ] **4. 驱动与数据检查**
  - [ ] 下拉框、列表项是否区分了“用户界面显示名”与“底代码/驱动唯一 ID”？
  - [ ] 实时微调逻辑是否能无条件穿透并直达硬件驱动？
- [ ] **5. 系统能力检查**
  - [ ] 涉及文件打开/保存时，是否直接调用 Win32 原生 `comdlg32` API，杜绝慢速易死锁的 PowerShell 脚本？
  - [ ] 硬件探测、网络特征枚举与压缩解包是否 100% 遵守零 PowerShell 铁律，纯走 Win32 API？
- [ ] **6. 门禁验证**
  - [ ] 运行 `cargo check --manifest-path src-tauri/Cargo.toml` 无编译错误与冗余 Warning。
  - [ ] 运行 `powershell -ExecutionPolicy Bypass -File scripts/check_vault_leaks.ps1` 校验硬件隔离门禁通过。
