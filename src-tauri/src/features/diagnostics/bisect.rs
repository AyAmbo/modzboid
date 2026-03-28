use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BisectSession {
    pub id: String,
    pub all_mods: Vec<String>,
    pub suspects: Vec<String>,
    pub test_mods: Vec<String>,
    pub step: usize,
    pub max_steps: usize,
    pub status: String, // "testing", "found", "not_found"
    pub culprit: Option<String>,
}

/// Start a new bisect session.
///
/// Splits the full mod list in half. The user should test with `test_mods` enabled.
/// If the crash reproduces, the culprit is in `test_mods`. Otherwise, it's in the other half.
pub fn start_bisect(all_mods: Vec<String>) -> BisectSession {
    let max_steps = if all_mods.is_empty() {
        0
    } else {
        (all_mods.len() as f64).log2().ceil() as usize
    };

    if all_mods.is_empty() {
        return BisectSession {
            id: uuid::Uuid::new_v4().to_string(),
            all_mods: vec![],
            suspects: vec![],
            test_mods: vec![],
            step: 0,
            max_steps: 0,
            status: "not_found".into(),
            culprit: None,
        };
    }

    if all_mods.len() == 1 {
        return BisectSession {
            id: uuid::Uuid::new_v4().to_string(),
            suspects: all_mods.clone(),
            test_mods: all_mods.clone(),
            culprit: Some(all_mods[0].clone()),
            all_mods,
            step: 0,
            max_steps: 0,
            status: "found".into(),
        };
    }

    let suspects = all_mods.clone();
    let mid = suspects.len() / 2;
    let test_mods = suspects[..mid].to_vec();

    BisectSession {
        id: uuid::Uuid::new_v4().to_string(),
        all_mods,
        suspects,
        test_mods,
        step: 1,
        max_steps,
        status: "testing".into(),
        culprit: None,
    }
}

