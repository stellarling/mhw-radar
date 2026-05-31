//! 怪物列表扫描：读取游戏 128 元素怪物组件数组，过滤大型怪物，读取各字段。
//!
//! 数据源自 game.exe + 0x0500CF40 → +0x38 → 怪物组件数组（128 个指针）。
//! 每个组件 +0x138 → 怪物数据指针，+0x2A0 → em 字符串用于过滤。
//! 过滤规则：大型怪物（em\emXXX），排除小型（em\emsXXX）。

use std::collections::HashMap;

use process_memory::ProcessHandle;

use crate::game_data::lookup_action_name;
use crate::memory;
use crate::types::{MonsterData, MonsterHp};

// ── 内存偏移常量 ──

/// 怪物组件数组基址（相对于 game base）
const COMPONENT_ARRAY_BASE: u64 = 0x0500CF40;
/// 解引用得到数组指针的偏移链
const COMPONENT_ARRAY_OFFSETS: [u64; 1] = [0x38];
/// 固定的组件数组大小
const COMPONENT_ARRAY_COUNT: usize = 128;
/// 组件 → 怪物数据指针的偏移
const COMPONENT_TO_MONSTER: u64 = 0x138;
/// 怪物数据上的 em 字符串偏移
const EM_STRING_OFFSET: u64 = 0x2A0;
const EM_STRING_MAX_LEN: usize = 64;
/// 怪物 ID
const MONSTER_ID_OFFSET: u64 = 0x12280;
/// 怪物在双向链表中的索引（用于锁定检测）
const DOUBLE_LIST_INDEX_OFFSET: u64 = 0x1228C;
/// 位置 Vector3
const POSITION_OFFSET: u64 = 0x160;
/// 朝向四元数起始
const QUAT_OFFSET: u64 = 0x170;
/// 当前动作 ID（actionPtr + 0x61C8, actionId = *actionPtr + 0xB0）
const ACTION_ID_OFFSET: u64 = 0x61C8 + 0xB0;
/// HP 指针
const HP_PTR_OFFSET: u64 = 0x7670;
/// HP 数据偏移（hp_ptr + 0x60 → [max_hp, cur_hp]）
const HP_DATA_OFFSET: u64 = 0x60;

pub struct MonsterScanner;

impl MonsterScanner {
    pub fn new() -> Self {
        Self
    }

