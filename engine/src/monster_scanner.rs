//! 怪物列表扫描：读取游戏 128 元素怪物组件数组，过滤大型怪物，读取各字段。
//!
//! 数据源自 game.exe + 0x0500CF40 → +0x38 → 怪物组件数组（128 个指针）。
//! 每个组件 +0x138 → 怪物数据指针，+0x2A0 → em 字符串用于过滤。
//! 过滤规则：大型怪物（em\emXXX），排除小型（em\emsXXX）。

use std::collections::HashMap;

use process_memory::ProcessHandle;

use crate::game_data::lookup_action_name;
use crate::memory;
use crate::types::{MonsterData, MonsterHp, RadarPosition};

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
/// 朝向四元数起始
const QUAT_OFFSET: u64 = 0x170;
/// 当前动作 ID（actionPtr + 0x61C8, actionId = *actionPtr + 0xB0）
const ACTION_ID_OFFSET: u64 = 0x61C8 + 0xB0;
/// HP 指针
const HP_PTR_OFFSET: u64 = 0x7670;
/// HP 数据偏移（hp_ptr + 0x60 → [max_hp, cur_hp]）
const HP_DATA_OFFSET: u64 = 0x60;
/// 怪物 AI 控制器及其 THK 上下文指针链
const THK_CONTEXT_OFFSETS: [u64; 2] = [0x12278, 0xAA8];
/// THK 上下文持有的怪物组件指针
const THK_OWNER_COMPONENT_OFFSET: u64 = 0x630;
/// 起始于三维距离，下一项为水平距离
const THK_TARGET_DISTANCES_OFFSET: u64 = 0x600;

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
        player_pos: Option<memory::Vector3>,
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
            let action_id =
                memory::read_memory::<i32>(handle, monster_ptr + ACTION_ID_OFFSET).unwrap_or(0);

            // 血量
            let monster_hp = read_monster_hp(handle, monster_ptr);

            // 怪物名称
            let monster_name = monster_names.get(&monster_id).copied();

            let radar_position =
                player_pos.and_then(|pos| read_radar_position(handle, monster_ptr, pos));

            // 招式中文名称查表
            let action_name = lookup_action_name(monster_id, action_id);

            monsters.push(MonsterData {
                monster_id,
                monster_name,
                addr: monster_ptr,
                radar_position,
                monster_hp,
                action_id,
                action_name,
                action_name_en: None,
                is_target: false,
                is_locked_on: false,
                double_list_index,
                counterattack_value: None,
                counterattack_scaled: false,
            });
        }

        monsters
    }
}

/// 读取同一怪物 THK 上下文中的三维和水平目标距离缓存。
pub fn read_ai_target_distances(handle: ProcessHandle, monster_addr: u64) -> Option<[f32; 2]> {
    let context = memory::read_ptr_chain(handle, monster_addr, &THK_CONTEXT_OFFSETS)?;
    let owner = memory::read_ptr_chain(
        handle,
        context,
        &[THK_OWNER_COMPONENT_OFFSET, COMPONENT_TO_MONSTER],
    )?;
    if owner != monster_addr {
        return None;
    }

    let distances = memory::read_memory::<[f32; 2]>(handle, context + THK_TARGET_DISTANCES_OFFSET)?;
    distances
        .iter()
        .all(|distance| distance.is_finite() && *distance >= 0.0)
        .then_some(distances)
}

// ── 内部工具函数 ──

fn read_radar_position(
    handle: ProcessHandle,
    monster_addr: u64,
    player_pos: memory::Vector3,
) -> Option<RadarPosition> {
    let monster_pos = memory::read_entity_position(handle, monster_addr)?;
    let dx = player_pos.x - monster_pos.x;
    let dz = player_pos.z - monster_pos.z;
    let dist_h = dx.hypot(dz);
    let dist_v = player_pos.y - monster_pos.y;
    if !dist_h.is_finite() || !dist_v.is_finite() {
        return None;
    }

    let angle = read_monster_yaw(handle, monster_addr)
        .map(|yaw| (dx.atan2(dz).to_degrees() - yaw).rem_euclid(360.0));
    Some(RadarPosition {
        dist_h,
        dist_v,
        angle,
    })
}

/// 从怪物朝向四元数取得 yaw 角度。
fn read_monster_yaw(handle: ProcessHandle, addr: u64) -> Option<f32> {
    let [qx, qy, qz, qw] = memory::read_memory::<[f32; 4]>(handle, addr + QUAT_OFFSET)?;
    let len_sq = qx * qx + qy * qy + qz * qz + qw * qw;
    if !len_sq.is_finite() || (len_sq - 1.0).abs() > 0.1 {
        return None;
    }
    let yaw_rad = (2.0 * (qw * qy + qx * qz)).atan2(1.0 - 2.0 * (qx * qx + qy * qy));
    Some(yaw_rad.to_degrees())
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
