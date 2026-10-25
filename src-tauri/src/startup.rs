//! What a launch does with the mail window.

/// Windows' own autostart entry passes this flag; a launch from the Start menu or a shortcut does not.
const AT_LOGON_FLAG: &str = "--minimized";

/// Was this launch made by Windows at logon?
pub fn at_logon(args: &[String]) -> bool {
    args.iter().any(|a| a == AT_LOGON_FLAG)
}

/// Does this launch open the mail window? A launch at logon stays in the tray only when the user
/// chose that; every other launch opens it.
pub fn opens_window(args: &[String], start_in_tray: bool) -> bool {
    !(at_logon(args) && start_in_tray)
}
