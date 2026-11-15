/// Fractional width target, independent of the native window's integer pixel size.
pub struct WidthMotion {
    pub current: f32,
    target: f32,
}
impl WidthMotion {
    pub fn new(width: f32) -> Self {
        Self {
            current: width,
            target: width,
        }
    }
    pub fn scroll(&mut self, delta: f32) {
        self.target = (self.target + delta * 0.35).max(10.);
    }
    pub fn advance(&mut self, seconds: f32) -> bool {
        self.current +=
            (self.target - self.current) * (1. - (-seconds.clamp(0., 0.1) / 0.075).exp());
        if (self.target - self.current).abs() < 0.01 {
            self.current = self.target;
            true
        } else {
            false
        }
    }
}
#[cfg(test)]
mod tests {
    use super::WidthMotion;
    #[test]
    fn scroll_accumulates_fractional_width_and_reverses() {
        let mut motion = WidthMotion::new(500.);
        for _ in 0..10 {
            motion.scroll(0.1);
        }
        assert!((motion.target - 500.35).abs() < 0.001);
        motion.advance(1. / 120.);
        assert!(motion.current > 500. && motion.current < 500.35);
        motion.scroll(-8.);
        let previous = motion.current;
        motion.advance(1. / 120.);
        assert!(motion.current < previous);
    }
    #[test]
    fn motion_converges_without_overshoot_and_respects_minimum() {
        let mut motion = WidthMotion::new(500.);
        motion.scroll(120.);
        let mut settled = false;
        for _ in 0..240 {
            let previous = motion.current;
            settled = motion.advance(1. / 120.);
            assert!(motion.current >= previous && motion.current <= 542.);
            if settled {
                break;
            }
        }
        assert!(settled);
        assert_eq!(motion.current, 542.);
        motion.scroll(-10000.);
        for _ in 0..240 {
            motion.advance(1. / 120.);
        }
        assert_eq!(motion.current, 10.);
    }
}
