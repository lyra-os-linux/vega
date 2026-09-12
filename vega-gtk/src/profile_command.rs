//! Local, unprivileged profile contract used by Lyra Welcome. This calls the
//! very same save/restore implementation as the GTK profile cards.
use crate::dock::{self, DesktopProfile};

#[derive(Debug, PartialEq)]
enum Action {
    Get,
    Set(DesktopProfile),
}

fn parse(args: &[String]) -> Result<Action, &'static str> {
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["--desktop-profile", "get"] => Ok(Action::Get),
        ["--desktop-profile", "set", id] => DesktopProfile::from_id(id)
            .map(Action::Set)
            .ok_or("unknown desktop profile"),
        _ => Err(
            "usage: vega-gtk --desktop-profile get | set <lyra|vanilla|ubuntu|windows10|windows11|macos>",
        ),
    }
}

/// None delegates ordinary launches to GTK. Contract errors never open a window.
pub fn run(args: Vec<String>) -> Option<u8> {
    if !args.iter().any(|arg| arg == "--desktop-profile") {
        return None;
    }
    let action = match parse(&args) {
        Ok(action) => action,
        Err(error) => {
            eprintln!("{error}");
            return Some(2);
        }
    };
    let result = (|| {
        // A suite Set may recover an interrupted transition; Get stays read-only.
        if !dock::suite_available() || matches!(action, Action::Get) {
            dock::confirmed_profile()?;
        }
        if let Action::Set(profile) = action {
            dock::apply_profile(profile)?;
            // The process exits after the write; flush dconf before returning.
            gtk::gio::Settings::sync();
        }
        dock::confirmed_profile()
    })();
    match result {
        Ok(actual) => {
            if let Action::Set(requested) = action
                && actual != requested
            {
                eprintln!("desktop profile readback differs from the requested value");
                return Some(1);
            }
            println!("{}", actual.id());
            Some(0)
        }
        Err(error) => {
            eprintln!("{error}");
            Some(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| (*s).into()).collect()
    }
    #[test]
    fn contract_accepts_only_exact_actions_and_known_profiles() {
        assert_eq!(parse(&args(&["--desktop-profile", "get"])), Ok(Action::Get));
        for id in [
            "lyra",
            "vanilla",
            "ubuntu",
            "windows10",
            "windows11",
            "macos",
        ] {
            let profile = DesktopProfile::from_id(id).unwrap();
            assert_eq!(profile.id(), id);
            assert_eq!(
                parse(&args(&["--desktop-profile", "set", id])),
                Ok(Action::Set(profile))
            );
        }
        for values in [
            vec!["--desktop-profile"],
            vec!["--desktop-profile", "get", "extra"],
            vec!["--desktop-profile", "set", "invalid"],
            vec!["--desktop-profile", "set", "lyra", "extra"],
        ] {
            assert!(parse(&args(&values)).is_err());
        }
        assert_eq!(run(vec![]), None);
        assert_eq!(run(args(&["--desktop-profile", "set", "invalid"])), Some(2));
    }
}
