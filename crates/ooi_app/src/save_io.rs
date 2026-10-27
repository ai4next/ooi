//! 存档落盘，交给 `oo_kit::save::SaveSlot`（原子写 + 一份备份）。
//!
//! `OOI_FRESH` 时既不读也不写，截图与调试不碰玩家的存档；测试同理。

use oo_kit::save::SaveSlot;
use ooi_core::SaveData;

fn slot() -> Option<SaveSlot> {
    let off = cfg!(test) || std::env::var_os("OOI_FRESH").is_some();
    (!off).then(|| SaveSlot::new("ooi", "OriginOfIntelligence"))
}

pub fn load() -> Option<SaveData> {
    slot()?.load(SaveData::from_json)
}

pub fn save(data: &SaveData) {
    if let Some(s) = slot()
        && let Err(e) = s.write(&data.to_json())
    {
        eprintln!("save failed: {e}");
    }
}
