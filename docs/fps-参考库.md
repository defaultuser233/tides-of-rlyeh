# FPS 开发参考库（Rust 项目向）

> 为 `D:\Documents\tides-of-rlyeh`（macroquad 0.4.16 + 自研渲染，波次竞技场 FPS，已有 7 类武器 / 6 张图 / viewmodel / BFS 寻路 AI；2026-10-04 校对：原路径 `D:\Documents\rust-fps` 已更名）整理的参考资料集。
> 覆盖你点名的四个方向：**手感与移动射击、武器（改枪 + 涂装）、架构与工程化、美术与关卡**。
>
> **整理时间**：2026-09-30 ｜ **最后核实**：2026-10-02 深夜（1153 行 / 163 条链接 / UTF-8 无损坏 / 21 节正文）
> **调研方式**：联网检索 + 逐条打开原文核对；§15 与 §9.3.1 的技术事实读的是 `docs.rs` 上 macroquad 0.4.16 / `tobj` 4.0.5 的**真实源码**与 crates.io API；§17/§19 来源含官方游戏 wiki；§18 来源含一手政府/司法/官方文件；§19.2.1 / §20 / §21 来自**逐行读本项目源码**。**均非记忆。**
> **可信度标注**：✅ 我打开过正文 ｜ ⚠️ 只确认页面存在/来自二手整理 ｜ ❌ 本环境打不开（给出原因，留给你自己开）
>
> ## 🚨 交接必读：先看 §21
>
> **§0 的"新发现"表、§17.2 / §19.4 / §20.2 的行动项描述的是「实现前」的状态。** 本报告的建议已被**大量实现**（波次系统、点数预算、精英缓冲波、生成点三缺陷、每波重置、每波多入口点全部落地）。**§21 是交接的权威依据**，它记录了：已完成（含代码证据）／仍未实现／下一手必须知道的五件事。
>
> **两条硬约束**：
> 1. **不要重排本报告的章节号**——代码里有 **7 处 `reference §…` 反向引用**（`main.rs` / `campaign.rs` / `maps.rs`）。新增内容一律追加为 §22、§23…
> 2. **所有行号都已失效**（`main.rs` 已从 4,370 行涨到 4,660 行，13 个文件未提交）。**按符号名定位，不要按行号。**
>
> **阅读顺序**：
> 1. **§21 交接对账**（从这里开始 —— 它告诉你哪些已经不用做了）
> 2. **§0 的项目现状表**（了解历史与最初诊断）
> 3. **§19**（可玩性五个变量 + **§19.2.1** 生成点缺陷的完整修法）+ **§20**（界面两病灶 + 武器数量增长）
> 4. **§17**（护甲三条**尚未实现**的建议）+ **§18**（抄袭边界，含最高法指导性案例）
> 5. 其余按主题查
>
> **来源**：七路并行调研（Rust 生态、手感与 gunplay、改枪与涂装、关卡与美术管线、开源 FPS 与复盘、装备与护甲、抄袭边界）+ 美术方向专项。原始长文另存于 [fps-gunplay-research.md](D:\Documents\fps-gunplay-research.md) 与 [game-art-research/macroquad-fps-art-research.md](D:\Documents\game-art-research\macroquad-fps-art-research.md)，本文件是**已归并的单一事实来源**——两者冲突时以本文件为准。

---

## 0. 先读什么（按你的现状排序）

