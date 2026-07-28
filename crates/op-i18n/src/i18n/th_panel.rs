//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `th_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "ค้นหารูปภาพ…",
        "imagePanel.searching" => "กำลังค้นหา…",
        "imagePanel.noResults" => "ไม่พบผลลัพธ์",
        "imagePanel.searchPrompt" => "ค้นหารูปภาพ",
        "imagePanel.sourceNotice" => {
            "รูปภาพจาก {{source}} ใบอนุญาตแบบเสรี — โปรดตรวจสอบใบอนุญาตก่อนใช้งาน"
        }
        "imagePanel.genNotConfigured" => "ยังไม่ได้ตั้งค่าการสร้างรูปภาพ",
        "imagePanel.openSettings" => "เปิดการตั้งค่า",
        "imagePanel.promptPlaceholder" => "อธิบายรูปภาพ…",
        "providerProbe.connectedViaCli" => "เชื่อมต่อผ่าน {{name}} CLI แล้ว",
        "providerProbe.cliExitedWithError" => "{{name}} CLI ปิดการทำงานพร้อมข้อผิดพลาด",
        "providerProbe.cliNoVersionOutput" => "{{name}} CLI ไม่ได้แสดงข้อมูลเวอร์ชัน",
        "providerProbe.modelQueryFailed" => "การขอรายการโมเดลของ {{name}} ล้มเหลวหรือหมดเวลา",
        "providerProbe.modelQueryFailedRunLogin" => {
            "การขอรายการโมเดลของ {{name}} ล้มเหลว เรียกใช้ {{command}} หนึ่งครั้งเพื่อยืนยันตัวตน"
        }
        "providerProbe.modelQueryNeedsAuth" => {
            "การขอรายการโมเดลของ {{name}} ต้องยืนยันตัวตน เรียกใช้ {{command}} หนึ่งครั้งเพื่อลงชื่อเข้าใช้"
        }
        "providerProbe.unrecognizedModelCatalog" => "{{name}} ส่งคืนรายการโมเดลที่ไม่รู้จัก",
        _ => return super::th_collab::lookup(key),
    })
}
