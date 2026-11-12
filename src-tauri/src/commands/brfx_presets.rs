// OpenRevo - Built-in BRFX Presets (Clean UTF-8)

pub const KAMEHAMEHA: &str = r#"// ==========================================
// @name: 龟派气功 vs 终极闪光 (Kamehameha)
// @author: OpenRevo Community
// @version: 2.0.0
// @description: 经典龙珠对波：左侧幽蓝激光贯穿、右侧终极闪光对冲，两头极细束流在中心激烈相撞拉锯，四射高能电流电弧
// @tags: 龙珠, 对波, 龟派气功, 终极闪光, 电流闪电
// @default_speed: 1.2
// @default_scale: 1.0
// @default_direction: 0
// @default_reactive: false
// @default_color: #00F0FF
// ==========================================

fn render_key(kx, ky, ctx) {
    let t_cycle = (ctx.t * ctx.speed * 1.0) % 6.5;
    let y0 = 2.5;
    let dist_y = abs(ky - y0);
    let clash_x = 9.0 + 1.1 * sin(ctx.t * 12.0) * (0.7 + 0.3 * cos(ctx.t * 23.0));

    let r = 0.0;
    let g = 0.0;
    let b = 0.0;

    if t_cycle < 1.2 {
        // [阶段 1: 0.0s ~ 1.2s: 蓝色龟派气功迅猛飞掠 <|=====O]
        let head_x = -1.0 + (t_cycle / 1.0) * 23.0;
        let tail_x = 0.0;

        if kx <= head_x && kx >= tail_x {
            // 龙头 O (巨大高能白热等离子球，饱满圆润且核心极度爆亮，球体扩大覆盖上下多行)
            let d_head = sqrt((kx - head_x) * (kx - head_x) + dist_y * dist_y);
            let head_radius = 2.5;
            if d_head < head_radius {
                if d_head < 1.1 {
                    // 核心极度白热爆亮
                    r += 255.0;
                    g += 255.0;
                    b += 255.0;
                } else {
                    let outer = (1.0 - (d_head - 1.1) / (head_radius - 1.1));
                    r += 170.0 * outer;
                    g += 235.0 * outer;
                    b += 255.0 * outer;
                }
            }

            // 细激光柱 ===== (极高对比度，核心白热，光晕幽蓝)
            let core_h = 0.60;
            if dist_y < core_h {
                let core = pow(1.0 - dist_y / core_h, 2.0);
                r += 240.0 * core;
                g += 250.0 * core;
                b += 255.0 * core;
            }
            let halo_h = 1.25;
            if dist_y < halo_h {
                let halo = 1.0 - dist_y / halo_h;
                let ripple = sin(kx * 4.0 - ctx.t * 30.0) * 0.15;
                r += 20.0 * halo;
                g += clamp(140.0 * halo + 40.0 * ripple, 0.0, 255.0);
                b += 255.0 * halo;
            }

            // 尾部喷口 <| (锥形扩散)
            if kx < 2.5 {
                let flare = max(0.0, 1.0 - kx / 2.5) * max(0.0, 1.0 - dist_y / 1.8);
                r += 60.0 * flare;
                g += 180.0 * flare;
                b += 255.0 * flare;
            }
        }
    } else if t_cycle < 2.4 {
        // [阶段 2: 1.2s ~ 2.4s: 金色终极闪光迅猛反冲 O=====|>]
        let p2_t = t_cycle - 1.2;
        let head_x = 19.5 - (p2_t / 1.0) * 23.0;
        let tail_x = 18.5;

        if kx >= head_x && kx <= tail_x {
            // 龙头 O (巨大高能白金等离子球，饱满圆润且核心极度爆亮)
            let d_head = sqrt((kx - head_x) * (kx - head_x) + dist_y * dist_y);
            let head_radius = 2.5;
            if d_head < head_radius {
                if d_head < 1.1 {
                    // 核心极度白金爆亮
                    r += 255.0;
                    g += 255.0;
                    b += 220.0;
                } else {
                    let outer = (1.0 - (d_head - 1.1) / (head_radius - 1.1));
                    r += 255.0 * outer;
                    g += 215.0 * outer;
                    b += 70.0 * outer;
                }
            }

            // 细金色激光柱 ===== (核心纯白，外圈辉煌金)
            let core_h = 0.60;
            if dist_y < core_h {
                let core = pow(1.0 - dist_y / core_h, 2.0);
                r += 255.0 * core;
                g += 250.0 * core;
                b += 160.0 * core;
            }
            let halo_h = 1.25;
            if dist_y < halo_h {
                let halo = 1.0 - dist_y / halo_h;
                let ripple = sin(kx * 4.0 + ctx.t * 30.0) * 0.15;
                r += clamp(255.0 * halo + 40.0 * ripple, 0.0, 255.0);
                g += 160.0 * halo;
                b += 20.0 * halo;
            }

            // 尾部喷口 |> (锥形扩散)
            if kx > 16.0 {
                let flare = max(0.0, (kx - 16.0) / 2.5) * max(0.0, 1.0 - dist_y / 1.8);
                r += 255.0 * flare;
                g += 180.0 * flare;
                b += 40.0 * flare;
            }
        }
    } else if t_cycle < 5.4 {
        // [阶段 3: 2.4s ~ 5.4s: 两头对波·中间相撞电流四射 ===O  💥  O===]
        let p3_t = t_cycle - 2.4;
        let left_tip = clash_x;
        let right_tip = clash_x;
        if p3_t < 0.4 {
            left_tip = min(clash_x, -1.0 + (p3_t / 0.4) * (clash_x + 1.0));
            right_tip = max(clash_x, 19.5 - (p3_t / 0.4) * (19.5 - clash_x));
        }

        // 左侧高能幽蓝束 ===O (带巨大高能白热等离子龙头)
        if kx <= left_tip {
            let d_head_l = sqrt((kx - left_tip) * (kx - left_tip) + dist_y * dist_y);
            let head_radius = 2.5;
            if d_head_l < head_radius {
                if d_head_l < 1.1 {
                    r += 255.0;
                    g += 255.0;
                    b += 255.0;
                } else {
                    let outer = (1.0 - (d_head_l - 1.1) / (head_radius - 1.1));
                    r += 170.0 * outer;
                    g += 235.0 * outer;
                    b += 255.0 * outer;
                }
            }

            let core_h = 0.50;
            if dist_y < core_h {
                let core = pow(1.0 - dist_y / core_h, 2.0);
                r += 240.0 * core;
                g += 255.0 * core;
                b += 255.0 * core;
            }
            let halo_h = 1.05;
            if dist_y < halo_h {
                let halo = 1.0 - dist_y / halo_h;
                r += 10.0 * halo;
                g += 130.0 * halo;
                b += 255.0 * halo;
            }
        }

        // 右侧高能金光束 O=== (带巨大高能白金等离子龙头)
        if kx >= right_tip {
            let d_head_r = sqrt((kx - right_tip) * (kx - right_tip) + dist_y * dist_y);
            let head_radius = 2.5;
            if d_head_r < head_radius {
                if d_head_r < 1.1 {
                    r += 255.0;
                    g += 255.0;
                    b += 220.0;
                } else {
                    let outer = (1.0 - (d_head_r - 1.1) / (head_radius - 1.1));
                    r += 255.0 * outer;
                    g += 215.0 * outer;
                    b += 70.0 * outer;
                }
            }

            let core_h = 0.50;
            if dist_y < core_h {
                let core = pow(1.0 - dist_y / core_h, 2.0);
                r += 255.0 * core;
                g += 245.0 * core;
                b += 180.0 * core;
            }
            let halo_h = 1.05;
            if dist_y < halo_h {
                let halo = 1.0 - dist_y / halo_h;
                r += 255.0 * halo;
                g += 150.0 * halo;
                b += 10.0 * halo;
            }
        }

        // 碰撞爆发中心 💥 (激荡拉锯、爆闪光球)
        let d_clash = sqrt((kx - clash_x) * (kx - clash_x) + dist_y * dist_y * 1.69);
        if p3_t >= 0.4 {
            let vibe = 0.85 + 0.15 * sin(ctx.t * 45.0);
            let r_clash = 2.5 + 0.3 * sin(ctx.t * 18.0);
            if d_clash < r_clash {
                let clash_glow = (1.0 - d_clash / r_clash) * vibe;
                r += 255.0 * clash_glow * 1.3;
                g += 255.0 * clash_glow * 1.3;
                b += 255.0 * clash_glow * 1.3;
            }

            // 垂直压力等离子喷射 (上下行被撞击冲击波刷亮)
            let dx_clash = abs(kx - clash_x);
            if dx_clash < 1.6 {
                let vert = (1.0 - dx_clash / 1.6) * max(0.0, 1.0 - dist_y / 2.8) * vibe;
                if kx <= clash_x {
                    r += 120.0 * vert;
                    g += 220.0 * vert;
                    b += 255.0 * vert;
                } else {
                    r += 255.0 * vert;
                    g += 210.0 * vert;
                    b += 70.0 * vert;
                }
            }

            // 电流四射 (超高频粒子电弧跳跃)
            let spark_seed = sin(kx * 19.37 + ky * 41.29 + ctx.t * 55.0);
            if spark_seed > 0.82 && d_clash < 7.5 {
                let arc_pwr = ((spark_seed - 0.82) / 0.18) * (1.0 - d_clash / 7.5);
                if kx <= clash_x {
                    r += 190.0 * arc_pwr;
                    g += 235.0 * arc_pwr;
                    b += 255.0 * arc_pwr;
                } else {
                    r += 255.0 * arc_pwr;
                    g += 220.0 * arc_pwr;
                    b += 90.0 * arc_pwr;
                }
            }
        }
    } else {
        // [阶段 4: 5.4s ~ 6.5s: 临界大爆炸与环形激波消退]
        let p4_t = t_cycle - 5.4;
        let ring_r = p4_t * 16.0;
        let d_center = sqrt((kx - 9.0) * (kx - 9.0) + dist_y * dist_y);
        let ring_dist = abs(d_center - ring_r);
        let fade = max(0.0, 1.0 - p4_t / 1.1);

        if ring_dist < 1.6 {
            let burst = (1.0 - ring_dist / 1.6) * fade;
            r += 255.0 * burst;
            g += 240.0 * burst;
            b += 255.0 * burst;
        }
        if d_center < 3.0 {
            let ember = (1.0 - d_center / 3.0) * fade * fade;
            r += 160.0 * ember;
            g += 190.0 * ember;
            b += 255.0 * ember;
        }
    }

    return [clamp(r, 0.0, 255.0), clamp(g, 0.0, 255.0), clamp(b, 0.0, 255.0)];
}
"#;

