use core::cell::{Cell, RefCell};
use core::sync::atomic::{AtomicU32, Ordering, compiler_fence};

use critical_section::CriticalSection;
use embassy_sync::blocking_mutex::CriticalSectionMutex as Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_time_driver::Driver;
use embassy_time_queue_utils::Queue;
use silabs_pac::sysrtc0_ns::regs::Grp0Ctrl;

use crate::interrupt::InterruptExt;
use crate::{interrupt, pac};

fn rtc() -> pac::sysrtc::ns {
    pac::SYSRTC
}

/// Calculate the timestamp from the period count and the tick count.
///
/// The RTC counter is 32 bit. Ticking at 32768hz, it overflows every 36 hours, 4 minutes and 32 seconds.
/// This is still too short, so we count overflow periods.
///
/// We define a "period" to be 2^31 ticks (instead of 2^32). One "overflow cycle" is 2 periods.
///
/// - `period` is incremented on overflow (at counter value 0)
/// - `period` is incremented "midway" between overflows (at counter value 0x8000_0000)
///
/// Therefore, when `period` is even, counter is in 0..0x7fff_ffff. When odd, counter is in 0x8000_0000..0xFFFF_FFFF
/// This allows for now() to return the correct value even if it races an overflow.
///
/// To get `now()`, `period` is read first, then `counter` is read. If the counter value matches
/// the expected range for the `period` parity, we're done. If it doesn't, this means that
/// a new period start has raced us between reading `period` and `counter`, so we assume the `counter` value
/// corresponds to the next period.
///
/// `period` is a 32bit integer, so It overflows on 2^32 * 2^31 / 32768 seconds of uptime.
/// 8.9 billion years might actually be a bit unnecessary.
fn calc_now(period: u32, counter: u32) -> u64 {
    ((period as u64) << 31) + ((counter ^ ((period & 1) << 31)) as u64)
}

struct AlarmState {
    timestamp: Cell<u64>,
}

unsafe impl Send for AlarmState {}

impl AlarmState {
    const fn new() -> Self {
        Self {
            timestamp: Cell::new(u64::MAX),
        }
    }
}

struct RtcDriver {
    /// Number of 2^23 periods elapsed since boot.
    period: AtomicU32,
    /// Timestamp at which to fire alarm. u64::MAX if no alarm is scheduled.
    alarms: Mutex<AlarmState>,
    queue: Mutex<RefCell<Queue>>,
}

embassy_time_driver::time_driver_impl!(static DRIVER: RtcDriver = RtcDriver {
    period: AtomicU32::new(0),
    alarms: Mutex::const_new(CriticalSectionRawMutex::new(), AlarmState::new()),
    queue: Mutex::new(RefCell::new(Queue::new())),
});

impl RtcDriver {
    fn init(&'static self, irq_prio: crate::interrupt::Priority) {
        let r = rtc();
        // Enable clocks
        pac::CMU.clken0().modify(|w| {
            w.set_sysrtc0(true);
            w.set_lfrco(true);
        });

        // RTC will use 32.768 kHz LFRCO clock
        // "The LFRCO has a fast start-up time (refer to the data sheet electrical specifications for the exact
        // start-up time). When the oscillator has started up and is ready to use, the RDY status bit
        // will go high and the RDY interrupt will be triggered. After start-up, it may take two
        // clock cycles for the clock to propagate through the CMU to the peripherals.

        // Start LFRCO
        pac::LFRCO.ctrl().write(|w| w.set_forceen(true));

        // Wait for LFRCO to be ready
        while !pac::LFRCO.status().read().rdy() {}

        // "The normal operation of SYSRTC is to configure it, enable it, start it, and then leave it running"
        // Config
        // Select LFRCO as the clock source
        pac::CMU
            .sysrtc0clkctrl()
            .write(|w| w.set_clksel(pac::cmu_ns::vals::Sysrtc0clkctrlClksel::LFRCO));
        // Set a compare to 0x8000_0000
        r.grp0_cmp0value().write(|w| w.set_cmp0value(0x8000_0000));
        // Enable the comparison and overflow interrupts
        r.grp0_ien().write(|w| {
            w.set_ovf(true);
            w.set_cmp0(true)
        });
        // Enable compares once and leave them running
        r.grp0_ctrl().write_value(Grp0Ctrl(0b11));
        r.grp0_ctrl().write(|w| {
            w.set_cmp0en(true);
            w.set_cmp1en(true)
        });

        // Enable RTC clock
        //if !r.en().read().en() {
        r.en().write(|w| w.set_en(true));
        //}

        // Start RTC
        r.cmd().write(|w| w.set_start(true));

        interrupt::SYSRTC_APP.set_priority(irq_prio);
        unsafe { interrupt::SYSRTC_APP.enable() };
    }

