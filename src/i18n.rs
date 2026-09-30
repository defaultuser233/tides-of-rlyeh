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
}

pub fn t(k: Key) -> &'static str {
    k.get(lang())
}
