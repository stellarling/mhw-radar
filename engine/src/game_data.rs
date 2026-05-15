//! 游戏数据聚合：招式映射 + 怪物名称
//!
//! 数据以 JSON 文件维护在 data/ 下，编译时通过 include_str! 嵌入二进制。
//! 招式查表由 LazyLock 启动时一次解析，后续 O(1) HashMap 查询。

use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(serde::Deserialize)]
struct RawActionEntry {
    action_id: i32,
    name: String,
}

#[derive(serde::Deserialize)]
struct RawMonsterEntry {
    id: i32,
    name: String,
}

// ── 招式名称查询 ──────────────────────────────────────────────

static ACTION_LOOKUP: LazyLock<HashMap<(i32, i32), &'static str>> = LazyLock::new(|| {
    fn load(monster_id: i32, json: &str) -> HashMap<(i32, i32), &'static str> {
        let entries: Vec<RawActionEntry> =
            serde_json::from_str(json).expect("invalid action data JSON");
        entries
            .into_iter()
            .map(|e| {
                let name: &'static str = Box::leak(e.name.into_boxed_str());
                ((monster_id, e.action_id), name)
            })
            .collect()
    }

    let mut map = HashMap::new();
    map.extend(load(92, include_str!("data/monster_092.json")));
    map.extend(load(101, include_str!("data/monster_101.json")));
    map
});

pub fn lookup_action_name(monster_id: i32, action_id: i32) -> Option<&'static str> {
    ACTION_LOOKUP.get(&(monster_id, action_id)).copied()
}

// ── 怪物名称加载 ──────────────────────────────────────────────

pub fn load_monster_names() -> HashMap<i32, &'static str> {
    let entries: Vec<RawMonsterEntry> =
        serde_json::from_str(include_str!("data/monster_names.json"))
            .expect("invalid monster_names.json");
    entries
        .into_iter()
        .map(|e| {
            let name: &'static str = Box::leak(e.name.into_boxed_str());
            (e.id, name)
        })
        .collect()
}

// ── 任务名称加载 ──────────────────────────────────────────────

pub fn load_quest_names() -> HashMap<i32, &'static str> {
    let map: HashMap<i32, String> =
        serde_json::from_str(include_str!("data/quest_names.json"))
            .expect("invalid quest_names.json");
    map.into_iter()
        .map(|(id, name)| {
            let leaked: &'static str = Box::leak(name.into_boxed_str());
            (id, leaked)
        })
        .collect()
}