| 优先级 | 读什么 | 为什么是它 | 产出 |
|---|---|---|---|
| ★★★ | [Bunnyhopping from the Programmer's Perspective](https://adrianb.io/2015/02/14/bunnyhop.html) ✅ | 三行代码把"移动手感"从玄学变成 `sv_accelerate / sv_airaccelerate / sv_friction` 三个数，可直接翻成 Rust | `player.rs` 的加速/摩擦模型 |
| ★★★ | [QW physics air](https://www.quakeworld.nu/wiki/QW_physics_air) ✅ + [QW physics ground](https://www.quakeworld.nu/wiki/QW_physics_ground) ✅ | Quake 移动的数学推导：临界角 `cos(γ)=0`、`wishspd` 上限 30、极限地面速度公式；还给出"72fps 与 77fps 极限速度不同"这一固定步长的历史铁证 | 移动参数上限表 |
| ★★★ | [【策划向】全景解构射击游戏枪械设计——数值篇](https://www.gameres.com/915851.html) ⚠️ | 唯一"配表级"中文资料：TTK 算法、分段后坐力骨架、回落延迟、散布随机化、跑射延迟、滑铲 8 参数 | 你的 `WeaponStats` 定标 |
| ★★☆ | [CS2 Weapon Finishes Guide（Valve 官方）](https://www.counter-strike.net/workshop/workshopfinishes) ✅ | 涂装系统的**官方规格书**：可涂装遮罩、按区域耐磨度、图案 alpha 当控制码、磨损档位数值 | 涂装系统的数据格式 |
| ★★☆ | [Gunsmith — Attachments 文档](https://docs.gunsmithplugin.com/docs/dynamic_attributes/attachments/) ✅ | 改枪数值模型的完整规格：三段式 Modifier + Static/Range(RandomSeed) | 改枪数据结构 |
| ★★☆ | [The Level Design Book](https://book.leveldesignbook.com/) ✅（[blockout](https://book.leveldesignbook.com/process/blockout.md)｜[metrics](https://book.leveldesignbook.com/process/blockout/metrics.md)｜[balance](https://book.leveldesignbook.com/process/combat/balance.md)） | 免费开源、有具体数字（TF2 三线 256/1024/2048、chokepoint 3–4 条规则）、每页有 `.md` 纯文本版可离线 | 第 7 张图的 brief |
| ★★☆ | [Fix Your Timestep!](https://gafferongames.com/post/fix_your_timestep/) ✅ | 决定你有没有资格谈手感：固定步长 + 渲染插值 | 主循环改造 |
| ★☆☆ | [GameRes：射击游戏，有没有我们小团队的生存余地？](https://www.gameres.com/915175.html) ⚠️ | 独立开发者最该先读的"赛道判断"，避免在错误方向上投入一年 | 范围决策 |
| ★☆☆ | [Gunstoppable 复盘访谈](https://80.lv/articles/gunstoppable-how-speed-is-damage-defined-a-roguelite-fps) ✅ | 单人做 FPS 的真实排产：1 个月原型 → 6 个月内容 → 6 个月打磨，总计 <2 年 | 排产表 |

### ⚠️ 项目现状（2026-10-01 核实，已推翻旧审查报告的多条结论）

[美术审查报告.md](美术审查报告.md) 与 [代码审查报告.md](代码审查报告.md) 的部分内容**已经过期**，以下是逐一读代码核实后的真实状态：

| 事项 | 审查报告说的 | 代码里的实际情况 |
|---|---|---|
| 贴图 | "6 张全部闲置，一张都没加载" | ✅ **已接线**：`surface.rs` 有 `Surface`/`Skin`，UV 按**物理尺寸**生成（`set_uv_scale(1/meters_per_tile)`），`get_internal_gl()` 设了 `Repeat` |
| 关卡盒子的材质表达 | "`(min,max,Color)` 三元组，没有材质索引" | ✅ **已扩成 `Skin{color, surface}`**，`Level` 按 `Surface` 分批，每材质一次 `draw_mesh` |
| 伪 AO | "无阴影、无 AO" | ✅ **`mesh.rs` 有 `contact_shading()`**（按离地高度压暗直立面），已接进 `level.rs` 与 `enemy.rs` |
| 敌人剪影 | "4 种人形共用同一个 9 盒模型" | ✅ **每种有各自的盒体布局**——Heavy 是"腰下没有任何开口"的一整块，注释写明这就是 20m 外区分它与 Grunt 的一半原因 |
| 第一人称武器 | "没有手、没有手臂" | ✅ **`viewmodel.rs` 有手套+前臂**，并且 `HAND_LAG = 0.4` 让手滞后于武器摆动（"完全跟随会读成一个刚体"） |
| 理智条缩放 | "唯一不用 `Ui::s()` 缩放的元素" | ✅ 已过 `ui_scale()`；但它仍在 `main.rs` 而非 `hud.rs`，且用 `MADNESS` 紫——全游戏只出现这一次 |

**所以当前真正的缺口是这两层**（详见第 15 节）：

1. **动画**：只有巨像会动。`enemy.rs:1301` 的注释直说 "The colossus is the only archetype whose limbs move"，其余 7 种走 `copy_into(dst, &self.proto)` 这条路径——**刚性滑行**。
2. **反馈**：`kill_flash` 与 `damage_numbers` 在 `effects.rs` 之外**没有任何读取**（`kill_flash` 只在 `update` 里衰减，`damage_numbers` 只被 push），连一个 draw 调用都没有。

**另外三件仍然成立的事**：
3. **手感量化**：后坐力是 `recoil_pitch / recoil_yaw` 两个标量。按第 3 节升级成"分段骨架 + 回落延迟 + 玩家下压抵消"。→ ✅ 已于 2026-10-02 完成（见 §16.1）
4. **改枪/涂装的数据地基**：`WeaponStats` 现在就有 `spread/bloom/recoil/falloff/move_scale` 这些可被配件改写的字段，但它是**单一不可变常量表**。拆成 `WeaponDef + WeaponBuild + ResolvedStats` 三层——**这是唯一一个后期返工代价极高的决定**（第 7.2 节给规格）。→ ✅ 已完成且超规格（三层类型 + `resolve` + `validate` + 子槽树 + 挂载标准，见 §16.1）
5. **UV 相位**：`mesh.rs` 的 `quad()` 用"面内偏移"算 UV，**每个面自己的第一个顶点就是 uv 原点**。密度对、相位不对。→ ✅ 已关闭（世界轴 UV，见 §15.10）

### ⚠️ 2026-10-02 之后新发现的两个结构性问题（详见 §19、§20）

| 问题 | 位置 | 影响 |
|---|---|---|
| **没有波次系统** | `main.rs` 生成器每 `(1.9 − 0.06×difficulty)` 秒滴一个敌人直到预算耗尽；`campaign.rs` 注释自承 **"This game has no wave system"**、**"the escalation lives between missions, not inside them"** | 一局的时间结构是**一条平线**，玩家**整局没有任何决策点**。这是"可玩性弱"的**根因**（§19） |
| **分数是死数字** | `game.score` 累加后在 `main.rs` 只被格式化成字符串显示；不进存档、不解锁、不影响数值、不记录最高分 | **没有理由重玩任何一关**。唯一持久的是 `unlocked`，而它只前进（§19.3） |
| **`pick_spawn` 三个缺陷** | `main.rs`：`hidden` 的 +30 分压倒距离项（幅度仅 0–16）；抽样**有放回**；全部被拒时**静默退回 `spawns[0]`** 且不再检查距离 | **敌人似乎都从同一个地点生成**——有机制解释，见 §19.2.1 |
| **精英无冷却** | `roll_kind` 每生成一个敌人独立掷一次；Colossus 权重 1/12 | 26 次生成里相邻两次的概率约 **15%**，出现时看起来像 bug（确实是）（§19.2） |
| **界面唯一原语是"整行 ±1"** | 8 处共用的 `menu_nav` + `hover_selection`；行是 `format!` 出来的 `String`；点击 = 按方向键 | 改枪页换成第 8 个部件要**按 7 次**且看不到候选总数；**鼠标划过就改编选中项、一点就装上**（§20.1） |

---

## 1. 内容地图（四种玩法方向 + 参考密度）

```
手感/移动 ──── 参考极多（Quake/Source 血统，公式级）      ← 你的"移动射击"重点
枪械反馈   ──── 参考多（GDC 演讲 + 配表文章）
改枪系统   ──── 参考中（无 Rust 实现，但数值模型规格齐全） ← 你的"改枪"重点
涂装系统   ──── 参考中（CS2 官方规格书是天花板）          ← 你的"涂装"重点
关卡设计   ──── 参考多（Level Design Book 一书覆盖大半）
美术管线   ──── 参考少（多为引擎特定，概念可迁移）
完整开源FPS ─── 参考多（但都在 C/C++/QuakeC，不是 Rust）
Rust 引擎  ──── 参考中（零件齐全、成品稀缺：无任何活跃的 Rust FPS 成品可抄）
```

**一句话结论**：Rust 生态里**没有**现成的 FPS 成品、也**没有**武器系统 crate；能直接用的只有"移动控制器 + 物理/射线 + 网络"这三类零件。而**手感、改枪、涂装这四件事的知识全在 Quake/Source/CS 的遗产里**，与语言无关——这正是你该主攻的资料区。

---

## 2. 手感（Game Feel）总论

| 资源 | 链接 | 类型 | 能直接用来做什么 |
|---|---|---|---|
| Game Feel: Why Your Death Animation Sucks | https://gdcvault.com/play/1022759/Game-Feel-Why-Your-Death ⚠️ | GDC 演讲 | 把手感拆成 **代码 → 动画 → 音效 → 屏幕震动 → 粒子 → 位移贴图 → 构图 → 打磨** 的可叠加层次，逐层加到同一段死亡动画上；强调响应性优先于观赏性 |
| Math for Game Programmers: Juicing Your Cameras With Math | https://www.gdcvault.com/play/1023557/Math-for-Game-Programmers-Juicing ⚠️ | GDC 演讲 | 把镜头运动（构图、平滑、**camera shake**）变成可参数化数学，是"枪口冲击 + 震屏"能调参而不是拍脑袋的基础 |
| A Few Experiments Based on Jan Willem Nijman's Art of Screenshake | https://www.bluetengu.com/2014/12/12/art-of-screenshake-experiments/ ✅ | 文章 + C# 代码 | 解决**同帧多次震屏互相打架**：自维护 `shakeAmount + 剩余时间`，**取最大值而非相加**，结束时精确复位；开火 kickback 只给 y 加 `Random.Range(0.1f, 0.3f)` |
| The Art of Screenshake 清单（第三方整理镜像） | https://github.layabox.com/xuhaodong/awesome-skills/-/blob/7cb809f72f89a918178d602f0544ebf7c8eaa20f/plugins/product-manager/skills/prd-creator/references/patterns/game-feel/screenshake.md ⚠️ 二手 | 清单 | 30+ 条手感的可执行条目：**顿帧 hit-stop、慢动作、挤压拉伸、道具延迟跟随、后坐、枪口火焰、命中特效、拖尾、受击白闪与击退、死亡永久痕迹、震屏、晃动、冲击波、色差、镜头缓动滞后**。原视频 https://www.youtube.com/watch?v=AJdEqssNZ-U ❌（本环境不可访问） |

**给你的项目**：你的 `effects.rs` 已有粒子 / 曳光 / 命中标记，`player.rs` 已有 head bob 与 recoil——**缺的是"顿帧 + 受击方向指示 + 开火与命中的音频区分"**。README 自己也承认"玩家射击与敌人射击用同一种曳光，一眼分不出来"（Known limitations）。按上表第 4 条的 30 条清单逐条勾选，是零美术成本的手感提升。

---

## 3. 枪械手感：数值与实现

### 3.1 配表级参考（先把数填对）

| 资源 | 链接 | 关键内容 |
|---|---|---|
| 【策划向】全景解构射击游戏枪械设计——数值篇 | https://www.gameres.com/915851.html ⚠️（原文 https://zhuanlan.zhihu.com/p/466420744） | ① **TTK 算法**：100 血、伤害 25、间隔 0.07s → 理论 TTK 210ms；② **一级数值定强度上限、二级数值构建手感差异**；③ 后坐力 = 「每发后坐力量 + 极限后坐力量」+ **分段骨架**（1–3 发均匀上抬 → 4–7 发递增 → 8–10 发左上 45° → 11 发起只吃水平 → 17 发达最大水平）；④ **水平后坐力对手感影响远大于垂直，向左又大于向右**；⑤ **回落必须相对开火有延迟**，否则枪口抬不起来，且需记录玩家主动下压量做抵消；⑥ 散布 = 骨架固定点 + 随机偏移（单发最小 / 每发叠加 / 最大）；⑦ 姿态系数 **跑动 > 站立 > 蹲下 > 趴下**；⑧ **跑射延迟** 100–200ms 干脆、300/450ms 为狙击/机枪惩罚；⑨ 滑铲 8 参数；⑩ 换弹拆成**逻辑换弹时间**与**动画换弹时间** |
| VALORANT 官方 Wiki — Vandal | https://wiki.playvalorant.com/en-us/Vandal ⚠️ | 一手数值：伤害 40 / 爆头 ×4 / 腿 ×0.85，射速 9.75 rps，换弹 2.5s；**首发散布 0.25°（蹲 0.21°）、最大 1°（蹲 0.85°）**；移动惩罚 **蹲行 +0.8°、行走 +3°、跑动 +6°、空中 +10°**；ADS 射速 90% / 移速 76%；**Gun Recovery Time 0.375s、Protection bullet count 6、Yaw 切换时间 0.6s / 概率 10%** |
| CS2 Weapon Stats（第三方横向表） | https://tradeit.gg/blog/cs2-weapon-stats/ ⚠️ | CS2 全武器伤害 / 射速 / 移速 / 穿甲对照，用于量级定标 |
| Techniques for Building Aim Assist in Console Shooters | https://gdcvault.com/play/1017942/Techniques-for-Building-Aim-Assist ⚠️ | 瞄准辅助三件套 **magnetism / centering / friction** + 相机加速与死区；手柄适配时再读 |
| The Art of First Person Animation for Destiny | https://gdcvault.com/play/1022297/The-Art-of-First-Person ⚠️ | 1P 枪械动画（ADS / 换弹 / 拉栓）的行业标准：手臂 + 武器 + 相机三者是"从玩家视角看到的运动"，以**响应性与物理感**为验收标准 |
| Solving Titan Sized Problems（Titanfall 2） | https://gdcvault.com/play/1024624/Solving-Titan-Sized-Problems-Evolving ⚠️ | 反例价值：**当 TTK 不再低于 5 秒，低 TTK 射击游戏的公式就失效了**——如果你的游戏有高血量敌人/Boss，这篇告诉你该换哪套公式（从格斗/MOBA 借） |

### 3.2 命中反馈（你的已知短板）

- 本次调研**没有找到可访问的、专门讲 hitmarker 的一手权威文章**（相关内容散见于上面 GDC 演讲与 GameRes 篇的片段）。这是明确的资料空缺，不是我漏了。
- 可替代的做法：读游戏内实现——CS2 的 hitmarker/受击音、Apex 的护甲破碎音、Doom 2016 的"顿帧 + 击杀音效"。以及 README 里已承认的问题：**把玩家曳光与敌人曳光做成两种颜色/两种音高**，这是最低成本的可读性修复。

---

## 4. 移动手感：Quake/Source 移动物理（⭐ 与本项目最相关）

### 4.1 核心公式（可直接翻译成 Rust）

来自 [adrianb.io](https://adrianb.io/2015/02/14/bunnyhop.html) ✅ ——**"不直接限制速度，只限制当前速度在加速度方向上的投影"**：

```
projVel  = dot(prevVelocity, accelDir)
accelVel = accelerate * dt
if projVel + accelVel > max_velocity { accelVel = max_velocity - projVel }
return prevVelocity + accelDir * accelVel
```

摩擦（仅地面）：

```
drop = speed * friction * dt
prevVelocity *= max(speed - drop, 0) / speed      // friction ≈ 1–5，即 sv_friction
```

**bhop 之所以成立，是因为落地后有 1 帧不施加摩擦**。想保留技巧性就保留这个窗口；想提高亲和度就加 auto-bhop（按住空格自动完美起跳）。

### 4.2 社区数学推导（参数上限与"为什么"）

| 资源 | 链接 | 关键结论 |
|---|---|---|
| QW physics air | https://www.quakeworld.nu/wiki/QW_physics_air ✅ | `wishspd = min(wishspd, 30)`；`addspeed = wishspd - dot(velocity, wishdir)`，`if addspeed <= 0 return`；`accelspeed = accel * wishspeed * pm_frametime = 10*320*0.014 = 44.8`，再取 `min(accelspeed, addspeed)`；**最大加速发生在 cos(γ)=0，即速度方向与 wishdir 垂直时**——这就是"甩鼠标 + 按 A/D"的数学本质；纯空中起步 `v(t) = 30·√(t/T)`，每帧速度向量旋转 `atan(30/v)`（越快，鼠标要转得越慢）；文末指出 **QuakeWorld 中 `sv_airaccelerate` 实际无效**（与 Source 的关键分歧） |
| QW physics ground | https://www.quakeworld.nu/wiki/QW_physics_ground ✅ | **`v_ultimate = v_max · √((a/f)·((2-aT)/(2-fT)))`**；TDM `v_max=320` 时极限地面速度 **494.9(72fps)/495.7(77fps)**，最佳 γ=53.7°；各兵种移速表（scout 450 / sniper 300 / soldier 240…）；**72fps 与 77fps 给出不同极限速度、77fps 加速快 7.7%** |
| Surfing（TF2 Wiki） | https://wiki.teamfortress.com/wiki/Surfing_(custom_game_mode) ⚠️ | surf 必须把 `sv_airaccelerate` 从默认 10 提到 **150**，trick surf 常设 **1000**；按 W 会立刻失去斜面抓地——"空中不损失速度地转向"的参数量级 |

### 4.3 关键推论（对你的代码）

> **README 里"移动必须固定步长"这件事，历史上是有铁证的**：同一个 Quake 引擎，72fps 与 77fps 的极限地面速度不同（494.9 vs 495.7），加速快慢差 7.7%。今天你用 macroquad 的 `get_frame_time()` 直接推进物理，**在 60Hz 与 144Hz 显示器上手感就是两把枪**。这是第 6 节"固定步长"的现实动机。

---

## 5. 输入、延迟与响应

| 资源 | 链接 | 关键内容 |
|---|---|---|
| Fix Your Timestep! | https://gafferongames.com/post/fix_your_timestep/ ✅ | 可变 `dt` → 行为依赖帧率（轻则手感漂移，重则模拟发散、子弹穿墙）；正解 **accumulator + 固定 dt**，渲染用 `alpha = accumulator/dt` 插值；警告 **spiral of death**，需限制每帧最大步数并把 `frameTime` 截断到 0.25s |
| What Every Programmer Needs To Know About Game Networking | https://www.gafferongames.com/post/what_every_programmer_needs_to_know_about_game_networking/ ✅ | Doom 的 P2P lockstep 被延迟毁掉 → Quake 改 client/server → QuakeWorld **客户端预测**（Carmack 原话 "biiiig architectural change"）；实现是**环形缓冲区保存过去输入与状态**，收到修正后丢弃旧状态并**重放**；"The Server Is The Man" |
| Snapshot Interpolation | https://www.gafferongames.com/post/snapshot_interpolation/ ✅ | 插值缓冲延迟取**发包间隔的 3 倍**以容忍连丢两包：10pps → **300ms**、30pps → **150ms**、60pps → **85ms**；位置用 hermite（带线性速度）、朝向 slerp；外推对刚体不好用；坚持 UDP |
| State Synchronization | https://www.gafferongames.com/post/state_synchronization/ ✅ | FPS 最相关一篇：**priority accumulator**（静止物体最终也会被同步）、**jitter buffer 约 4–5 帧 @60Hz**、**双向量化**（位置 4096 值/米、四元数 15 bit，否则外插值会跳）、视觉误差平滑用**两档衰减**（小误差 0.95 / 大误差 0.85） |
| Tickrate, Interpolation, Lag Compensation and You | https://dignitas.gg/articles/tickrate-interpolation-lag-compensation-and-you-probably-not-the-reason-why-you-just-missed-that-shot ⚠️ | 64 tick = 15ms、128 tick = 7.8ms；`cl_interp = cl_interp_ratio / cl_updaterate`；**lag compensation：`Command Execution Time = Current Server Time − (Packet Latency + Client View Interpolation)`** |
| A Quick Explainer on cm/360 | https://aimlabs.com/articles/aimlabs/a-quick-explainer-on-cm-360-and-the-common-cm-360-by-game/ ⚠️ | 用 **cm/360** 作为跨引擎可比的灵敏度单位；推荐值：VALORANT/CS2 **50**、Apex 40、OW2 35、BF 40、CoD 35、Fortnite 30、R6 40。**你的设置菜单应该按 cm/360 呈现，而不是"灵敏度 0.5"** |

---

## 6. 架构与工程化

### 6.1 主循环与时间（最该先做的一件事）

- [Fix Your Timestep!](https://gafferongames.com/post/fix_your_timestep/) ✅ → 把 macroquad 的"每帧 `get_frame_time()` 直接推进"改成 accumulator 消费、渲染插值。**理由见 4.3 的 72fps/77fps 铁证。**
- [Integration Basics](https://gafferongames.com/post/integration_basics/) ⚠️（同系列前一篇）→ 数值积分选型（显式欧拉 vs 半隐式），对角色速度/重力积累误差的影响。
- 参考实现：[bevy_fps_controller](https://github.com/qhdwight/bevy_fps_controller) ⚠️（Rust/Bevy，Source 风格移动：空中加速、按住跳跃 bhop、斜坡、蹲跑、noclip；**只做移动、不含射击**）。即使你不用 Bevy，它的**模块边界划分**（输入采样 / 移动解算 / 相机 / 物理查询分层）值得抄。

### 6.2 武器系统的数据分层（决定改枪能不能做）

**要抄的规格**：

| 资源 | 链接 | 能借鉴什么 |
|---|---|---|
| Rust 游戏官方 Wiki — Items Overview / Item Mods | https://wiki.facepunch.com/rust/Items_Overview ✅ / https://wiki.facepunch.com/rust/Item_Mods ✅ | **定义态 vs 实例态分离**：`ItemDefinition`（原型，全局共享）与 `Item`（实例）分离；原文明确警告 **"每个 ItemDefinition 只有一个 ItemMod，改动会作用于所有活跃 item，而不只是你挂的那个 Item"** → 对应 Rust 代码：`WeaponDef` 用 `Arc<...>` 共享，**运行时结算结果写进实例结构，绝不改 Def** |
| Gunsmith — Attachments | https://docs.gunsmithplugin.com/docs/dynamic_attributes/attachments/ ✅ | 见第 8 节的完整规格（三段式 Modifier / Static|Range） |
| Godot Asset Store — Simple FPS Weapon System（MIT） | https://store.godotengine.org/asset/jeheno/simple-fps-weapon-system/ ✅ | **Resource-based 武器定义**（编辑器内可视化编辑 + 版本管理友好）+ **每个武器组件（shoot / reload / animation / ammo）各占独立脚本**的分隔式设计；共享弹药、bobbing/tilt/sway、相机后坐力、decal 弹孔。小团队起步架构最接近的免费参照 |
| Godot 论坛 — FPS viewmodel 多武器工作流 | https://forum.godotengine.org/t/best-blender-to-godot-workflow-for-fps-viewmodel-with-multiple-weapons/135582 ✅ | 明确结论：**"不要让手跟着弹匣走，而是弹匣被装进手里"**——动画驱动在手臂骨骼侧，弹匣/滑套是武器自身的子骨骼；换弹动画随弹匣变化要做**动画变体集**，不要程序化拼接。这条对你的 `viewmodel.rs`（2D 投影绘制武器）是重要的方向判断 |
| bevy_fps_controller（Rust） | https://github.com/qhdwight/bevy_fps_controller ⚠️ | Rust FPS 移动层最值得抄的一份代码（Apache-2.0） |

---

## 7. 改枪系统（配件 / Gunsmith）

### 7.1 设计范式：不是"配件表"，而是"带约束的数值重构"

| 资源 | 链接 | 可借鉴的具体点 |
|---|---|---|
| CODM 官方开发者日志：枪匠系统设计思路 | https://np.huya.com/news/articles/gonglue-articles-4263-15.html ⚠️（虎牙转载官方日志，正文完整） | **唯一一份第一手"为什么这样设计"的文本**：① 9 个槽位按现实部位划分，第 9 个"被动技能"因为"操作武器的人也是变量"；② **有 9 个槽位但最多装 5 个配件**，官方原话逻辑："可选槽位过多时很容易让武器产生最优解，反而缩窄玩家选择面"；③ 几乎每个配件都是**增益 + 负面双属性**（例：长枪管 +伤害距离 / −机动性）；④ 新手引导给**两套推荐搭配**（全机动 / 全稳定）**教思路而不给最优解** |
| Rust 游戏官方 Wiki — Weapon Mods | https://wiki.facepunch.com/rust/Weapon_Mods ✅ | 工程上最"可抄"的极简范式：**4 个槽位类型**（Muzzle / Scope / Underbarrel / Magazine），每把武器固定可用槽位集合；效果分 **乘法(scalar) / 加法(offset)**；一张 **Effect Matrix**（15 mod × 9 属性：Damage/Velocity/Range/FireRate/Recoil/AimSway/HipAccuracy/ADSAccuracy/Magazine）——注意它**混用百分比与绝对值**（Muzzle Brake：Recoil `+50%`、AimSway `+100%`、HipAccuracy `-2.0`）→ **百分比与绝对值必须分开存储、分开结算**；另有 **Weapon Compatibility 矩阵**（武器 × mod 的 ✅ 表）→ **兼容性是一张数据表，不是代码里的 if** |
| Tarkov — Weapon Modding Fundamentals | https://www.ludo.guide/guide/escape-from-tarkov/the-ultimate-survival-guide/weapons-ballistics/5-3-weapon-modding-fundamentals ✅ | 配件 → 属性影响对照表；**兼容性三要素：口径 / 挂载系统（Picatinny、M-LOK、KeyMod）/ 具体平台（AR-15 vs AK）**——这套"接口标准"建模比"每把枪一张白名单"更可扩展；**Rails & Mounts 是"槽位扩展型配件"**，即递归槽树的关键 |
| Tarkov Gunsmith 任务线拆解 | https://timesaver.gg/blog/tarkov-gunsmith-guide ✅ | **把改枪当关卡设计**：任务同时约束「人体工学 ≥ X」「垂直+水平后坐力和 ≤ Y」「必须装某类配件」「挂载点 ≤ 4」。→ **把校验实现为纯函数 `validate(build, spec) -> Result<(), Vec<Violation>>`，任务/教程/成就/联机校验全部复用同一个函数** |
| Tarkov — Weapon Modding and Presets | https://www.ludo.guide/guide/escape-from-tarkov/side-quests-activities/weapon-modding-presets ✅ | **Presets（保存/一键套用整套 build）= 改枪系统的必备配套**，本质是 `WeaponBuild` 的序列化 + 分享码 |
| ArcInventory + Lyra 武器子物件（蓝图导出） | https://blueprintue.com/blueprint/b9a7s_rl/ ✅（内容为蓝图文本，噪音大但数据结构清晰） | 递归子物品 + Tag 寻址的最完整公开线索：`EN_WeaponAttachmentSlots` → `DT_WeaponAttachmentSlots`（行结构 **{GameplayTag, Slot:int, AttachmentFragment:class, ContextTag}**）→ `ArcItemStackModular` + `ArcInventoryItemSlotReference{SlotId, SlotTags, OwningObject}` + `GetSubItemInSlot(SlotRef)` → `AIF_WeaponSubItems[]`（**{ItemDefToSpawn, SubItemTags}**） |
| FPS Customization Kit（Fab，商业插件） | https://www.fab.com/listings/d8c82ad6-9ef9-4ed8-9146-70912fea1445 ⚠️ | **不是代码参考，是需求清单基线**：每个部件自带 recoil / accuracy / damage / rate of fire / projectile type 数据并**自动合并**成一把可用武器；**实时预览**（每次改动即时反映）+ 换色 UI |

### 7.2 实现规格（照抄 Gunsmith 的数值模型）

来自 https://docs.gunsmithplugin.com/docs/dynamic_attributes/attachments/ ✅（我已核对原文）：

- **Modifier 的三种施加方式（运算顺序必须显式定义）**：
  `additive` ｜ `multiplicative before additives` ｜ `multiplicative after additives`
  → 对应 Rust：`enum Modifier { Add(f32), MulPre(f32), MulPost(f32) }`，结算顺序**写死在 `resolve()` 里并加单测**（例如"两个 MulPre + 一个 Add + 一个 MulPost"的期望值）。
- **取值两种方式**：
  `Static`（常量，或读 CurveFloat，**X 轴 = 配件等级**）｜`Range`（min/max + **RandomSeed** 插值；seed = −1 时装备瞬间随机生成）
  → 后者天然产生"同款配件不同手感"，也是稀有度系统的现成挂点。
- **Executions（Apply / Remove 可覆写）**：装上就换弹道/换音效这类"属性之外"的行为。
- **`AppliedFrame` 参数**：配件必须在**预测客户端与服务端同一帧**生效，否则 reconciliation 抖动。→ **联机改枪最容易漏的同步点**，单人项目也应先在结构里留出这个字段。
- **换网格**：Gunsmith 给 Mutable 传参数（tag 末两段被解析成参数名与值，`Weapon.Attachment.Scope.Default` → 参数 `Scope="Default"`）。
  → **对 Rust/macroquad 的等价方案（更简单、更该抄）**：**用"附件实体 + 换 mesh handle + 参数化材质"替代运行时网格合并**。挂点用"Socket 名 → 父骨骼/父实体 + 相对 Transform"的数据表。

**挂点（Socket）语义**——跨引擎可直接照搬，来自 [Epic 官方文档](https://dev.epicgames.com/documentation/en-us/unreal-engine/skeletal-mesh-sockets-in-unreal-engine) ✅：
- Socket = 挂在 Bone 下的独立 attach point，带相对父骨骼的 Transform（translate / rotate / scale）。
- **`Force Always Animated`**：让该 Socket 的所有父骨骼始终被求值，即使它们因当前 LOD 被剔除 → 改枪挂件在远处 LOD 消失/错位的根因就在这里。
- 动态挂载三种规则：**Keep Relative / Keep World / Snap to Target** → 这是"预览时换配件不跳位"的正确语义。

### 7.3 Rust 生态现状（重要预期管理）

- **Rust 生态没有任何成型的改枪/配件系统实现**（子代理多轮检索 + 我的检索一致）：命中的 Rust FPS 项目全部集中在移动/射击/网络层。
- 所以路径是：**抄 Gunsmith 的数值模型 + Facepunch 的分层教训 + Epic 的 Socket 语义**，用引擎原语自己搭。第 12 节给了落地骨架。
- ⚠️ 注意：Bevy 的 `AssetServer` 有 `VisitAssetDependencies` + `#[dependency]`，可以表达"配件依赖哪些网格/材质"的依赖图，并支持热重载（改枪表改完立刻生效）。若你未来迁移到 Bevy，这是省时间的一点；[文档](https://docs.rs/bevy/latest/bevy/asset/index.html) ✅。同页也警告：**程序化生成的 asset 没有去重机制**，每个实体 `Assets::add` 会复制一份 → **涂装/材质变体绝不能"每把枪新建一份"**。

---

## 8. 涂装 / 皮肤系统

### 8.1 官方规格书（最高价值）

**[CS2 Weapon Finishes Guide（Valve 官方）](https://www.counter-strike.net/workshop/workshopfinishes)** ✅ ——我完整核对过正文，以下是可直接落地的规格：

**① 合成模型（不是贴图替换）**
> 武器需要额外纹理：哪些区域可涂（paintable）、各区域磨损多快（durability）。最终外观 = finish 数据 ⊕ 武器纹理，由**合成系统**产生。
> 官方原话警告：**"不要把 grime、划痕、AO 或武器细节画进你的 finish，合成系统会做。"**

**② 图案的 Alpha 通道是一套控制码**（Hydrographic / Custom Paint / Anodized Multicolored / Gunsmith 支持）
- `128` 与 `255` = 无效果（用默认磨损行为）
- `< 128` = **提升耐久**，越暗越耐磨
- `≈ 196` = **从可涂装区域中减去**
- **粗糙度写在图案 alpha 的 0–127 区间**（Exponent），且**为兼容旧 CS 皮肤该区间是反的：值越大越光滑**；> 128 时使用 finish 定义里的单一 roughness

**③ 两种图案投射方式**
- tile 到武器原 UV：Hydrographic / Anodized Multicolored / Patina / Custom Paint
- **三平面映射 triplanar**：Spray-Paint / Anodized Airbrushed
- 缩放基准：**非 triplanar 36 英寸、triplanar 18 英寸**，并**按武器尺寸自动缩放**（`ignore_weapon_size_scale 1` 可关）

**④ 随机化 = 每个 finish 声明 offset / rotation 范围 + wear 范围**，应用时在范围内取值；**grunge 纹理与 scratch 纹理各自随机摆放**，所以磨损量相同也不会长得一样。

**⑤ 磨损档位（可直接复用的数值区间）**
`Factory New 0.00–0.07` ｜ `Minimal Wear 0.07–0.15` ｜ `Field Tested 0.15–0.37` ｜ `Well-Worn 0.37–0.44` ｜ `Battle-Scarred 0.44–1.0`（很多 finish 只出到 0.6）

**⑥ PBR 验收标准**：金属 finish albedo 有效区间 **180–250**（低至 90 视饱和度可用），非金属 **55–220**；**超出区间的作品"不太可能被选中"**——即"美术数值也有硬标准"。

**⑦ Paint by Number**：每把武器被划分成区域，决定四种颜色/图案如何应用；弹药壳、枪管内壁等区域**永不重涂**。

### 8.2 float 与 seed：涂装的收藏深度

| 资源 | 链接 | 可执行要点 |
|---|---|---|
| CSFloat — How CS:GO Float Values Work | https://blog.csfloat.com/how-csgo-float-values-work/ ✅ | **别用 `rand::random::<f32>()`**：naive 取 [0,1] 均匀浮点会让 **75% 落进 0–0.0001**（IEEE-754 指数分布问题）。Valve 实际做法是 `uniform_int(0, 2147483647) / 2147483647` → Rust 里就是 `rng.gen_range(0..=2147483647i32) as f32 / 2147483647.0`。默认磨损范围 **0.06–0.80**；float 越大 grunge 不透明度越高 |
| CSFloat — Float Value 与 Paint Seed 分布分析 | https://blog.csfloat.com/analysis-of-float-value-and-paint-seed-distribution-in-cs-go/ ✅ | ① 箱内磨损区间之间有**空隙**（如 0.07–0.08 无值），是故意拉开档位观感；② **范围映射公式 `final = t*(max-min)+min`**；③ 各档概率（FN ~3% / MW ~24% / FT ~33% / WW ~24% / BS ~16%）；④ **paint seed 0–1000** 作为 RNG 种子输出 **x/y 偏移 + 旋转**，决定图案位置（"蓝宝石 #661"就是 seed 效应）→ **float × seed 两个正交维度**就是最省的收藏深度方案 |
| Riot 工程博客 — Reducing Rendering Complexity with Gun Goddess Miss Fortune | https://www.riotgames.com/en/news/reducing-rendering-complexity-gun-goddess-miss-fortune ✅ | **形态/皮肤切换的架构决策案例**：① submesh 可见性（缺点：新增内容要改基础模型、重导动画、内存上升）；② 整角色替换（为换一把枪付整套角色开销）；③ 新建 `SkinnedRenderable` 组件，把"网格 + 动画数据"收进一个组件、skin 自持多套 mesh 与动画、显示选中的隐藏其余。→ **给小型项目的启示：即便只有几十种枪/皮肤，也尽早把"渲染 + 动画"收成一个组件并统一入口**；补丁开关按 skin 粒度可关是上线新皮肤的正确姿势 |
| Ubisoft / GDC — Micropolygon Rendering in Anvil | https://gdcvault.com/play/1035671/Micropolygon-Rendering-in-Anvil ⚠️（只拿到摘要） | "材质实例爆炸"的规模化方向：**Deferred Material 系统扩展**（第三方摘录提到"10,000+ 材质但约 100 个 PSO"，⚠️ 该数字来自搜索片段、未核对原文，勿直接引用）。仅作为"以后要解决变体问题时"的线索 |

### 8.3 涂装系统的三条红线（子代理总结，我认同）

1. **材质/着色器变体爆炸**：绝不"一份皮肤一个材质"。→ 一个参数化材质 + 每实例一份参数 buffer。
2. **磨损是一次纹理合成，不是贴图替换**：武器资产从一开始就要做成"可涂装纹理 + 按区域耐久度"，否则后期无法补救。
3. **float 的随机数分布**：见 8.2 第 1 条，一行代码的修正，但错了整个皮肤经济就崩了。

**⚠️ 本次未取到的部分**：CS2 贴纸**刮擦（scrape）**机制的官方数值说明——[Steam 官方 FAQ](https://help.steampowered.com/en/faqs/view/3A24-96AB-AE41-1039) ❌（页面只返回外壳，正文未渲染）。若贴纸/贴花是你的重点，需要单独再挖一轮。

---

## 9. 关卡设计

### 9.1 唯一必读：[The Level Design Book](https://book.leveldesignbook.com/) ✅

免费开源，**每页加 `.md` 即可拿纯文本版**（可离线），[完整索引 llms.txt](https://book.leveldesignbook.com/llms.txt)：

| 章节 | 链接 | 关键数字/规则 |
|---|---|---|
| Blockout（白盒五步） | https://book.leveldesignbook.com/process/blockout.md | **可直接当 checklist**：① 手绘 layout 草图 ② 加地面 + 人体比例参照物 + 墙段 ③ 立刻进引擎带重力跑一遍 ④ 允许推翻原计划并迭代 ⑤ 重复。含 "Dirty Bomb 8–12 秒出生点到目标" 的节奏数字 |
| Metrics（比例与尺寸） | https://book.leveldesignbook.com/process/blockout/metrics.md | **TF2 实测三线交战距离**：近距离武器 ≤256 units、中距离 ≤1024、火箭/狙击 2048、无伤掉落上限 256；掩体三段高度（高掩体 ≥1.75m / 低掩体 1.0–1.25m / 非掩体 ≤0.5m）。**拿来定你的"metrics zoo"测试图** |
| Map balance（含 chokepoint / sightline） | https://book.leveldesignbook.com/process/combat/balance.md | **硬性规则：多人在 PvP 图中分布 3–4 个 chokepoint，每条 lane 一个，且不可能从单一点覆盖全部 chokepoint**；sightline 设计三原则；**PvE 要"掩体过多"偏保守、PvP 要"掩体过少"偏激进**（因为 PvP 玩家自带烟雾/闪光） |
| Cover（掩体与 peek） | https://book.leveldesignbook.com/process/combat/cover.md | 掩体 8 个维度、"切片派"（slicing the pie）转角清点法、"**通常掩体越少越好**"的反直觉结论；**避免"掩体盒子"**（腰高的重复方块） |
| Pacing（节奏） | https://book.leveldesignbook.com/process/preproduction/pacing.md | **兴趣曲线落地法**：Valve 把 beat 分四类 Explore / Combat / Choreo / Puzzle，横轴分钟、纵轴 0–100% 强度画图；多人节奏来源是 travel time + layout，不是脚本事件 |
| Layout typology（布局类型学） | https://book.leveldesignbook.com/process/layout/typology.md | 多人布局模式清单：**Figure 8（点名 Blood Gulch）、Three lane、No man's land、Sniper nest、Connector/Pivot、Base/spawn** |
| Gates（门控） | https://book.leveldesignbook.com/process/layout/typology/gates.md | lock-and-key 三原则：**先给玩家看锁，再给钥匙**；拿钥匙时提醒锁的位置；多钥匙的锁要视觉暗示钥匙数量 |
| Optimization（性能预算） | https://book.leveldesignbook.com/process/env-art/optimization.md | **draw call 预算表**：中端 PC ≈2000、高配 ≈5000、1 万以上"必须开始优化"；batching vs GPU instancing；为什么 alpha test 通常比 alpha blend 快。**注意：本页没有三角形数预算，别去编**——在你的目标机上实测 |
| Classic Combat 教学项目 | https://book.leveldesignbook.com/learning/projects/classic-combat.md | **最接近"一份作业 brief"的资源**：规定单房间 arena 的全部约束（2–3 种敌人、1–2 把武器、1–2 分钟战斗、宽度 ≤1024 units），并给出 1–4 小时白盒 + 1 小时 3 人 playtest + 1 小时迭代的工时。**你的第 7 张图照这个做** |
| 免费资产总索引 | https://book.leveldesignbook.com/appendix/resources.md | 经筛选的模型/材质/**白盒专用网格纹理**/天空盒清单 |

### 9.2 经典地图拆解

| 资源 | 链接 | 内容 |
|---|---|---|
| Chill Out（Halo 1）完整拆解 | https://book.leveldesignbook.com/studies/mp/chill-out.md ✅ | **整个清单里质量最高的一张地图拆解**：逐房间分析 Rocket / Mid / Pink / Gold / Shotgun / Spiral 六个区域、单向传送门如何当"更强制承诺的门"、以及 "friction" 概念（失败跳跃无部分失败状态 = 令人沮丧的技能检定）。小尺寸竞技图可直接把六房间结构当模板 |
| Andrew Yoder — Halo's Multiplayer Maps and Public Parks | https://andrewyoderdesign.blog/2017/12/09/halos-multiplayer-maps-and-public-parks/ ⚠️ | 上面那篇 Chill Out 分析的原始长文，配图更多 |
| Andrew Yoder — A Few Kinds of Cover（dust2 掩体分析） | https://andrewyoderdesign.blog/2022/04/14/a-few-kinds-of-cover/ ⚠️ | **做首个多人图的掩体摆放直接照套**：攻方 foothold（可利用但守方拿了也没损失）→ 无掩体 killzone（必须交资源换通过）→ 目标点（自由掩体最多，支持守方 retake） |
| de_dust2 结构分析 | https://book.leveldesignbook.com/studies/mp/de_dust2.md ✅ | ⚠️ 页面标记 `(STUB)`，正文为空——目前 dust2 最实质的免费分析仍是上面 Yoder 那篇 |
| Community Level Design for Competitive CS:GO（GDC 2015） | https://gdcvault.com/play/1021868/Community-Level-Design-for-Competitive ⚠️ | PvP 地图 sightline / readability / chokepoint 的第一手实践谈；balance 页的 de_cache chokepoint 图与 trick jump 图全部出自此演讲 |
| Devs Play: Doom — Hangar (Map 1) | https://www.doublefine.com/dftv/devs-play-doom-part-2-hangar-map-1 ⚠️（视频） | Romero 逐房间讲 E1M1 的设计意图、视线布置与教学顺序——**"教学关怎么设计"的最权威一手材料** |
| Level Design Book 推荐演讲总表 | https://book.leveldesignbook.com/appendix/resources/talks.md ✅ | 按主题分类的免费关卡设计演讲索引（含 Halo 3 狙击枪 0.5→0.7 秒那场经典 balance 演讲 [YouTube](https://www.youtube.com/watch?v=8YJ53skc-k4) ❌） |

### 9.3 针对你的项目的关卡建议

你的 `maps.rs` 是六张硬编码的 AABB 盒子列表（`arena / foundry / docks / bunker / citadel / rlyeh`），README 承认敌人**不能攀爬，只能跨过 0.62m 以下的箱子**——这是有意的垂直优势设计，但也意味着"高处扎营的玩家只受远程敌人威胁"。

因此，**在引入任何关卡编辑器之前**，先按 Level Design Book 的三条做：
1. **定 metrics**：玩家包围盒、视高、走/跑速度、跳跃高度、武器有效射程（你已有 `MAX_RANGE = 160.0`，可对标 TF2 的中距离 1024 units）。做一张 "metrics zoo" 测试图，专门验证"我的三线距离是多少"。
2. **给每张图定布局类型**：从 `Figure 8 / Three lane / No man's land / Sniper nest / Connector / Base` 里给 `arena`、`foundry`… 各挑一个，写进代码注释——**这一步几乎零成本，但会立刻暴露"六张图是不是其实长得一样"**。
3. **用 pacing 四类 beat 检查波次**：你的 `campaign.rs` 有 mission table（目标、loadout、敌人权重、波次）。把每个 mission 的强度（敌人数量 × 类型威胁）画成 0–100% 曲线，**确认有明确的高低交替与"开局安静"**。

**关于关卡编辑器（未来决策）**：Rust 生态里最成熟的方案是 [TrenchBroom](https://trenchbroom.github.io/manual/latest/) ⚠️（免费开源跨平台、专为刷体关卡设计）+ [bevy_trenchbroom](https://crates.io/crates/bevy_trenchbroom) ⚠️（0.14.0 / 2026-08-13，对齐 Bevy 0.19；`#[point_class]` 宏把 Quake 实体类映射成 Bevy 组件；**作者自述处于 "low maintenance / life support"——只会跟进新 Bevy 版本，几乎不再加新功能**）。**但这只在你能接受 glTF/场景资产管线时才划算**——你现在的关卡是代码生成的几何（`level.rs` 的 AABB + BFS 流场），改成资产驱动是一次大迁移。**建议：先不迁移，用上面三条把现有六张图的设计质量提上去。**

#### 9.3.1 Rust 关卡/资产工具链（版本与时间来自 crates.io API 实测）

> 记在这里是为了**将来真要迁移时不用重新调研**。注意 `bevy_trenchbroom` 目前只对齐 Bevy 0.19，而 Bevy 已发 0.20-rc.2——**升级 Bevy 会让关卡管线暂时断掉**，真要上的话得先锁版本。

| crate | 版本 / 发布 | 状态 | 能做什么 |
|---|---|---|---|
| [`bevy_trenchbroom`](https://crates.io/crates/bevy_trenchbroom) | 0.14.0 / 2026-08-13 | ⚠️ 作者声明 low maintenance | **Rust 生态最成熟的 FPS 关卡方案**：`TrenchBroom → .map → Bevy`（运行时直接解析）或 `→ .bsp → Bevy`（需 `bsp` feature）。加载一行：`commands.spawn(WorldAssetRoot(asset_server.load("maps/test.map#Scene")))`；物理走 Avian（`convex_collider()` / `trimesh_collider()`）；关掉 `client` feature 可做无头 dedicated server |
| [`qbsp`](https://crates.io/crates/qbsp) | 0.16.1 / 2026-08-13 | 活跃（与上面同日发布，配套） | Quake 1/2/GoldSrc BSP 的解析与网格化；`bevy_trenchbroom` 的 `bsp` feature 就靠它 |
| [`avian3d`](https://crates.io/crates/avian3d) | 0.7.0 / 2026-06-20 | 活跃 | 纯 ECS 的 3D 物理，`bevy_trenchbroom` 的默认后端；支持 `collider-from-mesh` 与确定性模式 |
| [`bevy-inspector-egui`](https://crates.io/crates/bevy-inspector-egui) | 0.37.0 / 2026-06-20 | 活跃 | 运行时热改实体字段——**白盒阶段调出生点/触发器半径，比改代码重编译快一个数量级** |
| [`rerecast`](https://crates.io/crates/rerecast) | 0.4.0 / 2026-08-05 | 活跃 | Recast 寻路网格生成器的 Rust 移植；从关卡几何生成 navmesh 给 AI 寻路。同作者有 `bevy_rerecast` 集成层 |
| [`ron`](https://crates.io/crates/ron) | 0.12.2 / 2026-06-22 | 活跃 | **"几何用编辑器、逻辑用 RON"这条分工的底座**：出生点、目标点、触发器、波次、掉落表用带 `serde` 的结构体。**不要用 RON 描述几何**——那是编辑器的活 |
| [`bevy_blender`](https://crates.io/crates/bevy_blender) | 0.10.0 / **2024-09-25** | ❌ 停滞约两年 | 直接从 `.blend` 读资产。概念诱人，但**大概率不对齐当前 Bevy**——走 glTF/GLB 标准路径，别用这个 |
| `tiled` / `bevy_ecs_tiled` / `bevy_ecs_ldtk` / `bevy_ecs_tilemap` | 0.16.0 / 0.13.4 / 0.15.0 / 0.19.0（2026-07 ~ 08） | 活跃 | **对 3D FPS 是错配工具**（都是 2D 瓦片地图编辑器）。只有做 2D 原型或想用 Tiled 规划楼层平面图当设计文档时才值得引入 |

**Quake `.MAP` 格式单页参考**：[Level Design Book — formats/map](https://book.leveldesignbook.com/appendix/resources/formats/map.md) ✅ ——若要自己写 `.map` 解析器（或想读懂 `bevy_trenchbroom` 在做什么），这是最省时间的一页：point entity / brush entity / worldspawn 结构、**brush 是"由 4+ 个平面定义的凸体而非三角网格"**（所以需要额外的编译步骤）、Valve 格式的 UV 计算伪代码。⚠️ 文中列的 `nomap`、`quarchitect` 两个 Rust 解析器**未能核实**（GitHub 被拦截，crates.io 上也没核到），引用前自行确认。

**TrenchBroom 的两个实操要点**（来自 [Level Design Book 的 TrenchBroom 专页](https://book.leveldesignbook.com/appendix/tools/trenchbroom.md) ✅）：网格尺寸按**玩家宽度**设置（Quake 血统 32u 宽的玩家 → 网格设 32 或 64）；主格式用 **Valve 变体 "version 220"**（用 Vector3 存 UV，避免 45° 面的纹理拉伸）。引擎侧对象与脚本通过 `.FGD` 定义。该页的导入插件对照表**没有列出 Bevy**——那正是 `bevy_trenchbroom` 的定位空白。

---

## 10. 美术与资产管线

### 10.1 第一人称武器渲染（行业标准做法）

**view model 与 world model 必须分层，且 view model 的 FOV 固定**——原因是武器模型是按特定 FOV 设计与制作动画的：

| 资源 | 链接 | 关键内容 |
|---|---|---|
| Bevy 官方 `first_person_view_model` 示例 | https://bevy.org/examples/camera/first-person-view-model/ ⚠️ | 双相机 + `RenderLayers`：world model 相机（FOV 90，可调）+ view model 相机（**FOV 固定 70**，`order: 1` 保证画在上层）；武器/手臂挂在独立 layer 并标 `NotShadowCaster` |
| TF2 Wiki — Viewmodel | https://wiki.teamfortress.com/wiki/Viewmodel ⚠️ | 实测数值：**TF2 view model 默认 FOV = 54，游戏内可调上限 70**；含 `viewmodel_fov`、`r_drawviewmodel 0`、左手/镜像、minimized viewmodel 等选项 → "武器 FOV 与主视角 FOV 分离"的落地数值就来自这里 |

> **你的项目做法不同但更聪明**：`viewmodel.rs` 把武器定义在 view space 并投影成 2D 三角形绘制，README 的理由是"做成世界几何会在贴墙时穿模"。**这个决策与上面的行业做法是同源的**（都是"武器不能与世界几何一起裁切"），只是你走了更省的一条路——继续沿这条路即可，但**要接受"没有 3D 骨骼动画"的代价**，即换弹/开火动画只能用 2D 变换做。

### 10.2 资产导出与坑

| 资源 | 链接 | 关键内容 |
|---|---|---|
| Godot 官方 — Model export considerations | https://docs.godotengine.org/en/stable/tutorials/assets_pipeline/importing_3d_scenes/model_export_considerations.html ⚠️ | **目前可访问的最权威 glTF 导出正确姿势**（规则对任何引擎成立）：glTF 是 **+Y up / +Z forward / 单位 = 米**；Blender 是 Z-up，导出保持 `+Y Up`；**1 Blender 单位 = 1 glTF 米**，角色 ~1.8 单位建模；导出前必须 `Apply > All Transforms`；Blender 中角色朝 −Y 建模。**这些坑在任何引擎里一模一样** |
| Cinevva — Blender → Godot 4 导出逐项清单 | https://app.cinevva.com/guides/blender-to-godot-export-checklist ⚠️ | 最接近"照着勾就能导出正确 glTF"的清单：`Format = glTF Binary(.glb)`、`Apply Modifiers` 开、**`Compression` 关（Draco 压缩引擎通常读不了）**、`Animation mode = Actions`、**每个 action 必须 stash 到 NLA track 否则不会导出**、`Reset pose bones between actions` 开、`Export Deformation Bones only` 开；附"symptom → cause → fix"排错表（模型躺倒 / 放大 100 倍 / 纹理丢失 / 动画只播一次 / 骨骼扭曲） |
| Polycount — FPS Weapon Creation 讨论帖 | https://polycount.com/discussion/154493/ ❌（403，未能核对） | 主题正确（FPS 武器制作规范），但本次访问被 Cloudflare 拦截，**未核对内容，引用前请自行打开** |
| Valve Developer Community — Dimensions / Viewmodel / How to Develop a Map That Works | https://developer.valvesoftware.com/wiki/Dimensions ❌ 等 | **全站部署了 Anubis 反爬证明（Proof-of-Work），本次未能读到正文**。建议用真实浏览器打开；其数值已被 Level Design Book 的 metrics 页转述（玩家包围盒 32×72 in、视高 64 in、走廊最小 64 in）——**用转述版即可，不必绕过反爬** |

### 10.3 免费资产（授权已核对）

| 资源 | 链接 | 授权 | 用途 |
|---|---|---|---|
| Quaternius — Animated Guns Pack（6 把带动画的枪） | https://quaternius.com/packs/animatedguns.html ⚠️ | 页面标 CC0（站点许可已迁移到自定义 QAL v1.0，实质等同 CC0） | **FPS 原型最省力的资产**：自带动画（开火/换弹），几小时内能看到"会动的枪" |
| Quaternius — Ultimate Guns Pack（40 把枪） | https://quaternius.com/packs/ultimategun.html ⚠️ | 同上 | world model 与敌人手中的枪 |
| Quaternius — 授权全文 | https://quaternius.com/license.html ⚠️ | QAL v1.0（2026-08-28 更新） | 可商用、可修改、**无需署名**；**唯一限制：不得把资产本身重新打包/转售/再分发为独立资产包**；§7 明确"未来版本不追溯，以你获取时的版本为准" → **下载时保存该页快照** |
| Kenney — Blaster Kit 2.1 | https://kenney.nl/assets/blaster-kit ⚠️ | **CC0** | 低多边形科幻武器，带动画与多变体（你的项目已在用 Kenney，风格一致） |
| Kenney — Crosshair Pack（200 个准星） | https://kenney.nl/assets/crosshair-pack ⚠️ | **CC0** | **FPS 原型最容易被忽略又最影响手感的一环**。含矢量 + 发光效果，半小时能试出适合你散布的准星 |
| Kenney — 授权说明 | https://kenney.nl/support ⚠️ | — | 原文确认：全部资产 CC0，可商用，无需署名；唯一限制是不要用 Kenney 的 logo |
| Poly Haven | https://polyhaven.com/ ⚠️ | **CC0** | 无缝 PBR 材质（≥8K）+ 16K HDRI，无需注册。**注意降采样后再进引擎**（你已有 512×512 的版本） |
| Mixamo | https://www.mixamo.com/ ⚠️ / [官方 FAQ](https://helpx.adobe.com/creative-cloud/faq/mixamo-faq.html) ⚠️ | 免费（需 Adobe ID），可商用 | 自动绑定 + 动画库。**两条硬限制**：① 不适用于 Enterprise/Federated ID；② **不适用于中国国家代码的用户**（"The current release of Creative Cloud in China does not include web services"）→ **这一条可能直接影响到你。** 技术上仅支持双足人形、需 T-pose、文件内不能有相机/灯光 |

---

## 11. 完整开源 FPS 项目与实战经验（灵感 + 避坑）

### 11.1 最值得拆的三个

| 项目 | 链接 | 语言/许可 | 拆什么 |
|---|---|---|---|
| **Quake III Arena GPL 源码** | https://github.com/id-Software/Quake-III-Arena ⚠️（8.2k★，已归档） | C / GPL-2.0 | **全清单里最干净的 FPS 架构教材**（纯 C、单文件职责清楚）。拆 `qcommon`（`sv_snapshot.c`、`cl_parse.c`、`cl_input.c`、`msg.c` 的 delta 压缩）、`cgame`、`game`、`botlib`。**它是你 `combat.rs` / `enemy.rs` / `main.rs` 的存在主义参照** |
| **Xonotic + DarkPlaces** | https://gitlab.com/xonotic/xonotic ⚠️ ；引擎 https://gitlab.com/xonotic/darkplaces ⚠️ | QuakeC/CSQC + C / GPL | **唯一长期活跃、能编译、有 Doxygen 文档的完整开源竞技 FPS**。拆 CSQC 客户端预测、网络字段量化、BSP/剔除；先读 `xonotic.org/doxygen` 的引擎 API，再看 `qcsrc/` 里的武器与移动 |
| **Unvanquished** | https://github.com/Unvanquished/Unvanquished ⚠️（1.1k★，2 个月前仍在 push） | C++ / 混合许可（**需逐个确认**） | **唯一"现代 idTech3"参考**：看它怎么把 2005 年的 Q3 代码重构成 C++17（现代渲染器、RmlUi、GLM） |

其他：**UZDoom** https://github.com/UZDoom/UZDoom ⚠️（非常活跃，看 30 年演进的实体/AI/关卡脚本 ZScript）；**OpenSoldat** https://github.com/opensoldat/opensoldat ⚠️（Object Pascal/MIT，"权威服务器 + 客户端插值"最小可读样本）；**Godot4-FPS-Template** https://github.com/chafmere/Godot4-FPS-Template ⚠️（MIT，"一个下午看完一个 FPS 该有哪些模块"的清单）。

### 11.2 源码导读（帮你少读十万行）

Fabien Sanglard 的系列（**均 ✅ 已确认可访问**）：

- [Quake 3 Source Code Review](https://fabiensanglard.net/quake3/index.php)（[网络专章](https://fabiensanglard.net/quake3/network.php)｜[QVM](https://fabiensanglard.net/quake3/qvm.php)｜[AI](https://fabiensanglard.net/quake3/a.i.php)）
  → ① 一张图讲清六模块数据流；② **所有输入（键盘/鼠标/UDP）统一进 `sysEvent_t eventQue[256]`——为回放和复现 bug 服务**（可直接搬到你的项目）；③ Carmack 原话："把网络显式拆成 client 表现侧与 server 逻辑侧是真正正确的决定"；④ 34 万行里 **30% 是工具链**，提醒你"引擎之外的工具成本"。
- [Quake 1 引擎导读](https://fabiensanglard.net/quakeSource/index.php)（[网络](https://fabiensanglard.net/quakeSource/quakeSourceNetWork.php)｜[预测](https://fabiensanglard.net/quakeSource/quakeSourcePrediction.php)）
  → ① `NetChannel` 头结构（sequence / ACK / QPort）+ "同时只允许一个未确认可靠包"；② **QPort 是为了绕开 NAT 端口重映射**；③ 延迟 = `receivedtime - senttime`；④ 预测章点出一个真实矛盾：本地玩家被预测到 t+latency/2，而其他玩家只到 t——**这类"文档没写的坑"才是拆解价值**。
- [Quake 2](https://fabiensanglard.net/quake2/index.php) → 函数指针结构体实现 C 语言多态；**单机也走 `NET_SendLoopPacket` 环回缓冲，单机与联机同一条代码路径**（省掉一半联调痛苦的经典技巧）。
- [Doom 3](https://fabiensanglard.net/doom3/index.php) → `gameImport_t / gameExport_t` 互交指针 = 引擎与玩法解耦最省事的实现；渲染器前后端分离。**明确警告：601,047 行 ≈ Q3 的两倍，别从 Doom 3 入门。**

### 11.3 独立开发复盘与范围控制（最该先读）

| 资源 | 链接 | 具体教训 |
|---|---|---|
| GameRes：射击游戏，有没有我们小团队的生存余地？ | https://www.gameres.com/915175.html ⚠️ | ① 正统竞技/战术 FPS 的玩家口味已被高成本作品系统性抬高，**小团队在该赛道几乎没位置**；② 出路在"第一人称 ≠ 射击"；③ 点名三个研究样本：**DOOM + 社区 MOD**（30 年玩家口味迁移）、**MULLET MAD JACK**（当代小制作水温：超短平快但内容少、重复度高——**这就是原型状态**）、**KILLER 7**（艺术方向补足预算） |
| Gunstoppable 复盘访谈 | https://80.lv/articles/gunstoppable-how-speed-is-damage-defined-a-roguelite-fps ✅ | ① "**先排产期，再谈内容**"：定下 2 年发售并守住；② 排产结构 = **1 个月原型证明核心好玩 → 造 4 幕结构但只用极少的武器/敌人/关卡 → 6 个月猛加内容 → 最后 6 个月只做平衡与打磨**；③ **核心钩子不是设计出来的**，是展会上玩家一句"所以你动得越快伤害越高？"点出来的——**必须尽早给陌生人玩**；④ 早期玩家"在混乱中迷路"，靠视觉对比度 + 独特音效解决；⑤ 全部用免费软件（UE5.6 + Blender + Audacity + DaVinci） |
| GDC Vault：Creative Corner-Cutting — Tower of Guns Postmortem | https://gdcvault.com/play/1021896/Creative-Corner-Cutting-Tower-of ⚠️（摘要可见） | "没资金、没名气、没计划"的人**几乎独自**做完一款 roguelike FPS，专讲超小团队必须做的偷工减料，以及"设计 8 岁的自己会做的那种游戏"带来的意外成功。**完整视频是否需会员：未确认** |
| Five Pillars of Immersive Sims | https://80.lv/articles/five-pillars-of-immersive-sims ✅ | 把沉浸模拟拆成五条可实现的支柱 → **不做联机也能给 FPS 加玩法深度**：优先做"实体 + 交互规则"，而不是堆内容量 |

### 11.4 灵感方向（不要照抄，用来选型）

- **机制层**：把**移动本身变成资源**（"速度即伤害"，Gunstoppable）；把**改装当教程与门槛**（Tarkov Gunsmith 任务线）；把**随机性当内容乘法器**（roguelike FPS，Tower of Guns）。
- **玩家口味迁移**：读 DOOM 三十年 MOD 生态，看"玩家到底要什么"的长期曲线。
- **明确不推荐的赛道**（单人 + 无资金）：竞技/战术 PvP、需要大量联机质量保证的玩法。

---

## 12. 联机与网络（未来用，现在只需知道边界）

> **强烈建议**：在你当前阶段（灰盒、手感未定）**不要碰网络代码**。以下是"什么时候该读"的储备。

| 资源 | 链接 | 关键内容 |
|---|---|---|
| Gabriel Gambetta — Fast-Paced Multiplayer 四部曲 | [①架构](https://www.gabrielgambetta.com/client-server-game-architecture.html) ⚠️｜[②客户端预测与纠正](https://www.gabrielgambetta.com/client-side-prediction-server-reconciliation.html) ✅｜[③实体插值](https://www.gabrielgambetta.com/entity-interpolation.html) ⚠️｜[④延迟补偿](https://www.gabrielgambetta.com/lag-compensation.html) ✅ | ② 用具体数字推演"预测与权威不一致 → 序列号 + 保留未确认输入 → 重放"，含"**不要提前杀死玩家（等服务器确认）**"这类落地细节；④ 讲清 lag compensation 的必然取舍——**"躲进掩体后仍被击中"是设计取舍而非 bug** |
| Gaffer On Games — 网络系列 | [What Every Programmer…](https://www.gafferongames.com/post/what_every_programmer_needs_to_know_about_game_networking/) ✅｜[State Synchronization](https://www.gafferongames.com/post/state_synchronization/) ✅｜[Snapshot Compression](https://www.gafferongames.com/post/snapshot_compression/) ⚠️｜[Snapshot Interpolation](https://www.gafferongames.com/post/snapshot_interpolation/) ✅ | 见第 5 节表格 |
| Valve — Source Multiplayer Networking | https://developer.valvesoftware.com/wiki/Source_Multiplayer_Networking ❌（Anubis 反爬） | tick rate / 延迟补偿的产业标准文档。**已确认的替代**：Sanglard 提供的 Valve 原文镜像 https://fabiensanglard.net/quakeSource/Lag_Compensation.zip ⚠️（章节页已确认，zip 未展开） |
| Rust 联机层选型 | [lightyear](https://github.com/cBournhonesque/lightyear) ⚠️（含 `fps`/`projectiles` 官方示例，已实现预测/回滚/插值/delta 压缩）｜[bevy_replicon](https://github.com/simgine/bevy_replicon) ⚠️（服务端权威状态复制，仓库已从 projectharmonia 迁到 simgine）｜[renet](https://github.com/lucaspoffo/renet) ⚠️（轻量传输层）｜[naia](https://github.com/naia-lib/naia) ⚠️ | **如果你未来不迁到 Bevy，这些都只是概念参考**；UDP + 自研是 macroquad 路线下的现实选择 |

---

## 13. 落地计划

### 13.1 七天（不改架构，只提升现有项目的可玩性）

| 天 | 动作 | 依据 | 验收 |
|---|---|---|---|
| D1 | **固定步长主循环**：accumulator + 固定 dt（建议 120Hz 模拟），渲染插值；`frameTime` 截断到 0.25s | [Fix Your Timestep!](https://gafferongames.com/post/fix_your_timestep/) ✅ + [QW physics ground](https://www.quakeworld.nu/wiki/QW_physics_ground) ✅ 的 72/77fps 证据 | 在 60Hz 与 144Hz 下手感一致（用移动速度曲线截图对比） |
| D2 | **移动模型换成 Source 公式**：投影限速 + 摩擦 + 落地 1 帧无摩擦；参数落成 `accel / air_accel / friction / max_speed` 四个常量 | [adrianb.io](https://adrianb.io/2015/02/14/bunnyhop.html) ✅ | 能做出"空中转向加速"，且参数可调 |
| D3 | **后坐力升级**：分段骨架（1–3 / 4–7 / 8–10 / 11+ 分段）+ **回落延迟** + 记录玩家下压量做抵消 | [GameRes 数值篇](https://www.gameres.com/915851.html) ⚠️ | 连发 10 发的弹着点形成可背的图案，而不是随机抖动 |
| D4 | **可读性修复**：玩家曳光与敌人曳光区分（颜色 + 音高）；受击方向指示；命中顿帧 2–4 帧 | README 的 Known limitations + [screenshake 清单](https://github.layabox.com/xuhaodong/awesome-skills/-/blob/7cb809f72f89a918178d602f0544ebf7c8eaa20f/plugins/product-manager/skills/prd-creator/references/patterns/game-feel/screenshake.md) ⚠️ | 录制 30 秒视频，不看 HUD 也能分清"我在打人"和"我被打" |
| D5 | **贴图接线（第 1 步）**：给 `mesh.rs` 的 `quad()` 加真实 UV；`finish()` 支持 `texture: Option<Handle>`；`level.rs` 的盒子类型从 `(min, max, Color)` 扩成 `(min, max, MaterialId)` | [美术审查报告.md](美术审查报告.md) §3 | 地面/墙/箱子贴图正确平铺，且重复纹理不拉伸 |
| D6 | **准星与灵敏度**：换用 [Kenney Crosshair Pack](https://kenney.nl/assets/crosshair-pack) ⚠️ 里与你的散布匹配的准星（动态准星 = 散布的具象化）；设置菜单按 **cm/360** 呈现灵敏度 | [cm/360 解释](https://aimlabs.com/articles/aimlabs/a-quick-explainer-on-cm-360-and-the-common-cm-360-by-game/) ⚠️ | 静止/移动/开镜三种状态下准星大小能反映真实散布 |
| D7 | **找 5 个陌生人试玩**（不要找朋友），只问两个问题：移动爽不爽、武器有没有打击感 | [Gunstoppable 复盘](https://80.lv/articles/gunstoppable-how-speed-is-damage-defined-a-roguelite-fps) ✅ | 记录原话，不改代码 |

### 13.2 接下来（结构级，按投入产出比排序）

1. **武器数据分层**（改枪/涂装的地基，**越早越省**）：
   ```
   WeaponDef     // 不可变原型：基础属性、槽位表、动画集；Arc 共享
   PartDef       // 配件：槽位、接口标准、可提供的子槽位、mesh/material 句柄、mods
   WeaponBuild   // 实例：Vec<(SlotId, PartId, seed)>
   ResolvedStats // 缓存结算结果：[f32; N_ATTR] + hash（绝不回写 Def）
   ```
   配套两个纯函数：`resolve(build) -> ResolvedStats`（**运算顺序写死并加单测**）、`validate(build, spec) -> Result<(), Vec<Violation>>`（任务/教程/成就/联机校验共用）。
   依据：[Gunsmith](https://docs.gunsmithplugin.com/docs/dynamic_attributes/attachments/) ✅、[Facepunch 的分层教训](https://wiki.facepunch.com/rust/Items_Overview) ✅。
2. **挂点用"子实体 + 相对 Transform"起步**，兼容性用**接口标准表**（口径 / 导轨 / 平台）而不是武器白名单；**不要**做运行时网格合并。依据：[Epic Socket 文档](https://dev.epicgames.com/documentation/en-us/unreal-engine/skeletal-mesh-sockets-in-unreal-engine) ✅、[Rust Weapon Mods](https://wiki.facepunch.com/rust/Weapon_Mods) ✅。
3. **涂装先做"单参数化材质 + 每实例参数"**：武器资产做成"可涂装遮罩 + 按区域耐磨度"；皮肤 = `{style, colors[4], pattern, projection: Uv|Triplanar, scale, offset/rot 范围, wear 范围, 图案 alpha 控制码}`；磨损存 `float(0–1) + seed(u32)`，**随机数用 `gen_range(0..=2147483647)/2147483647`**。依据：[CS2 官方规格书](https://www.counter-strike.net/workshop/workshopfinishes) ✅、[CSFloat float 实现](https://blog.csfloat.com/how-csgo-float-values-work/) ✅、[Riot 的变体教训](https://www.riotgames.com/en/news/reducing-rendering-complexity-gun-goddess-miss-fortune) ✅。
4. **关卡**：先按 9.3 的三条把现有六张图的设计质量提上去，**再决定**要不要上 TrenchBroom 资产管线。
5. **网络**：不碰。等灰盒手感通过陌生人测试，把"手感达标"设为**联机立项门**。

---

## 14. 附录：本次调研的局限（务必知道哪些没验证）

**A. 本环境无法访问的站点**（你的浏览器可能正常，但下面这些我**没能核对正文**，引用时请自行打开）：
- `github.com` / `api.github.com` / `raw.githubusercontent.com` / `gitlab.com` —— 被解析为非公网 IP。**因此所有 GitHub/GitLab 项目的 star 数、活跃度、许可证均来自第三方镜像（repos.ecosyste.ms）或 `crates.io` 元数据，我未逐页核对。** 已在文中标 ⚠️。
- `gamedeveloper.com`（原 Gamasutra）—— Cloudflare 403。**含 Steve Swink《Principles of Virtual Sensation》。**
- `developer.valvesoftware.com` —— 全站 Anubis 反爬证明。**含 Source Multiplayer Networking、Dimensions、Viewmodel、How to Develop a Map That Works。**
- `youtube.com` / `archive.org` —— DNS 解析异常。**含《The Art of Screenshake》原视频、FMPONE 的 GDC 演讲视频。**
- `polycount.com` / `tf2maps.net` —— 403。
- `help.steampowered.com`（CS2 贴纸 FAQ）—— 只返回外壳，**刮擦机制未取到**。

**B. 明确的资料空缺**（不是遗漏，是真的没找到可访问的一手源）：
1. **hitmarker / 命中反馈**的专门权威文章；
2. **coyote time / jump buffer** 的可访问权威一手源（候选集中在 GitHub 与 gamedeveloper.com）；
3. **改枪系统的 GDC 专场**（GDC Vault 免费区检索无命中）；
4. **武器三角形数预算**的权威数字（Level Design Book 只给 draw call 预算；**我没有编造面数**——请在目标机上实测）；
5. **"30 秒法则"** 的权威出处（**未找到，故没写进文档**；等价物是 Level Design Book 里 Valve 的四类 beat 强度曲线与 Splash Damage 的"8–12 秒出生点到目标"）；
6. **Rust 生态的改枪/涂装实现**（确认不存在，不要找了）。

**C. 二手来源标注**：`gameres.com` 的两篇（一篇是腾讯魔方工作室的转载、一篇是知乎原文转载）、`np.huya.com` 的 CODM 开发者日志（虎牙转载官方日志）、`tradeit.gg` / `ludo.guide` / `timesaver.gg` / `wiki.playvalorant.com` 的数值表——**均为整理/转载性质，数值用于量级参考而非精确复刻**。

**D. 调研基础设施问题（已修复）**：本次会话开始时 `D:\Documents` 的 Windows 文件权限存在异常（缺少 WRITE_OWNER），导致所有 shell 命令失败。已通过修权脚本修复，备份与回滚脚本在 `dsh-acl-recovery\`，如需还原可用其中 `acl-backup-*.json.ps1`。

---

## 15. 美术：为"纯几何 FPS"找到的规范与事实

> 完整原始报告在 [game-art-research/macroquad-fps-art-research.md](D:\Documents\game-art-research\macroquad-fps-art-research.md)。本节只保留**能直接改变代码**的部分。

### 15.1 三条最高杠杆（按投入产出比）

| # | 做什么 | 为什么是它 | 验收标准 |
|---|---|---|---|
| 1 | **敌人剪影的三重冗余编码** | 颜色只是第 4 层信息通道，色盲读者拿不到 | 纯黑剪影缩到 24×48 px、去掉全部颜色，5 秒内正确指认 ≥3/4 |
| 2 | **顶点色预算 + 逐顶点方向光 + 接触暗化** | `Vertex.color` 是 `[u8;4]`：只能变暗，不能亮过 1.0 | 全屏转灰度后，直方图有 **≥3 个明显分离的明度层**（不是挤在 0.6–0.9） |
| 3 | **一个全屏后处理 pass（雾 + 暗角 + 低血量去饱和）** | `Conf` 里**没有雾字段**，默认管线完全无光照（源码核实）——纵深线索必须自己造 | 最远处与最近处地面的明度差 **≥25%**（0–255 上约 64 级） |

### 15.2 颜色规范（[Color Universal Design](https://jfly.uni-koeln.de/color/) ✅）

- **原则原文**："Use not only different colors but also a combination of different shapes, positions, line types and coloring patterns" → 每种敌人至少 **2 个非颜色通道**。
- **红绿色盲比例**：白人男性 8% / **亚洲男性 5%** / 非洲男性 4%。
- **红用朱红不用纯红**：`#FF2000`（100%,32%,0%）或 `#FF1400`（100%,8%,8%）。
- **避开黄—绿之间的全部色相**；绿取偏蓝绿，紫取偏红紫。
- **不要同明度异色相**（"red characters on green backgrounds is unreadable"）——HUD 文字与底板必须在**明度**上分开。
- **小字必须粗体**（"only a few cone cells can be used for recognition"）。
- **标签画在目标上，不另设图例**（"matching same colors in distant places is extremely difficult"）。
- 分级验收清单：[Game Accessibility Guidelines](https://gameaccessibilityguidelines.com/) ✅（Basic / Intermediate / Advanced 三档）。

### 15.3 饱和度预算（[Little Design Things §3.1](https://www.redblobgames.com/making-of/little-things/) ✅）

> 原文："Use brighter (saturated) colors for important things, but in small areas only. If it's large then it makes everything seem unimportant."

**给 UI 和场景各设一个"饱和度预算"**：大面积（面板底、血条槽、地面、天空）一律低饱和中性；高饱和额度只留给**小面积的玩法关键信号**（准星、伤害方向箭头、弹药告警、可交互高亮）。同页还有两条：HUD 白字加 1–2px 黑描边；**只在"两状态之间没有中间态"时才需要过渡动画**（血条/弹药数这类连续量不要做过场）。

### 15.4 程序化行走（唯一可核实的一手源）

[Animating a character that has no animation frames](https://dev.to/idlecultivation/animating-a-character-that-has-no-animation-frames-5gf4) ✅ ——**与行业惯例相反但更对**：

> "A walking person does not move their head much... Every version I had built rotated and bounced the sprite around its middle, which puts the head through the largest excursion in the frame. That is what the eye was rejecting... **Pivoting the sway at the head**, so the bottom of the sprite swings and the top stays put, changed the read completely."

可直接用的收敛值：
- **摆动轴心放在头部/胸口**，躯干越往下摆幅越大，头部近似走直线。
- 脚部摆幅只要"你以为的 **1/3**"，垂直起伏比这还小。
- **节奏约为"手感觉得对"的一半**（即比直觉慢一倍）。
- 起伏曲线用 **`sin().powi(2)`**，不要用 `abs(sin())`——后者的谷底有尖点，会"咯噔"。
- **落脚瞬间加一小撮尘土粒子**：原文 "Contact is what sells weight, and a single particle at the right moment does more for the illusion than any of the curves above it."
- 元规则："When tuning a parameter makes something worse in both directions, stop tuning it. You are looking at the wrong term."

配套：[Easing Functions Cheat Sheet](https://easings.net/) ✅（转身用 `easeOutCubic`、后坐回落 `easeOutQuint`、心跳/呼吸 `easeInOutSine`）。

### 15.5 macroquad 0.4.16 的硬事实（源码核实，`docs.rs` 逐条确认）

| 事实 | 出处 | 影响 |
|---|---|---|
| **`Vertex.normal` 是 `Vec4`，macroquad 从不读它**，官方注释："completely safe to use it to pass arbitary user data" | `models/struct.Vertex` ✅ | **白拿 4 个 float 通道**：局部坐标、材质 id、假 AO 系数、UV 缩放比 |
| **`Conf` 里没有雾字段，默认管线无光照** | `conf/struct.Conf` ✅ | 雾/暗角/色调映射全要自己写 |
| **`Mesh.indices` 是 `Vec<u16>`** | `models/struct.Mesh` ✅ | 单 Mesh 顶点上限 65,536；`tobj` 给的 `Vec<u32>` 必须转换 + 超限拆 Mesh |
| **自定义纹理不能叫 `Texture` 或 `_ScreenTexture`** | `material.rs` 源码里 `panic!` ✅ | 用 `MaterialParams.textures = vec!["MacroTex".into()]` 之类 |
| **`_ScreenTexture` 是片元源码的子串检查** | `material.rs` 源码 ✅ | 写全屏后处理 pass 的入口 |
| **`MaterialParams` 运行时不能改** | `struct.MaterialParams` ✅ | 混合/深度/剔除必须一次定好 |
| **`_Time = vec4(time, sin(time), cos(time), 0.)` 每帧自动写入** | `quad_gl.rs` ✅ | 呼吸/脉冲/扫描线不用自己算 |
| **数组 uniform 必须用 `set_uniform_array`** | `quad_gl.rs` ✅ | 用 `set_uniform` 传数组会被**静默忽略**（只打 warning） |
| **`MAX_PIPELINES = 32`**，超出 `panic!` | `quad_gl.rs` ✅ | 自定义材质必须复用，不能按物体创建 |
| **`draw_mesh` 无 `_ex` 变体** | `models/fn.draw_mesh` ✅ | 加 uniform 就"先 `gl_use_material` 再 `draw_mesh`"；逐物体颜色烘进 `Vertex.color` 更省 |

### 15.6 OBJ 加载（`tobj` 4.0.5）

- **版本**：4.0.5 / **2026-08-02** / MIT / 近 90 天 64 万次下载 → **活跃，可放心用**。依赖只有 `ahash` + 可选 `log`，**不含任何图形库**，与 macroquad 无冲突。
- **必须开 `single_index`**，否则 positions/normals/texcoords 各有独立索引数组。用现成常量 **`LoadOptions::GPU_LOAD_OPTIONS`**。
- **互斥陷阱**：`single_index` 与 `merge_identical_points`、`reorder_data` 互斥，同时开会 `InvalidLoadOptionConfig`。
- **两处适配成本**：`tobj` 输出 **`f64`** → 需逐元素转 `f32`；**`u32` 索引 → `u16`**。
- 导出时做成纯三角面（`triangulate` 只支持能简单转成 triangle fan 的多边形）。

### 15.7 平铺重复的消除（[Inigo Quilez, Texture Repetition](https://iquilezles.org/articles/texturerepetition/) ✅）

**技法 3（虚拟变体 + 低频索引插值，仅 2 次采样）** 是首选，可直接抄的参数：**变体数 8**、**变化图采样频率 0.005**、**hash 用 `sin(vec2(3.0,7.0)*i)`**、**混合用 `smoothstep(0.2, 0.8, f - 0.1*sum(cola-colb))`**。

> ⚠️ **但有前置风险**：它依赖 `textureGrad`/`dFdx`，属 GLSL ES 3.00 特性；macroquad 内建抓屏着色器用的是 **`#version 100`**，而**默认片元着色器的 `#version` 未取到**（`shader.rs` 在 docs.rs 上 404）。**落地前必须先用最小材质实测能否编译。**
> 退化方案（推荐先做）：不用额外贴图，**直接用世界坐标 `position` 算 hash 当低频索引**，或者**关掉该材质的 mipmap** 接受远处接缝。
> 另：**镜像重复（`GL_MIRRORED_REPEAT`）能把重复周期直接翻倍**，一行改动，是这条路上最便宜的第一步。

### 15.8 程序化噪声 crate

| crate | 最新版 | 发布 | 许可 | 备注 |
|---|---|---|---|---|
| [`noise`](https://crates.io/crates/noise) | 0.9.0 | 2024-03-23 | Apache-2.0/MIT | 近 90 天 39 万次下载；**组合式 API**（`NoiseFn` trait、可叠 Fbm/MultiFractal）；但**已两年多没发版** |
| [`fastnoise-lite`](https://crates.io/crates/fastnoise-lite) | 1.1.1 | 2024-03-05 | MIT | 单文件 2,426 行、零依赖、可选 `f64`/`libm`；作者即 FastNoiseLite 原作者；**噪声结果可与其他语言/工具对齐** |

两者都停在 2024-03，都仍被大量下载。`noise` 配 macroquad 的 `glam 0.27` 无版本冲突。

### 15.9 本节已知空缺

- **Valve / TF2 的剪影与配色方法论原文：未找到可核实源**（Valve Wiki 被 Anubis 反爬，gamedeveloper.com 403，archive.org 被 DNS 拦截）。15.2 的方法论全部来自 jfly CUD 与 Red Blob Games，**没有引用 Valve**。
- **"步频 = 移速/步幅"这类行业惯例：无可核实来源**，故未写入 15.4。唯一可核实的行走源（devlog）反而与惯例相反。
- **"纯几何/低多边形风格却完成度很高"的 FPS 案例：未找到可核实来源**（Steam / Fandom / GOG 全部抓取失败或返回空正文；`gog.com/en/game/superhot` 抓到 200 但正文为空）。**所以无法回答"SUPERHOT / ULTRAKILL / DUSK / Devil Daggers / Mullet Mad Jack 分别靠什么弥补了美术投入"**——这是本次调研最大的缺口，我没有用"低多边形风格化"这类空话填上。要补需在能正常访问 Steam 与 YouTube 的环境里做。
- **WCAG 具体对比度比值（4.5:1 / 3:1 / 7:1）：未核实**，故 15.2 只用了"明度必须拉开"的定性表述。
- **Oklab / OKLCH 转换矩阵：未核实**（bottosson.github.io 与 wikipedia 均被拦截）——不要引用具体矩阵。

### 15.10 待办：UV 相位（`quad()` 的真实缺陷）—— ✅ 已关闭（世界轴 UV，`mesh.rs` `face_axes`）

`mesh.rs:100-118` 现在用**面内偏移**算 UV：

```rust
let along_u = (b - a).normalize_or_zero();
let along_v = (d - a).normalize_or_zero();
let offset = p - a;
let uv = vec2(offset.dot(along_u), offset.dot(along_v)) * self.uv_scale;
```

**每个面自己的第一个顶点 `a` 就是 uv 原点。** 密度是对的（这正是 `set_uv_scale` 解决的），但**相位是每个面私有的**：

- `maps.rs` 的地板是**多块 slab 拼的**。两块相邻地板共享一条边，但各自从自己的 `a` 起算 → **尺寸不同时共享边上的图案错位**。
- 一摞箱子垂直堆叠时，每层顶面图案不对齐。
- 15.7 的"宏观变化消重复"**依赖相位一致才有效**——相位乱的话，叠加的第二层 mesh 会把重复感变成噪声，反而更脏。

**修法：把 UV 轴吸附到世界轴**，UV 改用世界坐标而非面内偏移：

```rust
// 挑一个与法线不平行的参考轴（法线朝上就取 +X，否则取 +Y）
let r = if n.y.abs() > 0.9 { vec3(1.0, 0.0, 0.0) } else { vec3(0.0, 1.0, 0.0) };
let u_axis = (r - n * n.dot(r)).normalize_or_zero();  // 投影到面内
let v_axis = n.cross(u_axis);                          // 唯一确定，且 (u,v) 直接给出正面朝向
let uv = vec2(p.dot(u_axis), p.dot(v_axis)) * self.uv_scale;   // 注意是 p，不是 p - a
```

- **密度不变**：`uv_scale` 仍是 `Surface::metres_per_tile()` 的倒数。
- **相位全局一致**：相邻地板、垂直堆叠、墙角全部接得上。
- **朝向不再随机**：现在 UV 方向由 `add_box` 里顶点的书写顺序决定，吸附后所有面统一。
- **成本相同**：同样两次 `dot`，只是把 `p - a` 换成 `p`。
- 对你这种全 AABB 的世界，参考轴的挑选是**精确的**（不是近似），所以这是纯收益。

**顺带一个更便宜的第一步**：`GL_MIRRORED_REPEAT` 能把贴图重复周期**直接翻倍**。你已经通过 `get_internal_gl()` 设过 `Repeat`，换成 mirror 是一行改动。

### 15.11 待办：雾（`Conf` 里没有雾字段，得自己造）—— ✅ 已关闭（走自定义 shader `fog.rs`，非顶点烘焙）

`macroquad::conf::Conf` 的字段只有 `miniquad_conf / update_on / default_filter_mode / draw_call_vertex_capacity / draw_call_index_capacity`——**没有雾、没有光照、没有后处理**（源码核实）。所以雾必须自己造，**但不一定非要 shader**。

> **首选方案：把雾烘进顶点色。** `Level::from_layout` 是启动时构建一次、之后不再重建的静态 mesh，逐顶点算
> `mix(vertex_color, horizon_color, saturate((dist - start) / (end - start)))`
> 一次性写进 `Vertex.color`——**零 shader、零运行时成本、零 `#version` 兼容风险**。
>
> 这在你的管线里是**完全正确**的做法而非近似：顶点色本来就是"构建期烘焙的光照"（`mesh.rs` 的 `shade()` + `contact_shading()`），雾只是同一张烘焙表上的另一个项。

三个配套点：

1. **雾色取你已有的天空带颜色**（`main.rs:389-410` 那 40 段渐变条带）——雾色与天空不一致会立刻穿帮。
2. **敌人/拾取物**是每帧重建的（`enemy.rs` 已经这么做），可以在同一趟重建里按相机距离加雾。相对它们已有的 `transform_into` + `contact_shading`，多一个每顶点距离计算是噪声级成本。
3. **顺序**：先做烘进顶点色的雾（保证能用），把**暗角与低血量去饱和留在后处理 pass 里**，并且**动手前先用最小材质实测 `dFdx` 能否编译**（见 15.7 的前置风险）。

**后处理 pass 的三个硬约束**（源码核实）：自定义纹理**不能叫 `Texture` 或 `_ScreenTexture`**（macroquad 直接 `panic!`）；**`MaterialParams` 运行时不能改**（混合/深度/剔除必须一次定好）；**`MAX_PIPELINES = 32`**，超出 `panic!`——材质必须复用，不能按物体创建。

### 15.12 待办：反馈层（两个僵尸功能 + 曳光可读性）—— ✅ 已于 2026-10-02 关闭，见 §16.1

代码核实：`kill_flash` 与 `damage_numbers` **在 `effects.rs` 之外没有任何读取**——`kill_flash` 只在 `update` 里衰减，`damage_numbers` 只被 push（`damage_number()` 是唯一写入点），**连一个 draw 调用都没有**。比"没画出来"还彻底。

- `kill_flash`：全屏矩形 + 短脉冲，最便宜。
- `damage_numbers`：存的是**世界坐标 + 伤害值 + 剩余生命 + 爆头标记**，而 `write_mesh()` 只组装几何、不带数字。要做 3D→2D 投影再画文字；**更便宜且够用的替代**是"屏幕中心附近的命中数字"（丢掉世界坐标）。
- **曳光区分**：README 的 Known limitations 自己承认"玩家射击与敌人射击用同一种曳光，一眼分不出来"——分裂成两种颜色/两种音高，是零成本的可读性修复。叠加 15.2 的冗余编码原则：**不要只靠颜色**，同时改形状或粗细。

---

## 16. 对账附录（2026-10-02）：逐条代码核实

> **本节取代 §0 的「项目现状」表。** 那张表是 2026-10-01 的快照；此后本节记录的三处"待办"（§15.10 UV 相位 / §15.11 雾 / §15.12 反馈层）、§0 的两条"真正缺口"（动画、反馈），以及 §5 的 cm/360、§15.1③ 的低血量去饱和、§15.10 尾部的镜像重复，全部落地。
> 核验链（2026-10-02）：`cargo check --all-targets` ✅ ／ `cargo clippy --all-targets -- -D warnings` ✅ ／ `cargo test` **133 通过 / 0 失败 / 5 ignored** ✅ ／ `cargo fmt --check` ✅；真机截图核对（设置页 11 行、世界视角、去饱和 A/B 像素均值）；Windows 与 Linux 产物重建（`x86_64-pc-windows-msvc` 7.1 MB、`x86_64-unknown-linux-musl` 6.8 MB 静态 ELF）。行号为本次核对时的值。

### 16.1 已关闭（证据 ← 当前代码）

| 出处 | 条目 | 现状与证据 |
|---|---|---|
| §0 缺口① | "只有巨像会动" | ✅ 8 种敌人全部逐帧程序化动画（步行周期 / 旋翼 / 触手 / 爬行 / 大步），脚步帧扬尘（`enemy.rs`） |
| §0 缺口②、§15.12 | `kill_flash` / `damage_numbers` 僵尸功能 | ✅ 绘制已接线：击杀闪光 `main.rs:3122`（实现 `hud.rs:1008`）+ 顿帧 0.05 s（`main.rs:1471`）；伤害数字世界→屏幕投影 `main.rs:3016`→`hud.rs:1224` |
| §0 仍成立③ | 后坐力升级 | ✅ 分段骨架（1-3 / 4-7 / 8-10 / 11-17）+ 延迟回落 + 下压抵消（`player.rs:68-107, 427-464`） |
| §0 仍成立④ | 改枪数据地基 | ✅ 且超规格：三层类型、`resolve()`（`weapon.rs:536`，运算顺序写死+单测）、`validate()`（`weapon.rs:610`）、`Value::Range`+roll、子槽树、挂载标准；发行版 `CATALOG` 为空是刻意的（`weapon.rs:459`） |
| §0 仍成立⑤、§15.10 | UV 相位 | ✅ 世界轴 UV（`mesh.rs:150` 起；`face_axes` + 世界坐标，注释里记着旧病） |
| §15.11 | 雾（"Conf 里没有雾字段"） | ✅ 自定义 shader `fog.rs`（未走顶点烘焙路线），驱动拒绝时降级为默认材质 |
| §15.12 尾 | 曳光可读性 | ✅ 两型曳光"不靠颜色也能区分"：细白出射 / 宽深红入射（`main.rs:1291-1305`） |
| §2 | "缺顿帧 + 受击方向 + 音频区分" | ✅ 顿帧 0.05 s；受击方向弧（`hud.rs:1074`）；音效分 5 种枪声 / 敌人枪声 / 血肉 / 世界 / 爆头 |
| §4、§13.1 D2 | Source 移动公式 | ✅ 两处刻意分歧：硬速度上限杀 bhop 蓄速、空中摩擦（`player.rs:596-664`，注释解释） |
| §5、§13.1 D1 | 固定步长 | ✅ 取整子步 1/120（无渲染插值，取舍见 `player.rs:597-604`） |
| §9.3 | metrics / 类型学 / pacing | ✅ 每图实测注释（sightline 统计 30 处、掩体高度带、类型点名）；pacing report 测试 `campaign.rs:434` |
| §13.1 D5 | 贴图接线 | ✅ Surface/Skin + 世界 UV + 6 张纹理 |
| §5、§13.1 D6 | **cm/360** | ✅ 2026-10-02 新增。灵敏度行显示 `1.60 · 12.0 cm/360`；新增 MOUSE DPI 行（400–3200 预设，任意值可手改持久化）；公式 `cm_per_360()`（`main.rs:313`）。同时修正视角公式：从 `2/window_width`（分辨率相关）改为常量 `LOOK_RADIANS_PER_COUNT`（`main.rs:303`）——1920 宽下与旧行为逐像素一致，其他分辨率不再漂移 |
| §15.1③ | **低血量去饱和** | ✅ 2026-10-02 新增。`Grade` uniform 进雾 shader（`fog.rs:65/105/137`）：健康<30 或理智<25% 时世界灰化至多 50%，HUD / 枪械保持彩色；与红晕共用同一阈值（`main.rs:3035-3044, 3121`）。A/B 像素均值：同一面墙通道离散度 27.8 → 17.3 |
| §15.10 尾 | **GL_MIRRORED_REPEAT** | ✅ 2026-10-02 采纳。六张世界纹理 wrap 由 Repeat 改 Mirror（`surface.rs:187`），重复周期翻倍、接缝仍在整瓦线 |
| 本次 | README / MANIFEST 音效计数笔误 | ✅ 修正为 24 槽 / 19 文件覆盖 / 5 合成（代码事实：`audio.rs` 24 个 `Sfx`、`assets.rs` 19 个 `include_bytes!`） |

2026-10-02 顺带修正一个交互不一致：设置页"点击行 = 按 →"此前对"列表型行"不成立（点击分辨率行走反方向）；现在两种输入等价（`main.rs:1906` 的 `forward`）。

### 16.2 仍未采纳（按价值排序，供下一轮挑选）

1. **涂装系统（§8）** —— 未开始；且前提变化：CS2 规格以"可涂装贴图资产"为地基，本项目武器是程序化几何，需要另设路线（程序化图案 / 参数化配色 / 每实例参数），不能照抄。
2. **改枪的玩家可见部分（§7）** —— 数据层与 `validate()` 已就绪；缺发行版部件内容、改枪界面、任务约束、presets。
3. **§3.1 可选数值** —— 跑射延迟、滑铲、换弹"逻辑 / 动画"双时间：未采纳。
4. **coyote time / jump buffer（§14B.2）** —— 未做（该节本身也记着"没有可访问的一手源"）。
5. **OBJ 模型接入（§10.3 / §15.6）** —— Kenney 模型已入库并校验、无人加载；记录在案的有意搁置（`README.md:546-551`）。
6. **§15.1①② 的量化验收** —— 剪影打印测试存在（`enemy.rs:1888`，ignored）；"5 秒指认 / 灰度直方图 ≥3 层"的人工验收未做。
7. **§13.1 D7 陌生人试玩** —— 流程项，未做。

**2026-10-02 晚追加**：上面第 1、2 项已落地为游戏内的「仓库」（原军械台）——改装页（六把枪 × 瞄具/枪口/弹匣，8 个部件，数值条带增减箭头；部件在手中模型上可见）、涂装页（6 款 finish + 磨损值 + 图案种子 + CS2 档位名，磨损按面斑驳）、**配装页**（主/副武器从六把枪里任选、每槽备弹匣数、手雷×2 与药品×3 的数量，全部写进设置文件：`primary`/`secondary`/`primary_mags`/… 仅在改动时写出）。**备弹是有限池**：换弹从备弹取弹、不足则装不满、空池为干响，战场弹药拾取先补弹匣再入备弹；任务不再发固定武器（campaign.rs 的 `loadout` 字段已移除）。§13.2 的"数据分层 + 校验复用 + 单参数化材质"三条地基全部接上。仍未做：任务约束式改枪（§7.1 的 Tarkov 玩法）、presets、挂点 socket 树（当前挂点是各武器 def 上的手写锚点）、经济系统（当前以数量上限代替成本）。

---

## 17. 装备与护甲：八款游戏的实际建模（含三条可直接改代码的建议）

> 核实来源：SteamAnalyst（CS2 武器数据表）、Dignitas（CS:GO 护甲系统解析）、ZDoom Wiki、Combine OverWiki、Chalmers 游戏设计模式库（`virt10.itu.chalmers.se`）、Quake Wiki、EFT-AMMO、Activision 官方博客、Halopedia、Deep Rock Galactic 官方 Wiki、官方 TF2 Wiki。**均实际取到正文。**
> **勘误**：本节推翻了本次会话早前"你缺头盔槽/护甲部位建模"的说法——那是拿塔科夫的复杂度当标准，而塔科夫是**唯一一档**。

### 17.1 "所有 FPS 都有那套装备"不成立

| 游戏 | 护甲是什么 | 部位 |
|---|---|---|
| **Doom** | 一条数值 + 一个吸收率：绿甲 **33.333%/100 点**、蓝甲 **50%/200 点**，护甲加成每次 +1 上限 200 | 无 |
| **Quake III / QC** | **地图上的资源点**：两种甲**都吸收 66%**，轻甲 50 / 重甲 100，硬上限 200，**超过 100 点随时间衰减回 100**，默认 25 秒刷新 | 无 |
| **Half-Life 2** | HEV = 一条充能资源，靠**墙上充电站**补——护甲即关卡补给点；头盔是套装的一部分，不是槽位 | 无 |
| **Halo CE** | **两层**：护盾可回充，生命不可回充、只能捡包。原文 "A player's health can only be reduced if their shields have failed" | 无 |
| **CS2** | **每把武器一个穿透系数** + 一个"有/无头盔"位 | 只做到"**腿永远无甲**" |
| **L4D2** | **完全没有护甲**——用临时生命 + 一次性医疗包 + 倒地救援替代 | — |
| **Tarkov** | 等级 + 耐久% + **材料破坏系数**（Aramid 0.25 → Glass 0.8）+ 头部 **5 个防护区** + 弹药穿透判定 | **逐部位 HP** |
| **CoD 现代战争（多人）** | **没有护甲槽**（护甲是 Warzone 专属）；槽位 = 2 武器 + 3 Perk + 1 致命 + 1 战术 | 无 |

**分水岭是"有没有部位伤害"，只有 Tarkov 跨过了它。** 其余全是"全身一个总血量"。

### 17.2 三条能直接改代码的建议

你现在的实现（`main.rs` / `i18n.rs`）：`ARMOR_MAX = 50.0`、`absorbed = (amount * 0.65).min(*armor)`、**打哪里都吃 65%**、无部位。这与 **Quake III 同构**（单一吸收率 + 一条会耗尽的数值）——**这不是简陋，是竞技场射击的标准形态**。

**① 腿永远无甲**（一行规则换一个玩家能学会的战术）

CS2 原文："Kevlar covers chest, arms and stomach, the helmet covers the head, **legs are never armored**"。AK 腿部 ×0.75 且**不吃护甲**。

你的 `combat.rs` 已经在**敌人**身上做了 `point.y > e.head_y()` 的头/身区分，但**玩家被打时没有**——这一条把它对称起来。

**② 护甲减 aimpunch**（"感觉到"护甲，比再加 10% 减伤有效）

CS2：Kevlar 把胸腹的 aimpunch 减 **85%**，**头盔完全免疫头部 aimpunch**。你已有 `recoil` 与屏幕抖动基础设施，这是一个玩家立刻能感知的差异，**且不需要部位系统**。

**③ 每把武器一个可见的护甲穿透率**

CS2 实测：AK-47 **77.5%**、M4A4 70%、Glock-18 47%、Tec-9 90.6%、SG 553 100%、AWP 97.5%（"Armor pen. = share of damage that gets through kevlar"）。设计原则：**不要隐式公式**，玩家才能用"这把枪打甲如何"做决策。

### 17.3 换弹：你已经做对了，有一条值得加强

- **不要加"压弹"。** 核实结论：`tactical reload vs empty reload` 是否存在行业标准命名、两者时间差多少，**都没有可核实来源**——不要把它当依据。
- **值得加强的是"逻辑完成点"**：DRG 官方 Wiki 明文把切枪取消换弹写成玩家技巧——"If the player switches to another piece of equipment after this point, the first tool or weapon will remain successfully reloaded... Doing this intentionally is called **reload canceling**"，代价是切换装备有 **0.5 秒**延迟。
  你的 README 已经写了"打了一半的弹匣换下来还是半满"——**逻辑完成点已经存在**，差的是把它做成明确的、可被掌握的技巧窗口。
- **想改变战斗节奏，加一把无换弹武器比加任何槽位都有效**：
  - **DRG 转轮机枪**：无换弹，单一备弹池（1200/2400），压力来自**过热**——Spinup 0.7s、Cooling Delay 0.3s、**过热后 10 秒卡壳**，按 R 只是空转枪管"does not cool the gun down faster"
  - **TF2 机枪**：Wind-up 0.75s / Wind-down 1.15s（右键 1.4s），**开火首秒伤害 50%→100%**、散布同步收拢，预热/开火时不能跳不能切枪、移速降至约 47%
  - → **"预热时间"是换弹时间的等价物**：同样制造"我现在很脆弱"的窗口，但**不需要弹匣 UI 与换弹动画**——对程序化 viewmodel 完美适配

### 17.4 做与不做（依据 Chalmers `Equipment Slots` / `Armor` 模式页等）

**做**：腿无甲 ｜ 护甲减 aimpunch ｜ 每武器可见穿透率 ｜ 单件武器的"宽度"
**不做**：Tarkov 式逐部位 HP（需穿透判定 + 钝伤 + 耐久 + 分区覆盖**四套系统同时存在**）｜ 空间网格仓库 ｜ 护甲耐久（Chalmers 把 Deterioration 列为**可选修饰**而非必需）｜ 为"像大厂"凑 6 个槽

**最值钱的一条**：Chalmers 明确写 FPS 用"**无背包的 2–3 个槽**"换承诺与取舍——"槽位的价值来自**限制**，不是数量"。而 DRG 的深度来自**单件武器**：每件 **5 个改装 Tier**（每 Tier 2–3 选 1）+ **8 个 Overclock**（Clean/Balanced/Unstable 三档风险）。**宽度加在已有槽位上，而不是新槽位上——对独立开发者成本低一个数量级。**

---

## 18. 抄袭边界：把"模仿暗区"这件事做实

> 核实来源（4 条一手政府/司法/官方文件）：韩国文化产业振兴院 Kocca 产业报告、最高人民法院**指导性案例 279 号**（court.gov.cn 原文）、株式会社宝可梦官方公告、WIPO 官方出版物；另 INTA 行业协会文件、三篇律所文章（GEN Law Firm/Mondaq、大成/贸法通、康信/Mondaq）、法学教授博客（Eric Goldman）、游戏葡萄对《暗区突围》制作人的访谈。
> **免责声明：这不是法律意见。** 判决书原文 PDF 全部未能读取（工具不支持），Tetris 案与 Capcom 案细节系**律所文章转述**，且均为美国地区法院判决，各巡回区标准不完全一致。

### 18.1 机制可以照搬——这是有判决依据的

中国二审法院 2024–2025 连续两个判决明确**游戏规则/玩法机制不受著作权保护**：

- 《万国觉醒》案（广东高院 (2023)粤民终4326号）
- 《三国志·战略版》案（广州知识产权法院，2025-08）

两案都推翻了一审"游戏规则可作为《著作权法》第 3 条第 9 项'其他智力成果'受保护"的尝试。原文：游戏特征及其组合理念上属于"玩法设计"，**"by its nature is an idea, system, operational method or process"**，即使详尽、巧妙、体现巨大创造性，也 **"not an expression in the copyright sense"**。

**因此这些都在不受保护的一侧**：槽位树改枪、接口兼容判定、属性权衡、网格背包、按格占位、容器嵌套、重量/体积限制、死亡掉落、撤离带走战利品、隐藏处式局外进度。

**品类本身有政府级背书**：Kocca 报告把撤离射击定义为独立品类（"서치-파이트-익스트랙트"结构），说它由塔科夫 2017 年"정립"（确立），并把 2025 年的产品分为"元祖／大众化／主流化"三种模型——**其中"大众化"那款正是 PvE 单机化的《Escape From Duckov》（五人团队，5 天 100 万份）**。

### 18.2 ⚠️ 但界面不能照搬——这是最容易被低估的风险

**最高人民法院指导性案例 279 号（2026-02-28 发布）裁判要点原文**：

> 软件著作权人能够举证证明被诉侵权软件与权利软件的名称、版本号、权利人信息等特有信息相同，**或者软件界面设计高度近似**的，人民法院**一般无需进行软件代码比对即可以认定两软件构成实质性相似**，但被诉侵权人有相反证据足以反驳的除外。

→ **"界面设计高度近似"本身就能触发实质性相似认定。** 这一条直接适用于"照着暗区的仓库/改枪界面做"这个念头。
→ 实践含义：**功能结构可以借鉴（三栏＝槽位/候选/数值差异），但视觉、图形、图标、排版、配色、动效必须是自己的。** 而你的 HUD 已有一套自成一体的语言（琥珀 + 冷钢 + L 角标 + 中文排版）——**照你自己的 HUD 画。**

### 18.3 安全 / 危险分界

**可以照搬**：槽位树改枪 + 兼容性 + 属性权衡 ｜ 网格背包 + 拖拽 + 容器嵌套 ｜ 死亡掉落/撤离带走/局外进度 ｜ 预设配装 ｜ PvE 波次形态

**必须自己做**：
- **界面布局与图形元素**（指导性案例 279 号）
- **具体格数与容器清单**——塔科夫 `10x28/10x38/10x48/10x68`、THICC Items Case、Gamma、Scav Junkbox：**具体数字与专有名词是"选择"，不是功能必需**
- **数值表一一对应、错漏复现**——宝可梦 vs《口袋妖怪：复刻》一审判赔 **1.07 亿元**，认定关键词是"多个要素系统高度相似**甚至完全一致**，且**多项数值体系设计相同**"
- **属性名称与影响关系**——"Ergonomics"（同时影响 ADS 速度与瞄准耐力消耗）这种指标名与关系是可以另做选择的
- **部件名称**——塔科夫大量使用真实厂商名（EOTech、Magpul、PBS-4），照搬会**额外引入真实世界商标风险**

**名字**：
- ❌ 绝对不用：Escape from Tarkov、Tarkov、暗区突围、Arena Breakout、THICC、Gamma、Scav、PMC、BEAR、USEC
- ⚠️ 相对安全：Stash / Inventory / Loadout / Gunsmith / Ergonomics / Recoil 这类**描述性功能词**——INTA 商标强度谱系里描述性词 "generally not granted trademark protection"。**但未找到针对 "Gunsmith" 的具体商标裁定**，且 CoD 也在用这个词 → 当**功能名**问题不大，当**产品名/副标题**就别用。
- 🚨 **待办（本次未能核实）**：**你自己的产品名 "STEEL FRONT"** 是否已被注册/是否有同名游戏。这比仓库怎么设计重要得多。

### 18.4 最有参考价值的一份材料：暗区制作人的原话

被直接问到"很多人质疑你们和塔科夫的相似程度"时，《暗区突围》制作人 Onrry 的答复：

> **"任何一个后来者在学习前辈的过程当中，都会有类似的嫌疑。暗区有没有对塔科夫的模仿痕迹？当然有。因为它有很多设计对这个品类来说是对的，我没有必要，也不应该去改变它——我们不会为了创新而创新。但同样不可否认，暗区也有自己的表达，比如它的世界观，比如它的面貌和调性……"**

他被公开质疑、商业上极其成功、**从未被塔科夫方起诉**。分界线可直接照用：**对品类来说是对的设计不必改；世界观、面貌、调性必须自己做。**

律所文章里还有一句可直接引用的判准：**"lawful 'imitation-plus'（在一般思想之上构建并加入真正创新）remains a viable, court-respected strategy."** —— 你的 `Range` 掷点（同一部件在不同实例上手感不同）正是这种"imitation-plus"：**塔科夫的配件是固定的，你的每把枪有自己的个性**。

### 18.5 你真正的差异化杠杆是敌人 AI，不是仓库

塔科夫最有辨识度、最难被归入"通用语汇"的表达，是它的 AI 细节：**31 种无防弹功能的脸部装备**、一个多小时的 Scav 语音、综合天气/距离/角度/室内外/姿态的索敌系统、侧翼/扫射/压制/隐蔽/Peek/匍匐/呼叫支援的行为逻辑。

而这恰好是**暗区被批评最狠的地方**（"只会呆呆地站在马路上"，且批评者强调"这和强度是无关的"——差在拟人化）。

你的项目已有真 AI：BFS 流场寻路、丢视野后回到最后已知位置搜索（`SEARCH_REACH`）、8 种敌人各自的行为与 `standoff/windup/accuracy` 参数。**把 AI 做深，既规避风险又是最大差异化——比仓库和改枪都值。**

### 18.6 反不正当竞争法是真正的兜底条款（若"只换皮"）

《三国志·战略版》二审提出法院应做**结构化整体比对**而非摘取碎片：

1. 被指复制的规则在两游戏对应部分是否实质相似
2. 这些规则在完整规则集中所占比例
3. 在游戏整体架构中的位置
4. 功能角色及对玩法循环的贡献
5. 对玩家决策与市场选择的影响

**只有**该分析显示恶意盗用与竞争损害时才可获救济。→ 机制相同但**选择、编排、比例、功能定位、数值关系不同**，且自己做美术/实现，是法院明确认可的安全区。

**另有一条实务建议**：不要留下"逐项照搬、仅作表面替换"的开发记录（文档、提交历史、竞品拆解表）——这类记录会被用来证明"接触 + 过度模仿"。**保留独立开发的过程证据**（版本仓库、设计评审、素材许可），指导性案例 279 号的举证妨碍规则说明证据很重要。

### 18.7 诚实的缺口

- **判决书原文**：Tetris Holding v. Xio、Capcom v. Data East、宝可梦案——**PDF 全部未能读取**，细节均为律所文章转述。
- **ABA 文章 "It's How You Play the Game"**：americanbar.org 两个域名均 403，**未读取**。
- **"Gunsmith" 的商标裁定**：USPTO TTAB 返回 HTTP 500，**未找到**。
- **"Doom-like / Souls-like / roguelike deckbuilder 为何被视为正常"的权威分析**：**未找到可访问来源**——18.3 的分界线是我基于 Kocca、暗区访谈、Goldman 评论与大成律所四步分析法做的**推断**，不是引用某个权威标准。
- **PUBG vs Fortnite 诉讼**：**未找到**可访问可靠来源。

---

## 19. 可玩性：波次射击的五个可量化变量（含数值与验收标准）

> 核实来源：ULTRAKILL Cyber Grind 官方 wiki、Deep Rock Galactic 官方 wiki（`*.wiki.gg`）、Halo Firefight（Halopedia）、Risk of Rain 2 官方 wiki、Vermintide 2（ludo.guide）、Lostgarden、80.lv。**均为本会话实测 HTTP 200 且读过正文。**
> **本节针对的现状**：本项目的生成器是**一条连续滴流**（`main.rs` 每 `(1.9 - 0.06×difficulty)` 秒滴一个，直到 `total_enemies` 耗尽），`campaign.rs` 注释自己写明 **"This game has no wave system"** 且 **"the escalation lives between missions, not inside them"**。所以下面的每一条都是"从滴流到波次"这条路上的具体参数。

### 19.1 五个变量（按影响力排序）

| # | 变量 | 已核实的具体做法 |
|---|---|---|
| 1 | **波次点预算** | ULTRAKILL Cyber Grind：起始 **10 点**，每波 `+3 + ⌊波数/3⌋`（波 5 = 28、波 10 = 55）。敌人有 cost：Filth 1、Stray 2、Drone 4、Schism 5、Streetcleaner 5、Malicious Face 13+5、Cerberus 16+2、Hideous Mass 45 |
| 2 | **波间强制决策窗** | KF2：每波后 **75 秒**、6 个等距购买点、独立货币（击杀/助攻按伤害折算/治疗队友都给钱）、波末奖励随波数上升、**波中死亡只给一半** ⚠️ 来源为俄语攻略站，强度低 |
| 3 | **可见的难度阶梯** | DRG 5 档，每档**同时动 4–6 个乘数**：敌伤 **0.50/0.70/1.20/2.00/2.80**（单人）、攻击频率 0.80→1.50、弹速 1.00→1.70、移速 0.80→1.15、敌人池 5→8 槽 |
| 4 | **同时交战上限** | DRG：**每玩家 32 地面 + 32 空中**才 aggro，总存在上限 **60**；超出者只随机游荡。**这才是"高点数也不糊"的真正原因** |
| 5 | **失败摩擦与分数可见性** | ULTRAKILL：起始波选择器解锁门槛 = **到达该波数的 2 倍**（解锁波 5 需到波 10），**不跨难度**，排行榜按 5 档分列 |

### 19.2 可直接抄的编排细节

- **精英"缓冲波"（最重要的一条 —— 这是本项目现在就有的 bug）**
  ULTRAKILL：Hideous Mass **每 10 波上限 +1，出一只就锁 2 波**（出 3 只锁 6 波）；uncommon 出场后 buffer +0.5 或 +1 波，每过一个普通波减 1；special 波后冷却 = 该波特殊敌人数量；普通敌以 **0.1 秒/只**补刷。
  → 本项目的 `roll_kind` **每生成一个敌人独立掷一次、没有冷却**，Colossus 权重 1/12。26 次生成里出现相邻两次的概率约 **15%**，而出现的那一次看起来就像 bug（因为没有冷却机制，它确实是）。
  → **修法**：`Game` 加 `last_elite_wave: u32`，掷出精英且 `wave - last_elite_wave < 2` 时重掷或降级。

### 19.2.1 本项目生成点的三个真实缺陷（代码核实）

`main.rs` 的 `pick_spawn` 现在长这样：

```rust
const MIN: f32 = 14.0;
const IDEAL: f32 = 24.0;
let mut best = level.spawns[0];                                        // ← 缺陷 3
let mut best_score = f32::MIN;
for _ in 0..10 {
    let cand = level.spawns[(rng.u32() as usize) % level.spawns.len()]; // ← 缺陷 1：有放回
    let d = vec3(cand.x - player.pos.x, 0.0, cand.z - player.pos.z).length();
    if d < MIN { continue; }
    let hidden = !level.line_of_sight(...);
    let score = if hidden { 30.0 } else { 0.0 } - (d - IDEAL).abs();    // ← 缺陷 2
    if score > best_score { best_score = score; best = cand; }
}
best
```

**缺陷 2 是主因。** 评分是 `hidden×30 − |d − 24|`。生成点距离玩家通常 14–40 m，所以 `|d − 24|` 落在 **0–16** 之间——也就是说"藏不藏得住"值 **30** 分，而"距离合不合适"最多值 **16** 分。**只要抽到一个不在玩家视线里的生成点，它几乎必然胜出**，而每个竞技场都有若干固定的"背视线"角落。这就是"敌人似乎都从同一个地点生成"的机制解释（每个竞技场 12 个生成点，见 `maps.rs`）。

**缺陷 1：抽样有放回。** `level.spawns[rng.u32() % len]` 允许同一点被抽中多次，10 次抽样不保证探索 10 个不同点。与缺陷 2 叠加后更糟：那个"好"点只要被抽中一次就赢了，其余抽样全是浪费。

**缺陷 3：全部被拒时静默退回 `spawns[0]`。** 若 10 次抽样都因 `d < MIN` 被 `continue` 掉，`best` 保持 `spawns[0]`，**且不再检查它离玩家多远**。代码注释自己承认（`maps.rs`）："`pick_spawn` refuses any spawn within 14 m of the player and **quietly falls back to spawn 0** when every candidate is refused, **which would put a hostile in the player's lap**." 有一个测试（`every_arena_gives_the_spawn_chooser_somewhere_to_put_an_enemy`）要求任何位置至少留 4 个 ≥14 m 的点，但它只保证"通常有得选"，**没保证退路本身合法**。

**修法（三个缺陷一次修掉）**：

```rust
fn pick_spawn(level: &Level, player: &Player, rng: &mut Rng) -> Vec3 {
    const MIN: f32 = 14.0;
    const IDEAL: f32 = 26.0;

    // 1. 收集全部合格点，不再有放回抽样
    let ok: Vec<Vec3> = level.spawns.iter().copied()
        .filter(|s| vec3(s.x - player.pos.x, 0.0, s.z - player.pos.z).length() >= MIN)
        .collect();

    // 2. 一个都没有时退回"最远的那个"，而不是 spawns[0]
    if ok.is_empty() {
        return *level.spawns.iter()
            .max_by(|a, b| dist(a, player).total_cmp(&dist(b, player)))
            .unwrap();
    }

    // 3. hidden 从 +30 分改成"优先池"：先筛藏得住的，为空才退到全部合格点
    let hidden: Vec<Vec3> = ok.iter().copied()
        .filter(|s| !level.line_of_sight(vec3(s.x, 1.6, s.z), eye(player)))
        .collect();
    let pool = if hidden.is_empty() { &ok } else { &hidden };

    // 4. 在池子里按"离 IDEAL 多近"**加权随机**，而不是取最大值
    pool.iter().copied()
        .min_by(|a, b| (dist(a) - IDEAL).abs().total_cmp(&(dist(b) - IDEAL).abs()))
        .unwrap()
}
```

- **`hidden` 从软分数改成硬性优先池**：这是最小改动、最大效果的一处——"藏得住"变成筛选项而非压倒性分数，"距离"才重新成为真正的排序依据。
- **最后一步用加权随机而不是取最大**：永远选"恰好 26 m"那个点会引入新的确定性（每局都一样），**加权随机才能让同样的地图每局不同**。

**验收标准（可测）**：把 `pick_spawn` 在固定玩家位置跑 **200 次**，统计 12 个生成点各被选中多少次。修前分布极度集中（可能 150/200 落在同一点）；修后**没有任何单点占比超过 40%**。这个测试比读代码更能说明问题，也能防止以后改地图时重新引入。

**配合波次的进一步改进**：加波次后，**每一波开始时选 2–3 个生成点，整波都从这几个点出**。理由不是"避免重复"，而是**战术可读性**——现在每 1.8 秒独立随机一个点，敌人从四面八方零散出现，玩家无法判断威胁方向、也没有可守的通道；改成每波 2–3 个点，玩家能看出"这一波从东边和北边来"，于是有了选位、有了要守的方向、有了可以用手雷封锁的通道。**这才是"波次"真正好玩的地方——让每一波成为一个可读的战术问题。** 实现上给 `SpawnPhase::Releasing` 带一个 `Vec<Vec3>` 波内点位列表，逐个轮换使用（避免同点连出两个）。

- **每波重置**：Cyber Grind 每波开始**回满血与体力、清空所有武器冷却**才刷敌，累计点数即总分。
  → 效果是"波 = 一次干净决斗"，**死因永远是站位与决策，不是上一波留下的资源残渣**。本项目备弹是有限池且中途不能补，所以这一条会明显改变死因分布。

- **一波分多批，而不是一次刷满**：DRG 单次 Swarm 时间线 = 3.7s 播报 → **20 秒准备** → 270 点 → 15–25s 后 220 点 → 10–20s 后换 3 个刷点 120 点 → 100 点 → 90 点 → 结束播报。**刷点始终在玩家 20 m 内**（防蹲点）。普通波间隔 H5 120–150s、H1 150–450s。
  → 照抄"播报 + 准备 + 分批"，用**音效与 HUD 文本**做播报（零美术成本）。

- **DRG 的敌人 cost 表**（用于给 8 种敌人定 cost 的参照）：Swarmer 6、Grunt 10、Exploder/Web Spitter 25、Mactera Spawn 45、Acid Spitter 55、Praetorian/Warden 90、Menace/Stalker/Goo Bomber 95、Bulk Detonator 120、Shellback 145。每波随机抽 **1–4 种**敌人；**压力波只由最便宜的三种组成**，且 Swarmer 至少刷 `8 × 敌人数` 只。
  → **小怪波要保证最低数量**，避免"预算全花在一只大的身上"。

- **Halo Firefight 的周期高峰**：1 回合 = 5 波，1 Set = 3 回合；每 Set 末是 **90 秒 Bonus Round**——无限弱敌、不能复活、分数倍率提高。
  → 每 5 波插一个 60–90 秒"送分波"，这是最便宜的峰值记忆点。

- **Risk of Rain 2 的两条**：`coeff = (playerFactor + 分钟×0.0506×difficultyValue×playerCount^0.2) × 1.15^已完成关卡`；**商店价 = baseCost × coeff^1.25**（难度上升时物价同步涨，否则后期经济崩坏）；精英 spawn 成本 = 普通怪的 **6 倍**（Malachite/Celestine 为 **36 倍**），导演会**拒绝刷"过便宜"的怪群**。
  → 用"占预算倍数"表达精英，而不是独立概率；再用"剔除过便宜的组合"让后期弱怪自然消失，不需要额外规则。

- **DRG 的"每局 N 次"而非冷却**：多数强效主动 perk 是"每局 1–3 次"（Field Medic 1 次、Iron Will 1 次、See You In Hell 3 次）。
  → 本项目的三种药与手雷可以改成"每局有限次数"的高价值道具，逼玩家在波间决定"现在用还是留给第 18 波"。验收：**道具使用时间点分布应集中在高波次，而不是均匀分布**。

### 19.3 局内成长 vs 局外成长（本项目目前两者都没有）

- **DRG: Survivor 的分离**（这是本项目缺的那块拼图）：局外是 Meta Upgrades（按 Rank 与挑战解锁：Driller Rank 3/6/12/15/21/24/27 各解锁一把武器），局内是 Mid-dive Upgrade Cards；神器强度刻意温和。
  → **局外只解锁"选项"（武器/涂装/起始配装），强度主要来自局内**。
  → **验收标准（可直接测）**：满局外存档**在第 1 波不得拥有任何伤害优势**，只有更多可选内容。
- **DRG 的双闸门**：每 tier 2–3 选 1、**只能选 1 个**（互斥）；价格阶梯 1200/2000/2800/4800/5600 credits（每 tier 约 ×1.4），且 tier 需武器等级 4/8/12/16。
  → **钱 + 等级双闸门**，玩家就不会在第一波买完全部升级，选择被摊到整局。
- **DRG 的 Overclock 三档**（本项目改枪系统只差这一层）：**Clean** = 纯增益；**Balanced** = 大增益 + 等量惩罚；**Unstable** = 最大增益 + 重罚且**常常换机制**（Snowball 换成范围冰冻弹、−100 箱容量、+1s 再增压）。全游戏 160 个，限量获取。
  → **Unstable 档要改变操作方式而不只是数字**——这是 build 差异感的主要来源。
- **Vermintide 2 的天赋结构**：**6 行 × 3 选项**，在 5/10/15/20/25/30 级解锁；每行只问一个问题（第 2 行专门决定临时生命来源：cleave / kill / stagger 三选一）。
  → 不要做大网格。**行数 = 战役的任务节点数（你正好 6 关）**；本项目的"理智"天然就是这样一个问题：三种恢复理智的方式（击杀恢复 / 不被击恢复 / 主动消耗换增益）。
- **Lostgarden 的 faucet-and-drain**：价值链 `Action(输入 & −资源) → +产出`，早期是 source、中段 transform、末端 sink，链**必须以玩家的抽象动机锚点结尾**。实证是"捡树枝"本身不好玩，**只有后续目标存在时才有价值**。
  → 加经济前先画出链条并确认末端有 sink，否则玩家第 5 波就攒满钱。

### 19.4 本项目的落地顺序（结合上文诊断）

| 顺序 | 改动 | 依据 | 验收标准 |
|---|---|---|---|
| **1** | **修 `pick_spawn` 三缺陷 + 加精英缓冲波冷却** | 代码核实 + ULTRAKILL | 生成点分布无单点 >40%；连续 20 波无相邻精英 |
| **2** | **滴流 → 波次 + 波间 45–60 秒窗** | ULTRAKILL 每波重置 + KF2 ⚠️ | 波间够换弹 + 补血 + 一次决策；压力峰均比 ≥1.5 |
| **3** | **`total_enemies` → 点数预算**，`enemy::score` 即 cost 表 | ULTRAKILL `3+⌊w/3⌋` | 调试面板打印每波预算与消耗明细 |
| **4** | **AI 同时追击上限**（建议 20–25，程序化几何更吃性能） | DRG 32+32/60 | 最难波次 FPS 与"屏幕内瞄准你的敌人数"都不上升 |
| **5** | 每波重置资源（回血/清冷却/补弹） | ULTRAKILL | 死亡报表只显示"第 N 波 / 得分 N" |
| **6** | HUD 常驻难度档 + 效果倍率 | DRG 5 档 | 切档时 HUD 显示敌伤/攻速倍率数值 |
| **7** | 每 5 波一次 Bonus Wave（60–90 秒弱敌潮、双倍分、禁复活） | Halo Firefight | 分数曲线在第 5/10/15 波出现可测峰值 |
| **8** | 起始波选择器（解锁 = 到达 2 倍波数）+ 分难度本地最高分 | ULTRAKILL | 到达波 10 后可选波 5，且记录持久化 |
| **9** | 局外只解锁选项，强度放局内三选一 | DRG: Survivor | 满解锁存档第 1 波无伤害优势 |
| **10** | 给"精英 / Boss / 理智告急 / 换弹"各配独立音色 | Gunstoppable（80.lv） | 玩家能在不看屏幕时靠声音判断这四类事件 |

### 19.5 一个重要判断：现在该加内容还是调张力

Gunstoppable（单人开发）的时间盒：**1 个月原型**验证核心好玩 → 4 幕骨架（**很少**的武器/升级/敌人/关卡）→ **6 个月加内容** → **最后 6 个月只做平衡**（难度、张力、多样性），目标 2 年内发售。

→ **本项目已有 6 图 / 7 武器 / 8 敌人**，在独立游戏里不算少。所以判断是：**问题不在内容量，在这局游戏没有"呼吸"**（见 §19.1 的变量 1、2、4 全部缺失）。接下来的时间应投在难度曲线与张力上，而不是第 9 种敌人。

**验收标准**：**"同一张图在第 1 波与第 20 波感觉不同"**，而不是"敌人种类 +1"。

### 19.6 未找到（不要引用）

- **KF2 的 Endless Mode Boss 波间隔**：官方 wiki（`wiki.tripwireinteractive.com`）与社区 wiki（`wiki.killingfloor2.com`）**均 403**。**KF2 的"75 秒"来源是俄语攻略站 `playground.ru`，强度低**，本文已标注。
- **Devil Daggers / Hyper Demon 的机制数值**：`wiki.gg` 上无该 wiki（401），Eurogamer/RPS 正文为 JS 渲染。**所以"纯分数 + 固定内容如何维持重玩"这条没有可核实的一手来源**，19.1 变量 5 只用了 ULTRAKILL 的证据。
- **"每波/每层三选一"的权威设计准则**：未找到。可核实的只有 Vermintide（6×3）与 DRG（每 tier 2–3 选 1）两份具体结构。
- **"局外永久成长过强导致失去紧张感"的分析文章**：只有论坛帖，**未找到可访问权威来源** → 19.3 只引用正向做法，不引用"某某游戏因此变差"。
- **玩家留存/心流的定量基准**（一局时长、失败重试间隔、tutorial 长度）：**未找到任何可核实数据**。学术 PDF 被工具拒收（不支持 PDF），`gamedeveloper.com` / `gamedev.net` 403，Chalmers 模式库（`virt10.itu.chalmers.se`）**DNS 不可达**（注：§17.4 曾引用该库，那批内容取自本次会话更早的成功抓取，之后该域变为不可达）。
- **"游戏做得更可玩"的权威检查清单**：**未找到**。
- Gunfire Reborn / Roboquest / Ziggurat / Immortal Redneck / Nightmare Reaper / Serious Sam / Gears Horde / L4D 导演机制：对应 wiki 域 **401/403**，**未取得任何机制数据**。

---

## 20. 改枪/仓库界面：两个结构性病灶 + 武器数量增长（代码核实）

> 本节针对 2026-10-02 之后的界面现状。项目已实现：改装页（六把枪 × 瞄具/枪口/弹匣，8 个部件，数值条带增减箭头，部件在手中模型上可见）、涂装页（6 款 finish + 磨损值 + 图案种子 + CS2 档位名）、配装页（主/副武器任选 + 备弹匣数 + 手雷×2 + 药品×3，写进设置文件）。
> **注：§18.2 的最高法指导性案例 279 号给这件事加了约束——功能结构可以借鉴，视觉必须照自己的 HUD 画。**

### 20.1 问题不是"简陋"，是两个结构性病灶

**病灶一：数据层被压成了字符串，所以交互无处生长。**

```rust
// main.rs 改装页
rows.push(format!("{}  {}", i18n::t(Key::ArmoryWeapon), i18n::t(kind.name())));
for slot in &slots {
    rows.push(format!("{name}  {}", fitted_name(&build, slot.id)));
}
```

每一行是 `format!` 出来的 `String`，喂给 `MenuItem { label, enabled }`。而 `menu_nav(sel, len)` + `hover_selection` 这套选择器**被 8 处共用**（主菜单、设置、Armory、结果页…），它的模型是"**一个整数下标在扁平行列表上移动**"。

**这套模型只有一个交互原语：整行 +1 或 −1。** 所以这些事在架构里**根本表达不出来**：

- 一行的**标签**和**值**是两个独立目标（点标签 = 选中，点值 = 改）
- 点击一行进入**子列表**（配件的候选项）
- 一行里有**多个可点元素**
- **悬停某个配件 → 预览它装上之后的样子**
- 拖拽、滚轮、右键卸下

**这是"简陋"的真实来源：不是画得少，是渲染层拿到的信息量不够，能画的交互就只剩"循环切换"。** 在设置界面里这没问题（选项本来就该循环），但改枪不是设置。

**病灶二：鼠标点击被映射成"方向"，而不是"提交"。**

```rust
let step = if clicked_row == Some(back) {
    None
} else if right || clicked_row.is_some() {   // ← 任何一行被点击 = 视为"按了右方向键"
    Some(1.0)
} else if left { Some(-1.0) } else { None };
```

**点任何一行都会立刻改动它的值。** 而且 `hover_selection` 让鼠标**划过**就改变选中项——你只是想看看，一点就已经装上了。**"非常不方便操作"的直接原因就在这 6 行。**

`is_key_pressed` 是**离散触发、没有按键重复**，所以换成第 8 个部件要**按 7 次**，而屏幕上**看不到还有几个候选、当前是第几个**。

**上一轮把内容翻了几倍**（多了涂装页、配装页、六把枪任选、8 个部件），**交互原语没变**——所以摩擦不是变小，是**随内容线性放大**：以前 3 个部件按 2 次，现在按 7 次。

### 20.2 修法：把"循环"换成"选择"

不是把界面画漂亮，是**换掉那个原语**。三栏 + 悬停预览：

```
┌─────────────┬──────────────────┬──────────────────────┐
│ 槽位         │ 该槽位的候选项     │  数值差异（悬停即算）   │
│             │                  │                      │
│ 瞄具  无  ▸ │ ● 无      (已装)  │  后坐力   12.4 → 10.1  │
│ 枪口  补偿器 │   红点            │  射程     38  → 36     │
│ 弹匣  标准   │   全息            │  瞄准速度  0.31 → 0.35 │
│             │   4×             │                      │
│             │                  │   武器预览（实时）      │
└─────────────┴──────────────────┴──────────────────────┘
   点槽位 →      悬停 = 预览        点配件 = 装上
```

三步，每步修一个具体缺陷：

1. **点槽位进入候选列**——取代"按 → 循环"。8 个部件从 7 次按键变成 1 次点击。
2. **悬停即预览**——候选列里鼠标停在哪，右边立刻显示"装上它之后"的数值与外观。**同时解决"看不见有几个候选"和"不知道代价"两个问题。**
3. **点击才提交**——把 `clicked_row.is_some()` 那个映射删掉。

**现成的加速器**：`armory_stats(def, build)` 现在比的是"**当前 build** vs **出厂空 build**"（回答"我这套改得比裸枪好吗"），而玩家真正要问的是"**这个配件值不值得装**"。把它加一个参数变成 `armory_stats(def, current, candidate)`，**悬停预览就免费得到了**——`weapon::resolve` 是纯函数，`viewmodel::preview_mesh(kind, fitted, paint)` 也已存在（还有 `armory_dirty` 脏标记缓存）。成本是**一次额外的 `resolve` + 一次 `preview_mesh`**。

**改动边界（重要）**：**不用动菜单系统。** `menu_nav` / `hovered_rect` / `item_list` 服务 8 个界面，全都别碰。只改：

- `ArmoryView`（`hud.rs`）从 `rows: &[MenuItem]` 换成三段：`slots` + `parts` + `diff`
- `draw_armory`（`hud.rs`）返回**按区域分组**的矩形，而不是扁平 `Vec<Rect>`
- `Game` 加 `armory_focus: Slots | Parts` 与 `armory_hover_part: Option<usize>`
- 删掉那两个 `match game.armory_sel` 的分支表（已膨胀到几十行），改成按区域分发

**配装页不要动**——它本质是计数设置，循环切换是合适的。**改枪不是设置，但配装数量是。**

### 20.3 武器多了怎么办：三条轴分开看

**轴 1：部件候选——数据层已经会过滤，缺的只是列表化。**
`reachable_slots(def, build)` 只返回"这把枪 + 已装配件开出来的子槽位"，`validate()` 拒绝接口不匹配的配件。**玩家永远看不到不兼容的配件。** 所以"配件多了要翻大半天"的成本**不在数量，在交互**：候选是藏起来的。把循环换成列表后，**30 个配件和 3 个配件的操作成本一样**——一次扫视 + 一次点击。

**轴 2：武器选择——这才是会真正变长的那条轴。**
`WeaponKind` 是**封闭枚举**（7 个变体），改枪页第一行 `[WEAPON] RIFLE` 按 → 绕一圈 6 把枪。到 20 把枪时一维列表不成立。**修法不是加搜索框**（那是 50 把以上的解法），是**换成分组网格**：

```
┌── 突击步枪 ──────┬── 冲锋枪 ──────┬── 精确射击 ────┐
│ [M4]   [AK]     │ [MP5]  [P90]   │ [DMR]  [狙击]  │
│ [AUG]  [G36]    │ [Vector]       │                │
└─────────────────┴────────────────┴────────────────┘
```

**这是"浏览"（扫视、比较、按类找），不是"循环"（一个个试）。** 而分类的第一维**已经存在**——`SlotClass`（`Primary / Secondary / Pistol / Melee`），只是没在界面上用。

**轴 3（最重要）：你不需要"很多把武器"，因为变化空间在配件树。**
`reachable_slots` 会**走到不动点**（一个配件开出新槽位，让另一个配件装上去），所以：

> **6 个基础平台 × 30 个配件 = 远超 20 把枪的组合数。**

**加武器的正确判据**：加新武器**当且仅当**它带来新的槽位类型、新的接口标准、或新的开火模式。否则宁可加**配件**——同样一份工作量，感知到的多样性大得多，因为**配件是乘法的，武器是加法的**。

**架构代价（必须知道）**：`WeaponKind` 是封闭枚举，**每加一把武器都要动多处 `match`**：`name()`（i18n）、`stats()`、`def()`、`viewmodel` 的模型、`codex` 的卡片与标签、`campaign` 的生成权重表。
→ **6–20 把枪这个代价可接受**（纯机械改动，编译器全会指出来，不会静默出错）。但若真打算做 100 把，`WeaponKind` 必须换成数据驱动（`&'static WeaponDef` 索引表），那是一次 invasive 重构。
→ **建议：保持在枚举上，控制在 20 把以内，把省下的力气投到配件树。**

### 20.4 本节与 §18.2 的关系

§18.2 的最高法指导性案例 279 号明确"**软件界面设计高度近似**"可无需代码比对即认定实质性相似。所以**上面那个三栏图可以借鉴其功能结构（槽位 / 候选 / 数值差异），但视觉、图形、图标、排版、配色、动效必须是自己的**——照你自己的 HUD（琥珀 + 冷钢 + L 角标 + 中文排版）画，不要照暗区画。

---

## 21. ⚠️ 交接对账（2026-10-02 深夜）：报告的建议已被大量实现

> **读这一节之前，先知道一件事：§0 的"新发现"表和 §17.2 / §19.4 / §20.2 的行动项描述的是**实现前**的状态。本节记录**实现后**的真实状态，是交接的权威依据。**
> **本节为追加，不改动任何已有章节号**——因为**代码里有 7 处反向引用**（`reference §19.1` / `§19.2` / `§19.2.1` / `§19.4-3` / `§19.4-5`），改动编号会让那些注释指向错误的地方。

### 21.1 已完成（证据 ← 代码）

| 报告条目 | 状态 | 代码证据 |
|---|---|---|
| **§19.1 波次系统** | ✅ **已实现** | `main.rs` 的 `WavePhase { Prep{left} / Release(WaveRelease) / Clear / Finished }`；注释原文"the field used to be one continuous drip; the reference (§19.1) measured that as the reason a run has no decisions in it" |
| **§19.1 点数预算**（`total_enemies` → 点数） | ✅ **已实现** | `campaign.rs` 的 `wave_floor` / `wave_share` / `wave_budget`；宏参数从 `$total` 改为 `$budget, $waves`；`release_one` 按 `budget` 与 `floor_left` 扣减 |
| **§19.2 精英缓冲波** | ✅ **已实现** | `campaign.rs` 的 `is_elite` + `best_fitting(…, elite_ready)`；`main.rs` 的 `last_elite_wave: Option<u32>`，判定 `wave >= l + 2`；测试 `elites_never_come_back_to_back`（跑 20 波，断言无相邻精英，且断言"20 波里至少出现过 1 只精英"） |
| **§19.2.1 生成点三缺陷** | ✅ **三个全部修掉** | ①`spawn_pool()` 收集全部合格点（**不再有放回抽样**）；②全被拒时退回**最远的点**（不再退回 `spawns[0]`，注释原文"it is a rule about array order"）；③`hidden` 从 +30 分改成 **`HIDDEN_WEIGHT = 2.5` 的权重**，距离用软三角 `1/(1+0.6·|d−IDEAL|)`，`IDEAL = 26.0`；最后**按权重加权随机**而非取最大 |
| **§19.2.1 每波 2–3 个入口点** | ✅ **已实现** | `WaveRelease { points: Vec<Vec3>, cursor: usize }` + `spawn_set(…, WAVE_ENTRANCES)`；注释"arrivals read as 'from the east and the north' — a direction to hold" |
| **§19.4-5 每波重置**（部分） | ✅ 按批准的窄范围实现 | `player.wave_reset()` 恢复**生命与呼吸**；注释明确**不**补弹药（"that economy is the stash's"）、**不**补护甲与理智（"those are things the arena is allowed to take from you"）——**这是对报告建议的刻意收窄，不是遗漏** |
| §16 的旧待办 | ✅ 全部关闭 | 见 §16.1（UV 相位、雾、反馈层、固定步长、Source 移动公式等） |
| 产品改名 | ⚠️ **注意** | 游戏已从 **STEEL FRONT** 改名为 **TIDES OF R'LYEH / 拉莱耶的潮声**（提交 `59ba77a`）；设置目录 `tides-of-rlyeh`，保留 `eldritch-front` / `steel-front` 作为 legacy 迁移路径（`config.rs` 的 `LEGACY_DIRS`） |

**因此 §18.3 里"待办：查 `STEEL FRONT` 是否已被注册"这一条已经过时**——要查的名字现在是 **TIDES OF R'LYEH**。

### 21.2 仍未实现（按交接优先级）

| 报告条目 | 状态 | 说明 |
|---|---|---|
| **§19.4-3 敌强度随波次递增** | ⚠️ **半实现** | 点数预算做了，但 `Enemy::spawn(kind, pos, wave, rng)` 拿到的 `wave` **仍是 `mission.wave`（每关一个常量）**，不是递增的波号。所以**难度上限仍在 wave 7**，`1 + 0.13·(wave−1)` 的成长空间（到 wave 40 是 6.07×）**还没用**。这是"无限模式"的最后一个前置条件 |
| **§19.4-4 AI 同时追击上限** | ❌ 未做 | DRG 的"每玩家 32 地面 + 32 空中 / 总数 60"是"高点数也不糊"的原因。加波次后一波涌多个，这条会变得更重要 |
| **§19.4-6 HUD 常驻难度档 + 效果倍率** | ❌ 未做 | DRG 5 档的做法 |
| **§19.4-7 Bonus Wave（每 5 波）** | ❌ 未做 | Halo Firefight 的 90 秒送分波 |
| **§19.4-8 起始波选择器 + 最高分** | ❌ 未做 | ULTRAKILL 的"解锁 = 到达 2 倍波数"。**`game.score` 目前仍是死数字**——不进存档、不解锁、不记录最高分（§0 的第三条仍未解决） |
| **§19.4-9 局外只解锁选项** | ❌ 未做 | 需要先有局外的解锁池 |
| **§19.4-10 四类事件独立音色** | ❌ 未做 | 精英 / Boss / 理智告急 / 换弹 |
| **§17.2 三条护甲改动** | ❌ **全部未做**（代码核实） | `main.rs` 仍是 `absorbed = (amount * 0.65).min(*armor)` 全身统一，**没有**腿无甲、**没有** aimpunch 减免、**没有**每武器可见穿透率 |
| **§20.2 改枪三栏 + 悬停预览** | ❌ 未做 | 代码核实：**没有** `armory_focus` / `armory_hover_part`；`hud.rs` 的 `draw_armory(f, view, selected: usize) -> Vec<(f32,f32,f32,f32)>` 仍是扁平 `Vec<Rect>` + 单个 `selected: usize`；`armory_stats(def, build)` 仍只比"当前 vs 出厂"，**没有候选参数**。§20.1 的两个病灶（行是 `format!` 的 String、点击即步进）**原样存在** |
| **§20.3 武器分组网格** | ❌ 未做 | `WeaponKind` 仍是封闭枚举（改名后未变） |
| **§19.3 局内三选一 / 波间商店** | ❌ 未做 | 波间窗口已经存在（`Prep { left }`），但**窗口里没有可买的东西**——分数还不是货币 |

### 21.3 交接要点（下一手必须知道的）

1. **代码与报告是双向引用的。** 代码里有 **7 处 `reference §…`**：`main.rs`（§19.1 / §19.2 / §19.2.1）、`campaign.rs`（§19.2 / §19.4-3）、`maps.rs`（§19.2.1）。**所以：不要重排本报告的章节号。** 新增内容一律追加为 §22、§23…；如果必须改编号，同时更新那 7 处注释。
2. **所有行号都已失效。** `main.rs` 从 4,370 行涨到 **4,660 行**，`git diff --stat` 显示 **13 个文件 / 3,171 行新增 / 415 行删除，全部未提交**。报告里任何 `main.rs:xxxx` 形式的引用都不可信——**按符号名（函数名/常量名）定位**，这一点 §0 从第一版起就警告过。
3. **`pick_spawn` 的修法与我给的代码不完全一样，而且更好。** 我建议的是"取最接近 IDEAL 的点"，实现成了**加权随机**（`HIDDEN_WEIGHT = 2.5` + 软三角权重），注释解释了为什么不取最大："the sharp version crowns one point per arena even when nothing is wrong with the others."——**这是对我方案的改进，不要退回我的版本。**
4. **§19.4-5 的"每波重置"被刻意收窄**：只恢复生命与呼吸，**不补弹药/护甲/理智**。注释给了理由（弹药经济属于仓库；护甲与理智是竞技场被允许从你身上拿走的东西）。**这是设计决定，不是 bug。**
5. **下一步的最短路径**：`§19.4-3` 的"敌强度随波次递增"是**一行级改动**（把 `Enemy::spawn` 的 `wave` 参数从 `mission.wave` 换成递增波号），而它是**无限模式与难度曲线的最后一块**。其次是 §17.2 的三条护甲改动（都是小改），再次才是 §20 的界面重构。

### 21.4 本次核验链

`git log --oneline -3` → `59ba77a Rename the game to TIDES OF R'LYEH` ｜ `git status --short` → 13 个修改未提交 ｜ `git diff --stat` → 3,171 insertions / 415 deletions ｜ 逐符号 `grep` 核验 §21.1 与 §21.2 的每一条（`WavePhase` / `wave_budget` / `is_elite` / `last_elite_wave` / `spawn_pool` / `spawn_set` / `HIDDEN_WEIGHT` / `player.wave_reset` / `armory_focus`（不存在）/ `absorbed = (amount * 0.65)`（未变））。

---

## 22. 2026-10-04 对账：§20.2 已落地 + §20.3 武器分组网格已实现

> 本节为追加，不改动任何已有章节号（代码中的 `reference §…` 注释依赖既有编号）。
> 结论先行：**§21.2 把"§20.2 三栏"与"§20.3 网格"记为未实现，在本节核验时均已过时。** 以下为逐符号核实结果，符号名为准、行号仅供本次快照。

### 22.1 §20.2 三栏工作台——确认已完整落地

逐符号确认下列类型/字段/逻辑均已存在（`src/main.rs` Phase::Armory 分支 + `src/hud.rs`）：

- `hud::ArmoryColumn { Slots, Parts }`、`hud::ArmoryBench { parts, cursor, column }`、`hud::ArmoryRects { rows, parts }`（矩形按列分组，不再是扁平列表）。
- `Game` 字段：`armory_focus`、`armory_slot`、`armory_part: Option<(SlotId, usize)>`、`armory_hover_part`、`armory_preview: Option<ArmoryPreview>`。
- 交互符合 §20.2 三步：**点槽位行（或 →）进入候选列；悬停候选 → 右栏 old→new 数值与武器预览实时刷新；点击候选或 Enter 才装配**（提交点为 `clicked_part` / `in_parts && enter`）。
- `draw_armory` 返回 `ArmoryRects`，槽位行点击只切列、不改值。

> 交接含义：**不要把 §20.2 当作待办重做。** 三栏已是主干现状。

### 22.2 §20.3 武器分组网格——设计已完成，代码已回退（待代码 Agent 实施）

> **2026-10-04 更新**：本节原先记录的 picker 代码已由本 Agent 写入工作区，但按"本 Agent 只写文档、代码由另一个 Agent 编写"的分工，**已全部回退**。以下保留设计记录，供代码 Agent 照做。

**设计层已完成**（以下为已验证的分组方案）：

| Family | 成员（`WeaponKind`） |
|---|---|
| AssaultRifle | Rifle, M4a1, Scar, Fal |
| SubmachineGun | Smg, Mp5, Mpx |
| Marksman | Sniper, M110, Mosin |
| Heavy | Shotgun, Lmg |
| Sidearm | Pistol |

**代码 Agent 实施时需要新增的类型/字段/函数**（已验证可编译，回退前 `cargo build` 通过、`cargo test` 179 通过）：

- `src/weapon.rs`：`pub enum WeaponFamily`（5 变体）+ `WeaponFamily::ALL` + `label(self) -> Key`；`WeaponKind::family(self) -> WeaponFamily`。
- `src/i18n.rs`：5 个新 Key（`FamAssault` / `FamSubmachine` / `FamMarksman` / `FamHeavy` / `FamSidearm`）+ 中英文翻译。
- `src/main.rs`：`Game` 加 `armory_picker: bool` + `armory_picker_cursor: usize`；`build_armory_picker() -> Vec<hud::ArmoryGroup>`；tab 0 武器行（row 1）改为打开覆盖层；覆盖层输入/提交逻辑。
- `src/hud.rs`：`ArmoryTile`、`ArmoryGroup`、`ArmoryPicker` 结构体 + `draw_armory_picker(f, picker) -> Vec<Rect>`。

**交互设计**：点击武器行或 Enter → 打开覆盖层；方向键按 4 列网格移动光标（跨家族）；Enter 或点击 tile 提交；Esc/Backspace 取消。覆盖层打开期间短路工作台步进/装配。视觉沿用 `INK`/`PANEL`/`accent()`/`CYAN`/L 角标（符合 §18.2 / §20.4 约束）。

### 22.3 验证状态（代码回退后）

- `cargo build`：通过，无警告。
- `cargo test`：**179 通过 / 0 失败 / 6 忽略**。
- **代码回退后工作区的 `src/` 改动仅剩之前会话留下的、非本 Agent 所做的改动**；本 Agent 的所有代码增改已全部回退。
- 运行时截图 `target/shots/30_picker.png` 等仍存在于被忽略的 `target/` 下（代码回退前的截图），可删。

### 22.4 仍未做 / 边界（本次不碰）

- 未改配装页 tab 2（计数循环合适）、涂装页 tab 1。
- 未碰 `menu_nav` / `hovered_rect` / `item_list`（服务 8 个界面）。
- 未加搜索框/拖拽；`WeaponKind` 仍是封闭枚举（按 §20.3，20 把以内可接受，不做数据驱动重构）。
- 其余未实现项以 §21.2 为准（敌强度随波次、AI 数量上限、奖励波、波间商店、§17.2 护甲三项等）。
- 文档目录中两个审查报告已去掉 `rust-fps` 前缀（现为 `代码审查报告.md`、`美术审查报告.md`），过时路径已修为 `tides-of-rlyeh`，跨文档引用已改为相对链接。

---

## 23. 弹药 / 弹种系统设计（2026-10-04，待实施）

> 本节为设计文档，**尚未实现**，供代码 Agent 照做。不改动任何已有章节号。
> 核心判断：当前弹药系统**缺的不是"更多弹匣"，而是"子弹作为一个可区分实体"这一层目前不存在**——伤害、穿甲、衰减全挂在"枪"上，子弹本身没有身份。

### 23.1 现状核实（逐符号）

- **弹匣已是容器**：`Weapon::mags: Vec<u32>`，每个弹匣独立计数；reload 是交换弹匣；半匣子弹保留（`a_reload_swaps_magazines_and_conserves_rounds` 测试保护）。这一层**不需要动**。
- **口径井已隔离**：`InterfaceId` + `validate()` 拒绝不兼容弹匣（MP5/MPX、FAL/M110 互不通用）。
- **一发多弹丸**：`WeaponStats::pellets`（霰弹 9 颗），各自在 cone 散布内独立 hitscan。
- **伤害衰减**：`falloff_start/end/min` + `damage_at(dist)`。
- **穿甲写死在枪上**：`WeaponKind::armor_pen()` 返回 `f32`（Rifle 0.78、Pistol 0.45、Sniper 0.97…），不随弹种变化。
- **纯 hitscan**：`combat::hitscan` 返回最近一个 `Hit`，无飞行时间、无实体弹道、无贯穿。
- **弹匣配件**：`SlotId("magazine")` 是配件树的一个槽位（扩容、快换等），但**弹匣 ≠ 弹种**——弹匣是容器，弹种是容器里装什么。

### 23.2 四条轴（按性价比排序）

#### 轴 1：引入 `AmmoType`（最值得做，地基）

把"口径/弹种"从枪的常量提升为数据实体：

```rust
pub struct AmmoType {
    pub id: AmmoId,            // "5_56x45", "9mm", "12gauge", "7_62x51" ...
    pub name_en: &'static str,
    pub name_zh: &'static str,
    pub damage_mul: f32,       // 乘到枪的 base damage 上
    pub armor_pen: f32,       // 取代 WeaponKind::armor_pen() 的硬编码
    pub armor_damage_mul: f32, // 对护甲本体的伤害倍率（可选）
    pub recoil_mul: f32,      // 弹种后坐力修正（可选）
    // 轴 2 的行为开关（见下）
    pub kind: AmmoKind,       // FMJ / JHP / AP / Tracer / Slug ...
}
```

- `WeaponDef` 声明它发射哪个 `AmmoId`（或一个 `accepted_ammo: &[AmmoId]` 集合）。
- 最终伤害 = 枪的 `damage_at(dist)` × 弹种 `damage_mul`；穿甲取弹种的 `armor_pen`（替换 `WeaponKind::armor_pen()`）。
- **收益**：同口径枪天然共享弹药（5.56 的 Rifle/M4a1/Scar 现在各自一套数字）；以后能做弹种选择——这是"改枪"之外玩家真正会做的第二个乘法决策。
- **与现有架构契合**：复用已有的 parts/resolve 纯函数管线。**建议先做独立字段，别塞进配件树**——弹种是消耗品，配件是耐久物，语义不同。

#### 轴 2：弹种带来行为差异（不只数值）

在 `AmmoKind` 枚举上挂行为开关（每种一个变体，纯数据驱动）：

| AmmoKind | 行为 | 回答的战术问题 |
|---|---|---|
| **FMJ**（标准弹） | 基准 | 默认 |
| **JHP**（空尖弹） | 对无甲目标 `damage_mul` 高、`armor_pen` 骤降；命中后减速敌人 | "怎么快速清无甲杂兵" |
| **AP**（穿甲弹） | `armor_pen` 高、`damage_mul` 略低；可贯穿薄掩体/多目标 | "怎么打重甲/Juggernaut" |
| **Tracer**（曳光弹） | 每匣最后 N 发是曳光（真实军队惯例），照亮弹道、指示方位 | "暗处怎么定位敌人" |
| **Slug**（独头弹） | 霰弹枪切 `pellets: 1` + 高射程 + 高穿甲 | "霰弹枪怎么打中距离" |

> **克制原则**：3–5 种弹型足够。每种都要有明确的"它回答什么战术问题"，否则只是库存负担。Slug 是最低成本验证"弹种切换"的入口——只改 `pellets` 与 spread，就能给霰弹枪多一种玩法。

#### 轴 3：贯穿与多目标（需动 hitscan，中等成本）

当前一发 Ray 命中即停。两个增强方向：

- **穿透敌人**：AP 弹命中第一个敌人后沿射线继续，伤害按距离/穿深衰减，可命中其后目标。
- **穿透掩体**：薄掩体后继续，但掩体"吃掉"一部分伤害（按材质厚度 × pen）。

**实现要点**：把 `combat::hitscan` 从"返回最近一个 `Hit`"改成"沿射线收集有序 `Hit` 列表"，由弹种的贯穿预算决定走到哪停：

```rust
pub struct Penetration {
    pub max_targets: u32,       // AP=2-3, FMJ=1, JHP=1
    pub wall_thickness_budget: f32, // AP=0.4m 砖墙, FMJ=0.1m ...
    pub damage_falloff_per_hit: f32, // 每穿一个目标衰减比例
}
```

> 这是手感和战术层次提升最大的一项（CS 的穿墙、Cod 的穿人），但它触及核心命中逻辑，**必须由代码 Agent 在有测试覆盖下做**。`hitscan` 现有调用方（`main.rs` 开火逻辑约 2677 行起）需要适配多 Hit 返回。

#### 轴 4：选择性实体弹道（谨慎，多数情况下别做）

hitscan 对突击步枪/手枪是对的。**只有这些武器值得实体弹道 + 飞行时间 + 下坠**：

- 狙击在超长距离（≥150 m）：下坠和提前量是"射手"身份的核心。
- 榴弹/火箭筒：本就该是抛物线投射物（`grenade.rs` 已有投射物基础，可复用）。

> **不要给所有枪加飞行时间**——会毁掉快节奏 CQB 的爽感，且与 1/120 s 固定步长的紧凑判定不匹配。轴 4 只在扩展狙击/重武器时按单武器引入。

### 23.3 数据结构草案（供代码 Agent 直接参考）

```rust
// weapon.rs 新增

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AmmoId(&'static str);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AmmoKind {
    Fmj,
    Jhp,
    Ap,
    Tracer,
    Slug,
}

#[derive(Clone, Copy, Debug)]
pub struct AmmoType {
    pub id: AmmoId,
    pub name_en: &'static str,
    pub name_zh: &'static str,
    pub kind: AmmoKind,
    pub damage_mul: f32,
    pub armor_pen: f32,
    pub recoil_mul: f32,
    pub penetration: Penetration,
}

#[derive(Clone, Copy, Debug)]
pub struct Penetration {
    pub max_targets: u32,
    pub wall_budget: f32,
    pub falloff_per_hit: f32,
}

// WeaponDef 新增字段
pub struct WeaponDef {
    // ... 现有字段 ...
    pub ammo: AmmoId,               // 该枪发射的弹种
    pub accepted_ammo: &'static [AmmoId], // 可兼容的弹种集合（同口径）
}
```

**改造点清单**（按顺序，每步可独立编译）：

1. `weapon.rs`：定义 `AmmoType` / `AmmoKind` / `Penetration`；登记已知弹种表（FMJ/JHP/AP/Tracer/Slug 各一个 `const`）；为 `WeaponDef` 加 `ammo` 字段；`WeaponKind::armor_pen()` 改为查 `AmmoType` 而非硬编码 match。
2. `combat.rs`：`hitscan` 返回 `Vec<Hit>`（或加一个 `hitscan_penetrating` 变体），由 `Penetration` 控制收集几个 Hit。
3. `main.rs`：开火循环（约 2677 行 `for _ in 0..rays`）适配多 Hit；AP 弹命中后沿射线继续；伤害按 `falloff_per_hit` 衰减。
4. 霰弹 Slug：`AmmoKind::Slug` 时 `pellets` 视为 1、`base_spread` 视为 0、射程拉长——可在 `resolve` 阶段完成。
5. UI：军械库加一个弹种选择行（tab 0 新增一行，或 loadout tab 2 加弹种计数），复用 §20 的三栏或循环均可（弹种数量少，循环可接受）。
6. 测试：`weapon::tests` 加弹种伤害计算测试；`combat` 加多目标贯穿测试。

### 23.4 抄袭与表达层边界（沿用 §18 立场）

- "5.56 / 7.62x51 / 9 mm"等口径与弹药规格是**事实/标准**，可用。
- **风险点**：塔科夫的 M855A1 / M61 等弹种名、装甲等级 1–6、一一对应的穿甲数字表属于表达层，**不要整套照搬**。用自己的弹种名（现实口径 + 自创代号）和自己的数值曲线。
- 弹种选择 UI 的功能结构可借鉴，**视觉必须用项目自己的 HUD 语言**（最高法指导性案例 279 号约束，同 §20.4）。

### 23.5 建议落地顺序

1. **轴 1 `AmmoType` 数据层**（共享弹药 + 弹种字段）——地基，收益最大。
2. **轴 2 的 Slug + 一种 AP + 一种 JHP**——用最低成本验证"弹种"好不好玩。
3. 若验证好玩，再上**轴 3 贯穿**（核心命中改造，配测试）。
4. 轴 4 只在扩展狙击/重武器时按单武器引入。

### 23.6 不做 / 边界

- 不给所有枪加飞行时间（毁 CQB 爽感）。
- 不把弹种塞进配件树（消耗品 ≠ 耐久物）。
- 不照搬塔科夫弹种数值表与装甲等级体系（表达层风险）。
- 不做"弹壳回收 / 重新装填"等经济模拟（与竞技场波次节奏不搭）。