    /// 扫描当前区域所有大型怪物。
    ///
    /// 1. 从组件数组读取 128 个组件指针
    /// 2. 解析每个组件的怪物数据指针，读取 em 字符串
    /// 3. 过滤大型怪物（em\em 开头、非 em\ems）
    /// 4. 读取各怪物字段（ID、位置、血量、动作等）
    /// 5. 计算相对玩家的距离和角度
    /// 6. 返回 `Vec<MonsterData>`
    pub fn scan_all_monsters(
        &self,
        handle: ProcessHandle,
        game_base: u64,
        player_pos: memory::Vector3,
        monster_names: &HashMap<i32, &'static str>,
    ) -> Vec<MonsterData> {
        // 1. 获取组件数组指针
        let array_ptr = match memory::resolve_pointer(
            handle,
            game_base + COMPONENT_ARRAY_BASE,
            &COMPONENT_ARRAY_OFFSETS,
        ) {
            Some(ptr) if ptr != 0 => ptr,
            _ => return Vec::new(),
        };

        // 2. 读取 128 个组件指针
        let components = match memory::read_array_u64(handle, array_ptr, COMPONENT_ARRAY_COUNT) {
            Some(c) => c,
            None => return Vec::new(),
        };

        let mut monsters = Vec::with_capacity(components.len());

        for &comp_ptr in &components {
            if comp_ptr == 0 {
                continue;
            }

            // 3. 组件 → 怪物数据指针
            let monster_ptr = match memory::read_memory::<u64>(handle, comp_ptr + COMPONENT_TO_MONSTER)
            {
                Some(ptr) if ptr != 0 => ptr,
                _ => continue,
            };

            // 4. em 字符串过滤（offset 0x2A0 存的是指针，指向的 +0x0C 才是字符串）
            let em = memory::read_memory::<u64>(handle, monster_ptr + EM_STRING_OFFSET)
                .and_then(|ptr| memory::read_string(handle, ptr + 0x0C, EM_STRING_MAX_LEN));
            let is_large = em.as_ref().map_or(false, |s| s.starts_with("em\\em") && !s.starts_with("em\\ems"));
            if !is_large {
                continue;
            }

            // 5. 读取怪物字段
            let monster_id = memory::read_memory::<i32>(handle, monster_ptr + MONSTER_ID_OFFSET)
                .unwrap_or(-1);
            let double_list_index =
                memory::read_memory::<i32>(handle, monster_ptr + DOUBLE_LIST_INDEX_OFFSET)
                    .unwrap_or(-1);
            let monster_pos =
                memory::read_memory::<memory::Vector3>(handle, monster_ptr + POSITION_OFFSET)
                    .unwrap_or_default();
            let action_id =
                memory::read_memory::<i32>(handle, monster_ptr + ACTION_ID_OFFSET).unwrap_or(0);

            // 血量
            let monster_hp = read_monster_hp(handle, monster_ptr);

            // 怪物名称
            let monster_name = monster_names.get(&monster_id).copied();

            // 距离/角度计算
            let dx = player_pos.x - monster_pos.x;
            let dz = player_pos.z - monster_pos.z;
            let dist_h = (dx * dx + dz * dz).sqrt();
            let dist_v = (player_pos.y - monster_pos.y).abs();
            let angle = compute_angle_from_player(dx, dz, handle, monster_ptr);

            // 招式中文名称查表
            let action_name = lookup_action_name(monster_id, action_id);

            monsters.push(MonsterData {
                monster_id,
                monster_name,
                addr: monster_ptr,
                dist_h,
                dist_v,
                angle,
                monster_hp,
                action_id,
                action_name,
                action_name_en: None,
                is_target: false,
                is_locked_on: false,
                double_list_index,
                counterattack_value: None,
                counterattack_scaled: false,
                ai_dist: None,
                ai_angle: None,
            });
        }

        monsters
    }
}

// ── 内部工具函数 ──

/// 从怪物的四元数计算 yaw，再结合玩家方位计算角度
fn compute_angle_from_player(
    dx: f32,
    dz: f32,
    handle: ProcessHandle,
    monster_addr: u64,
) -> f32 {
    let monster_yaw = read_monster_yaw(handle, monster_addr);
    let dir = dx.atan2(dz) * (180.0 / std::f32::consts::PI);
    let dir = if dir < 0.0 { dir + 360.0 } else { dir };
    (dir - monster_yaw + 360.0) % 360.0
}

/// 从怪物内存读取朝向（四元数 → yaw 角度）
fn read_monster_yaw(handle: ProcessHandle, addr: u64) -> f32 {
    let (qx, qy, qz, qw) = match (
        memory::read_memory::<f32>(handle, addr + QUAT_OFFSET),
        memory::read_memory::<f32>(handle, addr + QUAT_OFFSET + 4),
        memory::read_memory::<f32>(handle, addr + QUAT_OFFSET + 8),
        memory::read_memory::<f32>(handle, addr + QUAT_OFFSET + 12),
    ) {
        (Some(x), Some(y), Some(z), Some(w)) => (x, y, z, w),
        _ => return 0.0,
    };
    let len_sq = qx * qx + qy * qy + qz * qz + qw * qw;
    if (len_sq - 1.0).abs() > 0.1 {
        return 0.0;
    }
    let yaw_rad = (2.0 * (qw * qy + qx * qz)).atan2(1.0 - 2.0 * (qx * qx + qy * qy));
    let mut deg = yaw_rad * (180.0 / std::f32::consts::PI);
    if deg < 0.0 {
        deg += 360.0;
    }
    deg
}

/// 读取怪物血量
fn read_monster_hp(handle: ProcessHandle, addr: u64) -> Option<MonsterHp> {
    let hp_ptr = memory::read_memory::<u64>(handle, addr + HP_PTR_OFFSET)?;
    if hp_ptr == 0 {
        return None;
    }
    let [max_hp, cur_hp] = memory::read_memory::<[f32; 2]>(handle, hp_ptr + HP_DATA_OFFSET)?;
    if max_hp > 0.0 {
        Some(MonsterHp {
            current: cur_hp,
            max: max_hp,
        })
    } else {
        None
    }
}
