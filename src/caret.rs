use gpui::*;
use std::time::Instant;

struct Motion {
    start: Point<Pixels>,
    current: Point<Pixels>,
    target: Point<Pixels>,
    since: Instant,
}
#[derive(Default)]
pub struct Carets {
    motions: Vec<Motion>,
}
fn progress(style: &str, time: f32) -> f32 {
    let t = time.clamp(0., 1.);
    match style {
        "off" => 1.,
        "linear" => t,
        "spring" => {
            if t >= 1. {
                1.
            } else {
                1. - (-7. * t).exp() * (10. * t).cos()
            }
        }
        "neovide" => 1. - (1. - t).powi(4),
        "trail" => 1. - (1. - t).powi(3),
        _ => t * t * (3. - 2. * t),
    }
}
fn interpolate(a: Point<Pixels>, b: Point<Pixels>, t: f32) -> Point<Pixels> {
    point(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}
impl Carets {
    pub fn paint(
        &mut self,
        targets: &[Point<Pixels>],
        style: &str,
        duration: f32,
        height: f32,
        color: u32,
        window: &mut Window,
    ) {
        let now = Instant::now();
        if self.motions.len() != targets.len() {
            self.motions = targets
                .iter()
                .map(|&p| Motion {
                    start: p,
                    current: p,
                    target: p,
                    since: now,
                })
                .collect();
        }
        let mut animating = false;
        for (motion, &target) in self.motions.iter_mut().zip(targets) {
            if motion.target != target {
                motion.start = motion.current;
                motion.target = target;
                motion.since = now;
            }
            let time = motion.since.elapsed().as_secs_f32() / (duration.max(1.) / 1000.);
            let moving = style != "off" && time < 1. && motion.start != target;
            motion.current = interpolate(motion.start, target, progress(style, time));
            if moving && matches!(style, "trail" | "neovide") {
                if style == "trail" {
                    for i in (1..=8).rev() {
                        let t = (time - i as f32 * 0.025).max(0.);
                        let p = interpolate(motion.start, target, progress(style, t));
                        window.paint_quad(fill(
                            Bounds::new(p, size(px(1.5), px(height))),
                            rgb(color).opacity((9 - i) as f32 * 0.035),
                        ));
                    }
                } else {
                    let tail =
                        interpolate(motion.start, target, progress(style, (time - 0.12).max(0.)));
                    let head = motion.current;
                    let mut path = PathBuilder::fill();
                    path.move_to(head);
                    path.line_to(head + point(px(1.5), px(0.)));
                    path.line_to(tail + point(px(1.5), px(height)));
                    path.line_to(tail + point(px(0.), px(height)));
                    path.close();
                    if let Ok(path) = path.build() {
                        window.paint_path(path, rgb(color));
                    }
                }
            }
            if style != "neovide" || !moving {
                window.paint_quad(fill(
                    Bounds::new(motion.current, size(px(1.5), px(height))),
                    rgb(color),
                ));
            }
            animating |= moving;
        }
        if animating {
            window.request_animation_frame();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::progress;
    #[test]
    fn all_variants_converge_and_have_distinct_motion() {
        for style in ["off", "linear", "smooth", "spring", "neovide", "trail"] {
            assert!((progress(style, 1.) - 1.).abs() < 0.0001);
            assert!(progress(style, 0.5).is_finite());
        }
        assert_ne!(progress("linear", 0.25), progress("smooth", 0.25));
        assert_ne!(progress("spring", 0.25), progress("neovide", 0.25));
        assert_eq!(progress("off", 0.), 1.);
    }
}