pub const CYBER_LOTUS: &str = r#"// ==========================================
// @name: 赛博青莲 (Cyber Lotus)
// @author: OpenRevo Community
// @version: 2.0.0
// @description: 6瓣极坐标青莲舒展盛开，伴随量子花蕊脉冲与荷塘微澜；小键盘区及外围静谧点缀时不时闪烁的冰晶/翡翠/流萤星芒
// @tags: 极坐标, 莲花, 赛博朋克, 星芒, 优雅
// @default_speed: 1.0
// @default_scale: 1.0
// @default_direction: 0
// @default_reactive: false
// @default_color: #00FFFF
// ==========================================

fn render_key(kx, ky, ctx) {
    let center_x = 7.0;
    let center_y = 2.5;
    let dx = (kx - center_x) / ctx.scale;
    let dy = ((ky - center_y) * 1.5) / ctx.scale;
    let r_dist = sqrt(dx * dx + dy * dy);
    let angle = atan2(dy, dx);

    let t = ctx.t * ctx.speed;
    let spin = t * 0.6;

    // 1. 莲花形态数学：6 瓣舒展主莲花，带优雅呼吸脉动
    let breathe = sin(t * 1.5) * 0.4;
    let petal_lobe = cos(6.0 * (angle + spin));
    let petal_r = 3.6 + breathe + 1.6 * petal_lobe;
    let inner_lobe = cos(3.0 * (angle - spin * 1.3));
    let inner_r = 1.8 + 0.5 * inner_lobe;

    // 2. 荷塘夜色基础底光 (防止小键盘及全键盘边缘死黑，确保夜间按键清晰可用)
    let r = 4.0;
    let g = 14.0;
    let b = 26.0;

    // 3. 莲花主体渲染
    if r_dist < inner_r {
        // [花蕊莲芯]：量子脉冲暖金翡翠核心
        let core_w = 1.0 - r_dist / inner_r;
        let pulse = 0.8 + 0.2 * sin(t * 4.0);
        r += 240.0 * core_w * pulse;
        g += 210.0 * core_w * pulse;
        b += 60.0 * core_w;
    } else if r_dist < petal_r {
        // [舒展莲瓣]：幽蓝向翠碧渐变，瓣尖带晶莹高光
        let petal_pos = (r_dist - inner_r) / (petal_r - inner_r);
        let edge_dist = abs(r_dist - petal_r);
        let edge_glow = smoothstep(1.2, 0.0, edge_dist);
        r += 10.0 + 80.0 * petal_pos + 120.0 * edge_glow;
        g += 160.0 + 95.0 * (1.0 - petal_pos);
        b += 255.0 * (1.0 - petal_pos * 0.3);
    } else {
        // [荷塘微澜]：从花瓣向外散开的柔和涟漪
        let ripple = sin(r_dist * 1.6 - t * 3.5);
        if ripple > 0.6 {
            let rip_pwr = ((ripple - 0.6) / 0.4) * max(0.0, 1.0 - r_dist / 12.0) * 0.4;
            r += 10.0 * rip_pwr;
            g += 150.0 * rip_pwr;
            b += 240.0 * rip_pwr;
        }
    }

    // 4. [时不时的星星点点] (Spontaneous Twinkling Starlight)
    // 专为小键盘区 (kx >= 13.0) 及全键盘外围打造的随机闪烁星芒
    let star_freq = 1.1 + sin(kx * 17.31 + ky * 31.17) * 0.5;
    let star_phase = kx * 4.83 + ky * 9.27;
    let p1 = sin(t * star_freq + star_phase);
    let p2 = sin(t * (star_freq * 0.47) + star_phase * 1.618);
    let spark = p1 * p2;

    if spark > 0.60 {
        let star_intensity = pow((spark - 0.60) / 0.40, 2.2);
        let star_seed = sin(kx * 37.19 + ky * 53.81);
        if star_seed > 0.3 {
            // 冰晶青白星光
            r += 140.0 * star_intensity;
            g += 240.0 * star_intensity;
            b += 255.0 * star_intensity;
        } else if star_seed > -0.3 {
            // 荧光碧翠星光
            r += 30.0 * star_intensity;
            g += 255.0 * star_intensity;
            b += 190.0 * star_intensity;
        } else {
            // 暖金浮游萤火
            r += 255.0 * star_intensity;
            g += 225.0 * star_intensity;
            b += 90.0 * star_intensity;
        }
    }

    return [clamp(r, 0.0, 255.0), clamp(g, 0.0, 255.0), clamp(b, 0.0, 255.0)];
}
"#;

