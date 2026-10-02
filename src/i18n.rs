use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En,
    Zh,
}

impl Lang {
    pub fn next(self) -> Self {
        match self {
            Lang::En => Lang::Zh,
            Lang::Zh => Lang::En,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Lang::En => "ENGLISH",
            Lang::Zh => "中文",
        }
    }

    fn code(self) -> u8 {
        match self {
            Lang::En => 0,
            Lang::Zh => 1,
        }
    }

    fn from_code(v: u8) -> Self {
        if v == 1 { Lang::Zh } else { Lang::En }
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set_lang(l: Lang) {
    CURRENT.store(l.code(), Ordering::Relaxed);
}

pub fn lang() -> Lang {
    Lang::from_code(CURRENT.load(Ordering::Relaxed))
}

macro_rules! define_strings {
    ($($key:ident => ($en:expr, $zh:expr)),+ $(,)?) => {
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        // Grouped prefixes (`Weapon*`, `Enemy*`, `Obj*`) are deliberate: this is
        // a flattened string table, and namespacing the keys keeps call sites
        // readable and stops similar strings from colliding.
        #[allow(clippy::enum_variant_names)]
        // This is a localization table: it deliberately holds every user-facing
        // string, including the ones whose screens are not written yet, and
        // `get` fills in both languages for every key. An unconstructed variant
        // is a string waiting for its UI, not dead code.
        #[allow(dead_code)]
        pub enum Key {
            $($key,)+
        }

        impl Key {
            pub fn get(self, l: Lang) -> &'static str {
                match self {
                    $(Key::$key => match l {
                        Lang::En => $en,
                        Lang::Zh => $zh,
                    },)+
                }
            }
        }
    };
}

define_strings! {
    // --- shell ---
    Title            => ("STEEL FRONT", "钢铁前线"),
    Subtitle         => ("A TACTICAL FPS", "战术第一人称射击"),
    NewCampaign      => ("NEW CAMPAIGN", "开始战役"),
    ContinueGame     => ("CONTINUE", "继续游戏"),
    SelectMission    => ("SELECT MISSION", "选择关卡"),
    HowToPlay        => ("HOW TO PLAY", "玩法介绍"),
    Codex            => ("CODEX", "图鉴"),
    ManualTitleEnemies => ("HOSTILES", "敌人图鉴"),
    ManualTitleKit   => ("WEAPONS AND KIT", "武器与装备"),
    ManualPages      => ("LEFT / RIGHT PAGE", "← / → 翻页"),
    Settings         => ("SETTINGS", "设置"),
    Quit             => ("QUIT", "退出"),
    Back             => ("BACK", "返回"),
    Resume           => ("RESUME", "继续游戏"),
    Restart          => ("RESTART MISSION", "重新开始"),
    MainMenu         => ("MAIN MENU", "返回主菜单"),
    Paused           => ("PAUSED", "已暂停"),
    PressAnyKey      => ("PRESS ENTER TO CONTINUE", "按回车继续"),
    Locked           => ("LOCKED", "未解锁"),
    ClickToStart     => ("CLICK TO CAPTURE MOUSE", "点击以锁定鼠标"),
    Language         => ("LANGUAGE", "语言"),

    // --- settings ---
    Sensitivity      => ("MOUSE SENSITIVITY", "鼠标灵敏度"),
    Volume           => ("MASTER VOLUME", "主音量"),
    InvertY          => ("INVERT MOUSE Y", "反转鼠标 Y 轴"),
    ShowFps          => ("SHOW FPS", "显示帧率"),
    Fullscreen       => ("FULLSCREEN", "全屏"),
    Resolution       => ("RESOLUTION", "分辨率"),
    On               => ("ON", "开"),
    Off              => ("OFF", "关"),

    // --- hud ---
    Health           => ("HEALTH", "生命"),
    Ammo             => ("AMMO", "弹药"),
    Reloading        => ("RELOADING", "换弹中"),
    ReloadPrompt     => ("PRESS R TO RELOAD", "按 R 换弹"),
    Score            => ("SCORE", "得分"),
    Kills            => ("KILLS", "击杀"),
    Time             => ("TIME", "用时"),
    Accuracy         => ("ACCURACY", "命中率"),
    Objective        => ("OBJECTIVE", "任务目标"),
    Hostiles         => ("HOSTILES", "剩余敌人"),
    ObjectiveDone    => ("OBJECTIVE COMPLETE", "目标已完成"),
    LowHealth        => ("CRITICAL", "生命垂危"),

    // --- objectives ---
    ObjEliminate     => ("Eliminate all hostiles", "消灭所有敌人"),
    ObjReach         => ("Reach the extraction point", "抵达撤离点"),
    ObjSurvive       => ("Hold out until extraction", "坚守至撤离"),
    ObjDestroy       => ("Destroy the marked targets", "摧毁标记目标"),
    ObjIntel         => ("Recover the intel", "回收情报"),
    ObjThenExtract   => ("Then reach extraction", "然后抵达撤离点"),

    // --- mission results ---
    MissionComplete  => ("MISSION COMPLETE", "任务完成"),
    MissionFailed    => ("MISSION FAILED", "任务失败"),
    NextMission      => ("NEXT MISSION", "下一关"),
    RetryMission     => ("RETRY MISSION", "重试本关"),
    CampaignCleared  => ("CAMPAIGN COMPLETE", "战役通关"),
    YouWereKilled    => ("YOU WERE KILLED", "你阵亡了"),

    // --- pickups ---
    PickHealth       => ("MEDKIT", "医疗包"),
    PickAmmo         => ("AMMO", "弹药"),
    AmmoFull         => ("AMMO FULL", "弹药已满"),
    ArmorFull        => ("ARMOUR ALREADY FULL", "护甲已满"),
    PickArmor        => ("ARMOR", "护甲"),
    PickWeapon       => ("WEAPON", "武器"),
    PressToPickUp    => ("PRESS F TO PICK UP", "按 F 拾取"),

    // --- weapons ---
    WeaponRifle      => ("MK-7 RIFLE", "MK-7 突击步枪"),
    WeaponShotgun    => ("BREACHER 12", "破门者-12 霰弹枪"),
    WeaponSmg        => ("VIPER SMG", "毒蛇 冲锋枪"),
    WeaponSniper     => ("LONGSHOT .50", "远程 .50 狙击枪"),
    WeaponLmg        => ("VULCAN LMG", "火神 轻机枪"),

    // --- enemy archetypes (for kill feed / bestiary) ---
    EnemyGrunt       => ("GRUNT", "步兵"),
    EnemyRusher      => ("STALKER", "突袭者"),
    EnemyHeavy       => ("JUGGERNAUT", "重装兵"),
    EnemySniper      => ("MARKSMAN", "射手"),
    EnemyDrone       => ("DRONE", "无人机"),

    // --- stances ---
    StanceStand      => ("STANDING", "站立"),
    StanceCrouch     => ("CROUCHED", "蹲下"),
    StanceProne      => ("PRONE", "趴下"),
    StanceWalk       => ("WALKING", "静步"),
    CannotStand      => ("NO ROOM TO STAND", "上方空间不足"),

    // --- fire modes ---
    FireAuto         => ("AUTO", "全自动"),
    FireBurst        => ("BURST", "三连发"),
    FireSingle       => ("SINGLE", "单发"),
    FireModeChanged  => ("FIRE MODE", "开火模式"),
    FireModeIs       => ("FIRE MODE IS", "当前开火模式"),

    // --- magazine check ---
    CheckingMag      => ("CHECKING MAGAZINE", "检查弹匣"),
    MagFull          => ("MAGAZINE FULL", "弹匣已满"),
    MagHigh          => ("MAGAZINE MOSTLY FULL", "弹匣基本满"),
    MagHalf          => ("MAGAZINE ABOUT HALF", "弹匣约一半"),
    MagLow           => ("MAGAZINE RUNNING LOW", "弹匣即将打空"),
    MagEmpty         => ("MAGAZINE EMPTY", "弹匣已空"),
    Inspecting       => ("INSPECTING", "检视武器"),

    // --- grenades ---
    GrenadeFrag      => ("FRAG", "破片手雷"),
    GrenadeSmoke     => ("SMOKE", "烟雾弹"),
    GrenadeWheel     => ("SELECT GRENADE", "选择投掷物"),
    NoGrenades       => ("NO GRENADES", "没有投掷物"),
    GrenadeOut       => ("GRENADE OUT", "投掷物已出手"),

    // --- medical ---
    MedBandage       => ("BANDAGE", "绷带"),
    MedKit           => ("MEDKIT", "医疗包"),
    MedPainkiller    => ("PAINKILLER", "止痛药"),
    MedWheel         => ("SELECT MEDICAL", "选择药品"),
    NoMeds           => ("NO MEDICAL SUPPLIES", "没有药品"),
    Healing          => ("HEALING", "治疗中"),
    HealInterrupted  => ("HEALING INTERRUPTED", "治疗被打断"),
    HealthFull       => ("HEALTH ALREADY FULL", "生命值已满"),

    // --- screens ---
    Inventory        => ("INVENTORY", "背包"),
    MapScreen        => ("MAP", "地图"),
    MissionPanel     => ("MISSION", "任务面板"),
    SlotPrimary      => ("PRIMARY", "主武器"),
    SlotSecondary    => ("SECONDARY", "副武器"),
    SlotPistol       => ("SIDEARM", "手枪"),
    SlotMelee        => ("MELEE", "近战"),
    Equipment        => ("EQUIPMENT", "装备"),
    Consumables      => ("CONSUMABLES", "消耗品"),
    Objectives       => ("OBJECTIVES", "任务目标"),
    Statistics       => ("STATISTICS", "统计"),
    HintTab          => ("TAB INVENTORY", "TAB 背包"),
    HintMap          => ("M MAP", "M 地图"),
    HintMission      => ("J MISSION", "J 任务面板"),
    HintClose        => ("ESC OR SAME KEY TO CLOSE", "按 ESC 或同一按键关闭"),

    // --- weapons ---
    WeaponPistol     => ("SIDEARM M9", "M9 手枪"),
    WeaponKnife      => ("COMBAT KNIFE", "战术匕首"),

    // --- breath ---
    BreathHeld       => ("HOLDING BREATH", "屏息中"),
    BreathSpent      => ("OUT OF BREATH", "气息已尽"),

    // --- free look ---
    FreeLook         => ("FREE LOOK", "自由视角"),

    // --- cosmic horror ---
    EnemyTentacle    => ("TENTACLE HORROR", "触手怪"),
    EnemyColossus    => ("THE COLOSSUS", "巨像"),
    EnemySpawnling   => ("SPAWN", "子体"),
    Sanity           => ("SANITY", "理智"),
    SanityFraying    => ("YOUR MIND IS FRAYING", "理智正在崩解"),
    SanityRestored   => ("MIND CLEARED", "理智恢复"),
    AbyssTaken       => ("THE ABYSS TOOK YOU", "深渊吞噬了你"),
    ColossusStirs    => ("SOMETHING ENORMOUS STIRS", "某种庞然巨物正在苏醒"),

    // --- aiming ---
    AimMode          => ("AIM MODE", "瞄准方式"),
    LeanMode         => ("LEAN MODE", "探头方式"),
    AimHold          => ("HOLD", "长按"),
    AimToggle        => ("TOGGLE", "切换"),
    LeanHint         => ("Q / E", "Q / E"),

    // --- field manual: the controls page ---
    // The key glyphs are language-independent, so every control row is one
    // string: what to press and what it does, in the order the hand meets it.
    ManualTitleControls => ("CONTROLS", "操作说明"),
    ManualGoal       => ("HOW A MISSION RUNS", "任务怎么打"),
    ManualGoalBody   => ("Clear every hostile, then reach extraction — or hold out until the clock opens it. Dying restarts the mission. Codex figures are an archetype's base values; later missions send more, and tougher, enemies.",
                         "清光本关的所有敌人，然后抵达撤离点；坚守任务则等倒计时走完开启撤离点。阵亡就从头重来，没有中途存档。图鉴数值是兵种基础值：关卡越靠后，敌人越多越硬。"),
    ManualSectionMoving => ("MOVEMENT", "移动"),
    ManualSectionFighting => ("COMBAT", "战斗"),
    ManualSectionScreens => ("SCREENS", "界面"),
    ManualSectionSurvival => ("SURVIVAL", "生存要点"),

    CtlMove          => ("W A S D　MOVE", "W A S D　移动"),
    CtlSprint        => ("SHIFT (TAP)　SPRINT", "Shift（轻按）　冲刺"),
    CtlJump          => ("SPACE　JUMP", "空格　跳跃"),
    CtlCrouch        => ("C　CROUCH (TOGGLE)", "C　蹲下（点按切换）"),
    CtlProne         => ("Z　PRONE (TOGGLE)", "Z　趴下（点按切换）"),
    CtlWalk          => ("CAPS LOCK (HOLD)　WALK QUIETLY", "CapsLock（按住）　静步"),
    CtlLean          => ("Q / E　LEAN LEFT / RIGHT", "Q / E　左右探头"),
    CtlFire          => ("LEFT MOUSE　FIRE", "鼠标左键　开火"),
    CtlAim           => ("RIGHT MOUSE　AIM DOWN SIGHTS", "鼠标右键　开镜"),
    CtlReload        => ("R　RELOAD", "R　换弹"),
    CtlFireMode      => ("B　CYCLE FIRE MODE", "B　切换开火模式"),
    CtlCheckMode     => ("ALT + B　CHECK FIRE MODE", "Alt + B　查询当前开火模式"),
    CtlCheckMag      => ("ALT + T　CHECK MAGAZINE", "Alt + T　检查弹匣余量"),
    CtlHoldBreath    => ("ALT (AIMING)　HOLD BREATH", "Alt（开镜时按住）　屏息"),
    CtlFreeLook      => ("MIDDLE MOUSE (HOLD)　FREE LOOK", "鼠标中键（按住）　自由视角"),
    CtlInspect       => ("U　INSPECT WEAPON", "U　检视武器"),
    CtlWeapons       => ("1 2 3 4 / WHEEL　SWITCH WEAPON", "1 2 3 4 / 滚轮　切换武器"),
    CtlGrenade       => ("G　THROW GRENADE · HOLD TO PICK", "G　投掷手雷；按住可选种类"),
    CtlHeal          => ("V　USE MEDICAL · HOLD TO PICK", "V　使用药品；按住可选种类"),
    // No key: the game has no interact verb — walking within a metre of a drop
    // picks it up. The row is here so a new player looking for "how do I pick
    // things up" finds the answer instead of a key that does nothing.
    CtlInteract      => ("WALK OVER　PICK UP DROPS", "走近　自动拾取掉落物"),
    CtlInventory     => ("TAB　INVENTORY", "Tab　背包"),
    CtlMap           => ("M　MAP", "M　地图"),
    CtlMission       => ("J　MISSION PANEL", "J　任务面板"),
    CtlHelp          => ("H　THIS MANUAL", "H　本手册"),
    CtlCodex          => ("K　CODEX", "K　图鉴"),
    CtlPause         => ("ESC　PAUSE / CLOSE SCREEN", "Esc　暂停 / 关闭界面"),

    TipSprint        => ("Sprint is a tap, not a hold: it latches when you press Shift and drops by itself when you stop pushing forward or raise the sights.",
                         "冲刺是点按不是长按：按一下 Shift 就锁定，停止前进或开镜时会自动解除。"),
    TipAim           => ("Aiming steadies the shot but halves your turn speed. Add held breath and the sway almost disappears.",
                         "开镜能压住散布，但转向速度减半；再按住屏息，准星几乎不再抖。"),
    TipArmor         => ("Armour absorbs 65% of every hit until the plate is used up. Pick it up the moment you see it.",
                         "护甲会吸收每发伤害的 65%，直到耗尽为止——看到就捡。"),
    TipCover         => ("Nothing can shoot you without line of sight. Break the line: crouch behind cover, lean out to return fire, smoke the open ground.",
                         "敌人看不见你就打不到你：蹲在掩体后、用探头还击、拿烟雾遮住开阔地。"),
    TipSanity        => ("Your mind is drained by what you look at, not by what hits you. Turn away and it comes back — and low sanity makes the aim wander.",
                         "理智是被「看」掉的，不是被打掉的：移开视线就会恢复；理智越低，准星越飘。"),
    TipPickups       => ("Health, ammo and armour drop where hostiles fall. Walking over them is enough.",
                         "敌人倒下的地方会掉落药品、弹药和护甲，走过去就能捡。"),
    TipFireModes     => ("B cycles the fire mode and Alt+B only reports it. Alt+T checks the magazine: a rough band plus the exact round count.",
                         "B 切换开火模式，Alt+B 只查询；Alt+T 检查弹匣，给出大致余量和准确发数。"),

    // --- field manual: bestiary ---
    // The notes are one line of a card each — about forty characters of CJK at
    // the card width the bestiary uses — so they carry the number that changes
    // how the fight is fought and nothing else. The full figures are already on
    // the line above them.
    CodexGruntTag    => ("BASELINE RIFLEMAN", "标准步枪兵"),
    CodexGruntNote   => ("Holds about 12 m and fires one aimed shot a second. Use cover; aim for the head.",
                         "在 12 米外每秒一发点射。借掩体对射，尽量打头。"),
    CodexRusherTag   => ("FAST MELEE CHARGER", "贴脸白刃，速度极快"),
    CodexRusherNote  => ("Closes to 1.2 m and claws for 13. Standing you outrun it; crouched you do not.",
                         "冲到 1.2 米内挥砍 13 点。站着跑得掉，蹲行静步跑不掉。"),
    CodexHeavyTag    => ("ARMOURED BURST GUNNER", "重甲三连发"),
    CodexHeavyNote   => ("Slow, armoured, three 15-damage rounds per burst. Flank it; do not trade shots.",
                         "慢速重甲，三连发每发 15 点。别正面换血，绕侧面打。"),
    // The marksman's keys are `Marksman`, not `Sniper`: the archetype's own
    // name is MARKSMAN, and `Sniper` is the *weapon*'s subject below — one
    // string table cannot hold both under the same name.
    CodexMarksmanTag => ("TELEGRAPHED LONG SHOT", "远程狙击手"),
    CodexMarksmanNote => ("Sees 55 m and takes 0.95 s to aim. A hit costs 34 — be behind cover by then.",
                          "射程 55 米，举枪 0.95 秒后一枪 34 点。看到动作就缩回掩体。"),
    CodexDroneTag    => ("HOVERING SKIRMISHER", "空中游猎者"),
    CodexDroneNote   => ("Hovers at 3 m, fast and erratic, 6 a burst. Lead it and shoot up.",
                         "悬停在 3 米高，走位飘，点射每发 6 点。抬枪预判就能打中。"),
    CodexTentacleTag => ("LONG-REACH HORROR", "远端抽击的异形"),
    CodexTentacleNote => ("Costs 3 sanity a second in view, and its corpse releases three spawnlings.",
                          "盯着看每秒掉 3 点理智；死后尸体会放出三只子体。"),
    CodexColossusTag => ("A WALKING MOUNTAIN", "行走的山"),
    CodexColossusNote => ("900 HP: a full magazine and eight more rounds. Headshots and frags are fastest.",
                          "900 血，打空整匣再补八发。爆头与破片手雷最快。"),
    CodexSpawnlingTag => ("SWARM HATCHLING", "成群涌上来的小东西"),
    CodexSpawnlingNote => ("18 HP and fast: one shot each. The trouble is the three a dead tentacle leaves.",
                           "18 血、极快，一枪一个。麻烦的是触手怪死后冒出的三只。"),

    // --- field manual: armoury ---
    CodexRifleTag    => ("FULL-AUTO BASELINE", "全自动基准枪"),
    CodexRifleNote   => ("Auto, burst or single; 30 rounds, 1.35 s reload. The most forgiving gun here.",
                         "全自动 / 三连发 / 单发随切，30 发弹匣，最省心的一把。"),
    CodexShotgunTag  => ("SHORT-RANGE STOPPING POWER", "近距离一锤定音"),
    CodexShotgunNote => ("Nine 15-damage pellets in one pull: deadly inside 8 m, a quarter of that at 26 m.",
                         "一次击发 9 颗 15 点弹丸：8 米内摧枯拉朽，26 米外只剩四分之一。"),
    CodexSmgTag      => ("HIGH-CADENCE SPRAYER", "高射速泼水枪"),
    CodexSmgNote     => ("40 rounds at 1091 RPM, with the bloom to match. Halved by 45 m.",
                         "40 发弹匣、1091 RPM，散布跟着涨；45 米外伤害只剩一半。"),
    CodexSniperTag   => ("LONG-RANGE PRECISION", "一枪定生死"),
    CodexSniperNote  => ("115 a shot, no falloff at any range, 16-degree scope. Five rounds.",
                         "单发 115 点、任何距离都不衰减，配 16 度瞄准镜；5 发一匣。"),
    CodexLmgTag      => ("SUSTAINED FIRE", "持续压制"),
    CodexLmgNote     => ("100 rounds of 21 without letting up — and the heaviest thing you can carry: 82% speed.",
                         "100 发弹链砸 21 点一发，代价是全场最沉：移速只剩 82%。"),
    CodexPistolTag   => ("ALWAYS ON THE HIP", "随身副武器"),
    CodexPistolNote  => ("26 a shot, the best headshot multiplier at 2.6x, and a 1.25 s reload.",
                         "单发 26 点、全场最高的 2.6 倍爆头、1.25 秒换弹。"),
    CodexKnifeTag    => ("SILENT LAST RESORT", "无声的最后手段"),
    CodexKnifeNote   => ("55 a swing, no ammunition, no noise — and the lightest kit: 118% speed.",
                         "每刀 55 点，无需弹药、没有枪声；最轻的装备，移速 118%。"),

    // --- field manual: kit ---
    // These seven carry the manual's one class of copied number: `grenade.rs`
    // keeps its fuse, blast radius and damage private, `main.rs` keeps the
    // medical and armour figures private, and a module that can only see the
    // public API cannot read them. They are quoted beside the names so the copy
    // is visible in one place and a search for the constant finds it.
    CodexItemFrag    => ("FRAG — 3.2 s fuse; 130 damage at the centre of a 7 m blast, cover blocks it",
                         "破片手雷 — 投出后 3.2 秒起爆；7 米半径内越近越疼（中心 130 点），掩体能挡住"),
    CodexItemSmoke   => ("SMOKE — blooms on landing into a 5 m cloud that blocks every line of sight for 16 s",
                         "烟雾弹 — 落地即炸开 5 米烟幕，16 秒内挡住所有视线"),
    CodexItemBandage => ("BANDAGE — 25 health over a 1.2 s channel; you carry three. Damage or firing breaks it",
                         "绷带 — 引导 1.2 秒回复 25 点；随身三卷，受伤、开火或跑动都会打断"),
    CodexItemMedkit  => ("MEDKIT — 60 health over a 2.5 s channel; you carry one, and damage, firing or running breaks it",
                         "医疗包 — 引导 2.5 秒回复 60 点；只带一个，受伤、开火或跑动都会打断"),
    CodexItemPainkiller => ("PAINKILLER — 35 health over a 1.8 s channel; you carry one. Damage or firing breaks it",
                            "止痛药 — 引导 1.8 秒回复 35 点；只带一剂，受伤、开火或跑动都会打断"),
    CodexItemArmor   => ("ARMOUR PLATE — soaks 65% of every hit until the plates run out; 50 points is a full vest",
                         "防弹插板 — 吸收每发伤害的 65%，直到耗尽；50 点为满护甲"),
    CodexItemAmmo    => ("AMMO — refills the magazine of the weapon in your hands; there is no reserve pool",
                         "弹药箱 — 直接补满手上这把武器的弹匣（本作没有备用弹药池）"),

    // --- field manual: stat labels ---
    // The label column of a card's number line. The values beside them are
    // formatted by `codex`, out of the same tables the simulation reads.
    StatHealth       => ("HEALTH", "生命"),
    StatSpeed        => ("SPEED", "速度"),
    StatDamage       => ("DAMAGE", "伤害"),
    StatRange        => ("RANGE", "射程"),
    StatRate         => ("RATE", "射速"),
    StatMag          => ("MAGAZINE", "弹匣"),
    StatReload       => ("RELOAD", "换弹"),
    StatRecoil       => ("RECOIL", "后坐"),
    StatSpread       => ("SPREAD", "散布"),
    StatPellets      => ("PELLETS", "弹丸"),
    StatModes        => ("MODES", "开火模式"),
    StatClass        => ("CLASS", "类别"),
}

pub fn t(k: Key) -> &'static str {
    k.get(lang())
}
