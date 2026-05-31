//! 目标选择算法：从当前区域所有大型怪物中选出应显示的目标。
//!
//! 优先级：
//!   1. 玩家游戏内 R3 锁定的怪物（通过链表索引匹配）
//!   2. 距离最近的怪物
//!   3. 距离相同时，血量百分比低的优先
//!   4. 距离和血量百分比都相同，确定性随机

use crate::types::MonsterData;

/// 目标选择上下文
pub struct TargetContext {
    /// 游戏内当前锁定的怪物链表索引。`-1` = 无锁定。
    pub locked_on_index: i32,
}

/// 目标选择器
pub struct TargetSelector;

impl TargetSelector {
    pub fn new() -> Self {
        Self
    }

    /// 从怪物列表中选出当前应显示的目标。
    ///
    /// 返回 `Some(monsters 中的索引)` 或 `None`（列表为空时）。
    pub fn select_target(
        &mut self,
        monsters: &[MonsterData],
        context: &TargetContext,
    ) -> Option<usize> {
        if monsters.is_empty() {
            return None;
        }
        if monsters.len() == 1 {
            return Some(0);
        }

        // 1. 检查游戏内锁定目标（R3 锁定）
        if context.locked_on_index >= 0 {
            if let Some(idx) = monsters
                .iter()
                .position(|m| m.double_list_index == context.locked_on_index)
            {
                return Some(idx);
            }
        }

        // 2. 自动选择：距离 → HP% → 随机
        let mut indices: Vec<usize> = (0..monsters.len()).collect();

        // 距离容差：差 < 500 单位视为"同样接近"
        const DISTANCE_TOLERANCE: f32 = 500.0;

        indices.sort_by(|&a, &b| {
            let da = monsters[a].dist_h;
            let db = monsters[b].dist_h;

            if (da - db).abs() < DISTANCE_TOLERANCE {
                // 比较 HP%
                let hp_a = monsters[a]
                    .monster_hp
                    .map(|h| h.current / h.max)
                    .unwrap_or(1.0);
                let hp_b = monsters[b]
                    .monster_hp
                    .map(|h| h.current / h.max)
                    .unwrap_or(1.0);
                hp_a.partial_cmp(&hp_b).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            }
        });

        // 3. 检查前两名是否距离+HP%均接近（触发随机分支）
        if monsters.len() >= 2 {
            let a = indices[0];
            let b = indices[1];
            let same_dist = (monsters[a].dist_h - monsters[b].dist_h).abs() < DISTANCE_TOLERANCE;
            let same_hp = if same_dist {
                let hp_a = monsters[a]
                    .monster_hp
                    .map(|h| h.current / h.max)
                    .unwrap_or(1.0);
                let hp_b = monsters[b]
                    .monster_hp
                    .map(|h| h.current / h.max)
                    .unwrap_or(1.0);
                (hp_a - hp_b).abs() < 0.01
            } else {
                false
            };

            if same_dist && same_hp {
                // 确定性随机：基于 monster_id 做哈希，保证同任务内稳定
                let hash = (monsters[a].monster_id as u64).wrapping_mul(2_654_435_761);
                let pick = hash as usize % 2;
                return Some(if pick == 0 { a } else { b });
            }
        }

        Some(indices[0])
    }
}