pub const SINGULARITY: &str = r#"// ==========================================
// @name: 引力黑洞 (Singularity)
// @author: OpenRevo Community
// @version: 1.0.0
// @description: 时空曲率吸积盘剧烈旋转，事件视界吞噬光芒，击键触发引力红移坍缩
// @tags: 宇宙, 黑洞, 吸积盘, 相对论
// @default_speed: 1.5
// @default_scale: 1.0
// @default_direction: 0
// @default_reactive: true
// @default_color: #8B5CF6
// ==========================================

fn render_key(kx, ky, ctx) {
    let center_x = 8.5;
    let center_y = 2.5;
    let dx = (kx - center_x) / ctx.scale;
    let dy = ((ky - center_y) * 1.5) / ctx.scale;
    let r_dist = sqrt(dx * dx + dy * dy);
    let angle = atan2(dy, dx);

    let t = ctx.t * ctx.speed;

    // 事件视界半径 (中心绝对全黑)
    let event_horizon = 1.2;
    if r_dist < event_horizon {
        return [0.0, 0.0, 0.0];
    }

    // 吸积盘螺旋物质流
    let spin = t * 2.5 - r_dist * 1.2;
    let spiral = sin(angle * 3.0 + spin);

    let r = 0.0;
    let g = 0.0;
    let b = 0.0;

    // 光子球强引力透镜高光环 (1.2 ~ 2.0)
    if r_dist >= event_horizon && r_dist < 2.0 {
        let ring_intensity = (1.0 - (r_dist - event_horizon) / 0.8);
        r = 255.0 * ring_intensity;
        g = 220.0 * ring_intensity;
        b = 255.0 * ring_intensity;
    } else if r_dist < 7.5 {
        // 吸积盘炽热等离子体多普勒红移
        let disk_intensity = (1.0 - (r_dist - 2.0) / 5.5) * (0.6 + 0.4 * spiral);
        let doppler = cos(angle); // 迎向观测者蓝移，背向红移
        if doppler > 0.0 {
            // 相对论蓝移喷流 (高能青紫)
            r = (180.0 + 75.0 * doppler) * disk_intensity;
            g = (100.0 + 100.0 * doppler) * disk_intensity;
            b = 255.0 * disk_intensity;
        } else {
            // 相对论红移 (深红橙黄)
            r = 255.0 * disk_intensity;
            g = (120.0 + 80.0 * doppler) * disk_intensity;
            b = 40.0 * disk_intensity;
        }
    }

    return [clamp(r, 0.0, 255.0), clamp(g, 0.0, 255.0), clamp(b, 0.0, 255.0)];
}
"#;

