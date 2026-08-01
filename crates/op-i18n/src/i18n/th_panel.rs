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
        "promptCenter.title" => "ศูนย์พรอมป์",
        "promptCenter.searchPlaceholder" => "ค้นหาพรอมป์…",
        "promptCenter.category.all" => "ทั้งหมด",
        "promptCenter.category.starter" => "เริ่มต้นอย่างรวดเร็ว",
        "promptCenter.category.mobileApp" => "แอปมือถือ",
        "promptCenter.category.webPage" => "หน้าเว็บ",
        "promptCenter.category.dashboard" => "แดชบอร์ด",
        "promptCenter.category.component" => "คอมโพเนนต์",
        "promptCenter.category.modify" => "ปรับแก้",
        "promptCenter.category.custom" => "ของฉัน",
        "promptCenter.empty" => "ไม่พบพรอมป์ที่ตรงกัน",
        "promptCenter.saveCurrent" => "บันทึกข้อความปัจจุบันเป็นพรอมป์",
        "promptCenter.saveTitlePlaceholder" => "ใส่ชื่อพรอมป์",
        "promptCenter.save" => "บันทึก",
        "promptCenter.cancel" => "ยกเลิก",
        "promptCenter.delete" => "ลบ",
        "promptCenter.screens" => "{{count}} หน้าจอ",
        "promptCenter.freeform" => "อิสระ",
        "promptCenter.item.wander.title" => "Wander · วางแผนการเดินทาง",
        "promptCenter.item.forage.title" => "Forage · สูตรอาหารตามฤดูกาล",
        "promptCenter.item.still.title" => "Still · สมาธิและการนอน",
        "promptCenter.item.hearth.title" => "Hearth · บ้านอัจฉริยะ",
        "promptCenter.item.meteo.title" => "Meteo · สภาพอากาศแบบเต็มอารมณ์",
        "promptCenter.item.marginalia.title" => "Marginalia · อ่านและจดบันทึก",
        "promptCenter.item.lingua.title" => "Lingua · เรียนภาษา",
        "promptCenter.item.daybreak.title" => "Daybreak · สั่งกาแฟ",
        "promptCenter.item.verdant.title" => "Verdant · ดูแลต้นไม้",
        "promptCenter.item.companion.title" => "Companion · ชีวิตสัตว์เลี้ยง",
        "promptCenter.item.relic.title" => "Relic · ตลาดสินค้ามือสองคัดสรร",
        "promptCenter.item.nocturne.title" => "Nocturne · คู่มือดูดาว",
        "promptCenter.item.marquee.title" => "Marquee · รายการภาพยนตร์",
        "promptCenter.item.ritual.title" => "Ritual · สร้างนิสัย",
        "promptCenter.item.ember.title" => "Ember · บันทึกอารมณ์",
        "promptCenter.item.volt.title" => "Volt · ผู้ช่วยรถยนต์ไฟฟ้า",
        "promptCenter.item.aloft.title" => "Aloft · ติดตามเที่ยวบิน",
        "promptCenter.item.gallery.title" => "Gallery · นิทรรศการและวัฒนธรรม",
        "promptCenter.item.nightcap.title" => "Nightcap · ผสมเครื่องดื่มที่บ้าน",
        "promptCenter.item.bloom.title" => "Bloom · บันทึกการเติบโตของลูก",
        "promptCenter.item.extremeWeather.title" => "แอปสภาพอากาศ · ทำให้ฉันทึ่ง",
        "promptCenter.item.extremeNowPlaying.title" => "กำลังเล่น · สวยพร้อมเผยแพร่",
        "promptCenter.item.extremeDailyApp.title" => "แอปที่อยากเปิดทุกวัน",
        "promptCenter.item.extremeCalendar.title" => "ออกแบบปฏิทินขึ้นใหม่",
        "promptCenter.item.extremeCalm.title" => "ความสงบในหนึ่งหน้าจอ",
        "promptCenter.item.webOrbit.title" => "Orbit · หน้าแลนดิ้งเวิร์กเบนช์ AI",
        "promptCenter.item.webAtelier.title" => "Atelier · อีคอมเมิร์ซเฟอร์นิเจอร์",
        "promptCenter.item.dashboardPulse.title" => "Pulse · แดชบอร์ดวิเคราะห์การเติบโต",
        "promptCenter.item.dashboardSentinel.title" => "Sentinel · ปฏิบัติการโลจิสติกส์",
        "promptCenter.item.componentDataGrid.title" => "Gridworks · ตารางข้อมูลองค์กร",
        "promptCenter.item.componentFormLab.title" => "Form Lab · ระบบคอมโพเนนต์ฟอร์ม",
        "promptCenter.item.modifyPolishCurrent.title" => "ปรับแต่งหน้าจอปัจจุบัน",
        "promptCenter.item.modifyCompleteStates.title" => "เติมสถานะคอมโพเนนต์ให้ครบ",
        "sceneTemplate.title" => "เทมเพลตฉาก",
        "sceneTemplate.searchPlaceholder" => "ค้นหาฉากหรือเทมเพลต…",
        "sceneTemplate.empty" => "ไม่พบเทมเพลตที่ตรงกัน",
        "sceneTemplate.frames" => "{{count}} หน้า",
        "sceneTemplate.filter.all" => "ทั้งหมด",
        "sceneTemplate.scene.tutorial" => "ภาพสอนใช้งาน",
        "sceneTemplate.scene.comparison" => "ภาพเปรียบเทียบ",
        "sceneTemplate.scene.carousel" => "การ์ดความรู้",
        "sceneTemplate.scene.slides" => "PPT",
        "sceneTemplate.item.screenshotTutorial.title" => {
            "การ์ดสอนใช้งานด้วยภาพหน้าจอ 3 ขั้นตอน"
        }
        "sceneTemplate.item.screenshotTutorial.summary" => {
            "ประกอบด้วยหน้าปก ขั้นตอนการใช้งาน 3 ขั้น และคำกระตุ้นการตัดสินใจตอนท้าย เพียงเปลี่ยนภาพหน้าจอและคำอธิบายก็พร้อมเผยแพร่"
        }
        "sceneTemplate.item.knowledgeCarousel.title" => "คารูเซลความรู้และมุมมอง",
        "sceneTemplate.item.knowledgeCarousel.summary" => {
            "ประกอบด้วยหน้าปก ประเด็นหลัก 3 ข้อ และหน้าสรุป เหมาะสำหรับแยกหนึ่งแนวคิดเป็นการ์ดต่อเนื่องที่ปัดดูได้"
        }
        "sceneTemplate.item.beforeAfter.title" => "เปรียบเทียบก่อนและหลังปรับดีไซน์",
        "sceneTemplate.item.beforeAfter.summary" => {
            "วางภาพก่อนและหลังเทียบกันซ้ายขวา พร้อมคำอธิบายการเปลี่ยนแปลง เหมาะสำหรับการสรุปบทเรียนและนำเสนอในพอร์ตโฟลิโอ"
        }
        "sceneTemplate.item.slideDeck.title" => "งานนำเสนอ · 6 สไลด์",
        "sceneTemplate.item.slideDeck.summary" => {
            "ประกอบด้วยหน้าปก สารบัญ ประเด็นสำคัญ ข้อมูล แผนภูมิ และหน้าปิด ในอัตราส่วนฉายภาพ 16:9 เพียงเปลี่ยนข้อความก็พร้อมนำเสนอ"
        }
        "fileMenu.newFromTemplate" => "สร้างใหม่จากเทมเพลต",
        "collab.ownerConfirm.title" => "ยืนยันว่าคุณกำลังเข้าร่วมกับใคร",
        "collab.ownerConfirm.hint" => "ยังไม่มีเนื้อหาใดจากเซสชันนี้ถูกโหลด",
        "collab.ownerConfirm.account" => "บัญชีที่ยืนยันแล้ว",
        "collab.ownerConfirm.device" => "อุปกรณ์ที่ยืนยันแล้ว",
        "collab.ownerConfirm.claimedName" => "ชื่อที่บัญชีนี้ตั้งเอง (ยังไม่ยืนยัน)",
        "collab.action.confirmOwner" => "เข้าร่วมเซสชันนี้",
        "collab.action.rejectOwner" => "ไม่เข้าร่วม",
        "collab.error.ownerNotConfirmed" => "คุณไม่ได้ยืนยันผู้จัด จึงไม่มีการโหลดข้อมูลใด",
        _ => return super::th_collab::lookup(key),
    })
}
