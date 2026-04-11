/// Level & XP configuration for the quiz platform.

#[derive(Debug, Clone)]
pub struct LevelConfig {
    pub level: i32,
    pub name: &'static str,
    pub icon: &'static str,
    pub total_xp_required: i64,
}

pub const LEVELS: &[LevelConfig] = &[
    LevelConfig { level: 1, name: "Pemula",   icon: "🌱", total_xp_required: 0 },
    LevelConfig { level: 2, name: "Pelajar",  icon: "📖", total_xp_required: 500 },
    LevelConfig { level: 3, name: "Terampil", icon: "✏️", total_xp_required: 2_000 },
    LevelConfig { level: 4, name: "Mahir",    icon: "🎯", total_xp_required: 5_000 },
    LevelConfig { level: 5, name: "Cakap",    icon: "🔥", total_xp_required: 10_000 },
    LevelConfig { level: 6, name: "Ahli",     icon: "💡", total_xp_required: 20_000 },
    LevelConfig { level: 7, name: "Master",   icon: "🏆", total_xp_required: 40_000 },
    LevelConfig { level: 8, name: "Legenda",  icon: "👑", total_xp_required: 80_000 },
];

pub const XP_DAILY_CAP: i64 = 2_000;

/// Compute the current level, optional next level, and progress (0.0–1.0)
/// based on a user's accumulated `total_xp`.
pub fn compute_level_info(total_xp: i64) -> (&'static LevelConfig, Option<&'static LevelConfig>, f64) {
    let current = LEVELS
        .iter()
        .filter(|l| total_xp >= l.total_xp_required)
        .last()
        .unwrap_or(&LEVELS[0]);

    let next = LEVELS.iter().find(|l| l.total_xp_required > current.total_xp_required);

    let progress = match next {
        Some(n) => {
            let xp_in_level = total_xp - current.total_xp_required;
            let xp_span = n.total_xp_required - current.total_xp_required;
            (xp_in_level as f64 / xp_span as f64).clamp(0.0, 1.0)
        }
        None => 1.0,
    };

    (current, next, progress)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_1_at_zero_xp() {
        let (cfg, next, progress) = compute_level_info(0);
        assert_eq!(cfg.level, 1);
        assert!(next.is_some());
        assert_eq!(progress, 0.0);
    }

    #[test]
    fn test_level_2_at_500_xp() {
        let (cfg, _, _) = compute_level_info(500);
        assert_eq!(cfg.level, 2);
    }

    #[test]
    fn test_level_8_at_80000_xp() {
        let (cfg, next, progress) = compute_level_info(80_000);
        assert_eq!(cfg.level, 8);
        assert!(next.is_none());
        assert_eq!(progress, 1.0);
    }

    #[test]
    fn test_max_progress_at_level_8() {
        let (cfg, next, progress) = compute_level_info(999_999);
        assert_eq!(cfg.level, 8);
        assert!(next.is_none());
        assert_eq!(progress, 1.0);
    }

    #[test]
    fn test_mid_level_progress() {
        // Level 2 goes from 500 to 2000 (span 1500). At 1250 total XP → 750/1500 = 0.5
        let (cfg, _, progress) = compute_level_info(1_250);
        assert_eq!(cfg.level, 2);
        assert!((progress - 0.5).abs() < 1e-9);
    }
}