pub const MARQUEE: &str = r#"// ==========================================
// @name: 文字弹幕 (Cyber Marquee)
// @author: OpenRevo Community
// @version: 1.0.0
// @description: 顶天立地大字体从右向左全盘流动滚动，自适应大写英文与数字，智能避让下沉方向键保持纯正矩形排版
// @tags: 弹幕, 文字, 走马灯, 赛博朋克, 霓虹
// @default_speed: 1.0
// @default_scale: 1.0
// @default_direction: 0
// @default_reactive: false
// @default_color: #00F0FF
// @default_text: I LOVE OPEN-REVO
// ==========================================

fn render_key(kx, ky, ctx) {
    // 关键优化：忽略键盘右下方凸出的下沉三方向键（Left/Down/Right），保持大文字矩形平整不发生畸变
    if ky > 5.05 {
        return [0.0, 0.0, 0.0];
    }

    let text = if ctx.text != "" { ctx.text } else { "I LOVE OPEN-REVO" };
    let speed = ctx.speed * 6.5;
    
    // 滚动总偏移：从键盘右端向左行进
    let offset_x = kx - 19.0 + ctx.t * speed;
    let hit = sample_text(text, offset_x, ky);

    let r = 0.0;
    let g = 0.0;
    let b = 0.0;

    // 深邃宇宙幽蓝夜光背景 (保证整张键盘在暗光下轮廓清晰可用)
    r += 3.0;
    g += 8.0;
    b += 18.0;

    if hit > 0.05 {
        // 文字本体：亮丽高能霓虹色（随位置产生赛博彩虹色散）
        let hue = ((kx * 12.0 - ctx.t * 30.0) % 360.0);
        if hue < 0.0 { hue += 360.0; }
        
        let c = hsv(hue, 0.95, hit);
        
        // 核心加纯白爆亮高光
        let white_core = hit * hit * 60.0;
        r = c[0] + white_core;
        g = c[1] + white_core;
        b = c[2] + white_core;
    }

    return [clamp(r, 0.0, 255.0), clamp(g, 0.0, 255.0), clamp(b, 0.0, 255.0)];
}
"#;

pub fn get_default_presets() -> [(&'static str, &'static str); 4] {
    [
        ("kamehameha.brfx", KAMEHAMEHA),
        ("cyber_lotus.brfx", CYBER_LOTUS),
        ("singularity.brfx", SINGULARITY),
        ("marquee.brfx", MARQUEE),
    ]
}
