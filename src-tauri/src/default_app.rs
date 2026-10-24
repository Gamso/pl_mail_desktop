//! Lets Windows list this application among the mail apps it can open `mailto:` links with.
//!
//! Everything is written under HKEY_CURRENT_USER (no administrator rights). Windows does not
//! allow an application to make itself the default: this only makes it appear in
//! *Settings → Apps → Default apps*, where the user picks it once.

use std::{io, path::Path};

use winreg::{RegKey, enums::HKEY_CURRENT_USER};

const PROG_ID: &str = r"Software\Classes\plMailDesktop.Mailto";
const CLIENT: &str = r"Software\Clients\Mail\plMail Desktop";
const CAPABILITIES: &str = r"Software\Clients\Mail\plMail Desktop\Capabilities";
const REGISTERED: &str = r"Software\RegisteredApplications";
const REGISTERED_NAME: &str = "plMail Desktop";

pub fn register(exe: &Path, name: &str, description: &str) -> io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let command = format!("\"{}\" \"%1\"", exe.display());

    let (prog, _) = hkcu.create_subkey(PROG_ID)?;
    prog.set_value("", &format!("{name} mailto link"))?;
    let (icon, _) = prog.create_subkey("DefaultIcon")?;
    icon.set_value("", &format!("\"{}\",0", exe.display()))?;
    let (open, _) = prog.create_subkey(r"shell\open\command")?;
    open.set_value("", &command)?;

    let (client, _) = hkcu.create_subkey(CLIENT)?;
    client.set_value("", &name)?;
    let (client_open, _) = client.create_subkey(r"shell\open\command")?;
    client_open.set_value("", &format!("\"{}\"", exe.display()))?;

    let (caps, _) = hkcu.create_subkey(CAPABILITIES)?;
    caps.set_value("ApplicationName", &name)?;
    caps.set_value("ApplicationDescription", &description)?;
    let (urls, _) = caps.create_subkey("URLAssociations")?;
    urls.set_value("mailto", &"plMailDesktop.Mailto")?;

    let (registered, _) = hkcu.create_subkey(REGISTERED)?;
    registered.set_value(REGISTERED_NAME, &CAPABILITIES)?;
    Ok(())
}

pub fn unregister() -> io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for key in [PROG_ID, CLIENT] {
        ignore_missing(hkcu.delete_subkey_all(key))?;
    }
    if let Ok(registered) = hkcu.open_subkey_with_flags(REGISTERED, winreg::enums::KEY_SET_VALUE) {
        ignore_missing(registered.delete_value(REGISTERED_NAME))?;
    }
    Ok(())
}

fn ignore_missing(result: io::Result<()>) -> io::Result<()> {
    match result {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}
