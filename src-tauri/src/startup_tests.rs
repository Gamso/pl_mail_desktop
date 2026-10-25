use crate::startup::{at_logon, opens_window};

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[test]
fn only_the_autostart_flag_marks_a_launch_at_logon() {
    assert!(at_logon(&args(&["plmail.exe", "--minimized"])));
    assert!(!at_logon(&args(&["plmail.exe"])));
    assert!(!at_logon(&args(&[
        "plmail.exe",
        "mailto:alice@example.com"
    ])));
}

#[test]
fn a_launch_at_logon_opens_the_window_unless_the_user_chose_the_tray() {
    let logon = args(&["plmail.exe", "--minimized"]);
    assert!(opens_window(&logon, false));
    assert!(!opens_window(&logon, true));
}

#[test]
fn a_launch_by_hand_always_opens_the_window() {
    let by_hand = args(&["plmail.exe"]);
    assert!(opens_window(&by_hand, false));
    assert!(opens_window(&by_hand, true));
}