    fn on_interrupt(&self) {
        let r = rtc();
        // Overflow
        if r.grp0_if().read().ovf() {
            r.grp0_if().write(|w| w.set_ovf(false));
            self.next_period();
        }
        // Compare 0
        if r.grp0_if().read().cmp0() {
            r.grp0_if().write(|w| w.set_cmp0(false));
            self.next_period();
        }

        // Compare 1
        if r.grp0_if().read().cmp1() {
            r.grp0_if().write(|w| w.set_cmp1(false));
            critical_section::with(|cs| {
                self.trigger_alarm(cs);
            });
        }
    }

    fn next_period(&self) {
        critical_section::with(|cs| {
            let r = rtc();
            let period = self.period.load(Ordering::Relaxed) + 1;
            self.period.store(period, Ordering::Relaxed);
            let t = (period as u64) << 31;

            let alarm = &self.alarms.borrow(cs);
            let at = alarm.timestamp.get();

            if at < t + 0xc000_0000 {
                // just enable it. `set_alarm` has already set the correct CC val.
                r.grp0_ien().write(|w| w.set_cmp1(true));
            }
        })
    }

    fn trigger_alarm(&self, cs: CriticalSection) {
        let r = rtc();
        r.grp0_ien().write(|w| w.set_cmp1(true));

        let alarm = &self.alarms.borrow(cs);
        alarm.timestamp.set(u64::MAX);

        // Call after clearing alarm, so the callback can set another alarm.
        let mut next = self
            .queue
            .borrow(cs)
            .borrow_mut()
            .next_expiration(self.now());
        while !self.set_alarm(cs, next) {
            next = self
                .queue
                .borrow(cs)
                .borrow_mut()
                .next_expiration(self.now());
        }
    }

    fn set_alarm(&self, cs: CriticalSection, timestamp: u64) -> bool {
        let alarm = &self.alarms.borrow(cs);
        alarm.timestamp.set(timestamp);

        let r = rtc();

        loop {
            let t = self.now();
            if timestamp <= t {
                // If alarm timestamp has passed the alarm will not fire.
                // Disarm the alarm and return `false` to indicate that.
                r.grp0_ien().write(|w| w.set_cmp1(false));

                alarm.timestamp.set(u64::MAX);

                return false;
            }

            // If it hasn't triggered yet, setup it in the compare channel.

            // Write the CC value regardless of whether we're going to enable it now or not.
            // This way, when we enable it later, the right value is already set.

            // "Note that when setting the compare value to the current counter value, a compare event
            // may not get generated until the counter overflows and reaches the current value again.
            // To generate a compare event quickly, it is recommended to program the compare value to
            // the current counter value + 1. "

            // So, let a safe timestamp be +2
            let safe_timestamp = timestamp.max(t + 2);
            r.grp0_cmp1value()
                .write(|w| w.set_cmp1value(safe_timestamp as u32));

            let diff = timestamp - t;
            if diff < 0xc000_0000 {
                r.grp0_ien().write(|w| w.set_cmp1(true));

                // If we have not passed the timestamp, we can be sure the alarm will be invoked. Otherwise,
                // we need to retry setting the alarm.
                if self.now() + 2 <= timestamp {
                    return true;
                }
            } else {
                // If it's too far in the future, don't setup the compare channel yet.
                // It will be setup later by `next_period`.
                r.grp0_ien().write(|w| w.set_cmp1(false));
                return true;
            }
        }
    }
}

impl Driver for RtcDriver {
    fn now(&self) -> u64 {
        // `period` MUST be read before `counter`, see comment at the top for details.
        let period = self.period.load(Ordering::Relaxed);
        compiler_fence(Ordering::Acquire);
        let counter = rtc().cnt().read().cnt();
        calc_now(period, counter)
    }

    fn schedule_wake(&self, at: u64, waker: &core::task::Waker) {
        critical_section::with(|cs| {
            let mut queue = self.queue.borrow(cs).borrow_mut();

            if queue.schedule_wake(at, waker) {
                let mut next = queue.next_expiration(self.now());
                while !self.set_alarm(cs, next) {
                    next = queue.next_expiration(self.now());
                }
            }
        })
    }
}

#[cfg(feature = "rt")]
#[interrupt]
fn SYSRTC_APP() {
    DRIVER.on_interrupt()
}

pub(crate) fn init(irq_prio: crate::interrupt::Priority) {
    DRIVER.init(irq_prio)
}
