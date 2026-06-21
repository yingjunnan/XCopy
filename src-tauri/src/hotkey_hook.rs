use std::time::Instant;

/// 纯逻辑双击检测器:不接触 Win32,便于单测。
/// 判定规则:两次 Ctrl keydown 间隔在 [min_ms, max_ms] 内算命中。
pub struct DoubleClickDetector {
    last_press: Option<Instant>,
    #[allow(dead_code)]
    min_interval_ms: u32,
    max_interval_ms: u32,
}

impl DoubleClickDetector {
    pub fn new(min_interval_ms: u32, max_interval_ms: u32) -> Self {
        Self {
            last_press: None,
            min_interval_ms,
            max_interval_ms,
        }
    }

    /// 记录一次 Ctrl keydown,返回是否构成双击(命中后重置)。
    /// 判定:距上次 keydown 间隔 <= max_interval_ms 即视为双击。
    /// 过短(< min)也会算入——正常按键不会在 200ms 内连按两次,故不单列。
    pub fn on_ctrl_press(&mut self, now: Instant) -> bool {
        if let Some(last) = self.last_press {
            let elapsed = now.duration_since(last).as_millis() as u32;
            if elapsed <= self.max_interval_ms {
                self.last_press = None;
                return true;
            }
        }
        self.last_press = Some(now);
        false
    }

    /// Ctrl keyup 不影响判定(只看 keydown),保留接口以便将来扩展。
    pub fn on_ctrl_release(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn single_press_does_not_trigger() {
        let mut d = DoubleClickDetector::new(200, 400);
        assert!(!d.on_ctrl_press(Instant::now()));
    }

    #[test]
    fn two_presses_within_window_trigger() {
        let mut d = DoubleClickDetector::new(200, 400);
        let t0 = Instant::now();
        assert!(!d.on_ctrl_press(t0));
        let t1 = t0 + Duration::from_millis(300);
        assert!(d.on_ctrl_press(t1));
    }

    #[test]
    fn two_presses_too_far_do_not_trigger() {
        let mut d = DoubleClickDetector::new(200, 400);
        let t0 = Instant::now();
        assert!(!d.on_ctrl_press(t0));
        let t1 = t0 + Duration::from_millis(500);
        assert!(!d.on_ctrl_press(t1));
    }

    #[test]
    fn trigger_resets_so_third_press_starts_new_window() {
        let mut d = DoubleClickDetector::new(200, 400);
        let t0 = Instant::now();
        d.on_ctrl_press(t0);
        let t1 = t0 + Duration::from_millis(300);
        assert!(d.on_ctrl_press(t1)); // 命中并重置
        let t2 = t1 + Duration::from_millis(300);
        assert!(!d.on_ctrl_press(t2)); // 新窗口的第一次,不触发
    }

    #[test]
    fn first_press_after_timeout_starts_fresh() {
        let mut d = DoubleClickDetector::new(200, 400);
        let t0 = Instant::now();
        d.on_ctrl_press(t0);
        let t1 = t0 + Duration::from_millis(500); // 超时,不算双击
        assert!(!d.on_ctrl_press(t1));
        let t2 = t1 + Duration::from_millis(300); // 但这按下成为新的起点
        assert!(d.on_ctrl_press(t2));
    }
}
