use embassy_silabs::pac;

/// SYSRTC runs at 32768 Hz after platform init step 6.
const TICKS_PER_MS: u32 = 33;

fn sysrtc_cnt() -> u32 {
    pac::SYSRTC.cnt().read().cnt()
}

/// Blocking delay using the free-running SYSRTC counter (poll only — no WFI;
/// SYSRTC has no periodic IRQ configured in bt-empty).
pub fn delay_ms_blocking(ms: u32) {
    let start = sysrtc_cnt();
    let wait = ms.saturating_mul(TICKS_PER_MS);
    while sysrtc_cnt().wrapping_sub(start) < wait {}
}

pub fn init() {}