/// Report the result of a bisect test step.
///
/// - `crashed = true`: the bug is in `test_mods` (first half of suspects)
/// - `crashed = false`: the bug is in the other half of suspects
///
/// Narrows down suspects and sets up the next test, or declares found/not_found.
pub fn report_bisect(session: &BisectSession, crashed: bool) -> BisectSession {
    let mut new_session = session.clone();
    new_session.step += 1;

    let mid = session.suspects.len() / 2;
    let first_half = &session.suspects[..mid];
    let second_half = &session.suspects[mid..];

    let new_suspects = if crashed {
        first_half.to_vec()
    } else {
        second_half.to_vec()
    };

    if new_suspects.len() <= 1 {
        new_session.suspects = new_suspects.clone();
        new_session.test_mods = new_suspects.clone();
        if new_suspects.len() == 1 {
            new_session.status = "found".into();
            new_session.culprit = Some(new_suspects[0].clone());
        } else {
            new_session.status = "not_found".into();
        }
    } else {
        let next_mid = new_suspects.len() / 2;
        new_session.test_mods = new_suspects[..next_mid].to_vec();
        new_session.suspects = new_suspects;
        new_session.status = "testing".into();
    }

    new_session
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bisect_8_mods_find_culprit() {
        let mods: Vec<String> = (0..8).map(|i| format!("Mod{}", i)).collect();
        // The "culprit" is Mod5 (index 5, in the second half)
        let culprit = "Mod5";

        let mut session = start_bisect(mods.clone());
        assert_eq!(session.status, "testing");
        assert_eq!(session.max_steps, 3);
        assert_eq!(session.test_mods.len(), 4); // first half: Mod0-Mod3

        // Step 1: test_mods = [Mod0, Mod1, Mod2, Mod3]. Mod5 not here → no crash
        let crashed = session.test_mods.contains(&culprit.to_string());
        assert!(!crashed);
        session = report_bisect(&session, crashed);
        // suspects should now be second half: [Mod4, Mod5, Mod6, Mod7]
        assert_eq!(session.suspects, vec!["Mod4", "Mod5", "Mod6", "Mod7"]);
        assert_eq!(session.status, "testing");

        // Step 2: test_mods = [Mod4, Mod5]. Mod5 is here → crash
        let crashed = session.test_mods.contains(&culprit.to_string());
        assert!(crashed);
        session = report_bisect(&session, crashed);
        // suspects = [Mod4, Mod5]
        assert_eq!(session.suspects, vec!["Mod4", "Mod5"]);
        assert_eq!(session.status, "testing");

        // Step 3: test_mods = [Mod4]. Mod5 not here → no crash
        let crashed = session.test_mods.contains(&culprit.to_string());
        assert!(!crashed);
        session = report_bisect(&session, crashed);
        // suspects = [Mod5] → found!
        assert_eq!(session.status, "found");
        assert_eq!(session.culprit, Some("Mod5".into()));
    }

    #[test]
    fn test_bisect_8_mods_find_in_first_half() {
        let mods: Vec<String> = (0..8).map(|i| format!("Mod{}", i)).collect();
        let culprit = "Mod2";

        let mut session = start_bisect(mods);
        assert_eq!(session.status, "testing");

        // Step 1: test_mods = [Mod0, Mod1, Mod2, Mod3]. Mod2 is here → crash
        let crashed = session.test_mods.contains(&culprit.to_string());
        assert!(crashed);
        session = report_bisect(&session, crashed);
        assert_eq!(session.suspects, vec!["Mod0", "Mod1", "Mod2", "Mod3"]);

        // Step 2: test_mods = [Mod0, Mod1]. Mod2 not here → no crash
        let crashed = session.test_mods.contains(&culprit.to_string());
        assert!(!crashed);
        session = report_bisect(&session, crashed);
        assert_eq!(session.suspects, vec!["Mod2", "Mod3"]);

        // Step 3: test_mods = [Mod2]. Mod2 is here → crash
        let crashed = session.test_mods.contains(&culprit.to_string());
        assert!(crashed);
        session = report_bisect(&session, crashed);
        assert_eq!(session.status, "found");
        assert_eq!(session.culprit, Some("Mod2".into()));
    }

    #[test]
    fn test_bisect_single_mod() {
        let mods = vec!["OnlyMod".to_string()];
        let session = start_bisect(mods);
        // With only 1 mod, it's immediately found
        assert_eq!(session.status, "found");
        assert_eq!(session.suspects, vec!["OnlyMod"]);
        assert_eq!(session.culprit, Some("OnlyMod".into()));
    }

    #[test]
    fn test_bisect_empty() {
        let session = start_bisect(vec![]);
        assert_eq!(session.status, "not_found");
        assert!(session.suspects.is_empty());
        assert!(session.test_mods.is_empty());
    }

    #[test]
    fn test_bisect_two_mods() {
        let mods = vec!["ModA".into(), "ModB".into()];
        let mut session = start_bisect(mods);
        assert_eq!(session.status, "testing");
        assert_eq!(session.test_mods, vec!["ModA"]);
        assert_eq!(session.max_steps, 1);

        // If crash → culprit is ModA
        session = report_bisect(&session, true);
        assert_eq!(session.status, "found");
        assert_eq!(session.culprit, Some("ModA".into()));
    }

    #[test]
    fn test_bisect_two_mods_no_crash() {
        let mods = vec!["ModA".into(), "ModB".into()];
        let mut session = start_bisect(mods);
        assert_eq!(session.test_mods, vec!["ModA"]);

        // No crash with ModA → culprit is ModB
        session = report_bisect(&session, false);
        assert_eq!(session.status, "found");
        assert_eq!(session.culprit, Some("ModB".into()));
    }

    #[test]
    fn test_bisect_session_serialization() {
        let session = start_bisect(vec!["A".into(), "B".into(), "C".into(), "D".into()]);
        let json = serde_json::to_string(&session).unwrap();
        let restored: BisectSession = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.all_mods, session.all_mods);
        assert_eq!(restored.suspects, session.suspects);
        assert_eq!(restored.test_mods, session.test_mods);
        assert_eq!(restored.status, session.status);
    }

    #[test]
    fn test_bisect_step_count() {
        let mods: Vec<String> = (0..8).map(|i| format!("Mod{}", i)).collect();
        let session = start_bisect(mods);
        assert_eq!(session.max_steps, 3); // log2(8) = 3

        let mods16: Vec<String> = (0..16).map(|i| format!("Mod{}", i)).collect();
        let session16 = start_bisect(mods16);
        assert_eq!(session16.max_steps, 4); // log2(16) = 4

        let mods3: Vec<String> = (0..3).map(|i| format!("Mod{}", i)).collect();
        let session3 = start_bisect(mods3);
        assert_eq!(session3.max_steps, 2); // ceil(log2(3)) = 2
    }
}
