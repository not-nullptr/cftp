pub mod auth;
pub mod cwd;
pub mod impl_command;
pub mod opts;
pub mod pass;
pub mod retr;
pub mod rnfr;
pub mod rnto;
pub mod stor;
pub mod r#type;
pub mod user;

use crate::{impl_command, unit_commands};

unit_commands![
    (feat, Feat),
    (list, List),
    (pasv, Pasv),
    (pwd, Pwd),
    (syst, Syst),
    (utf8, Utf8), // shouldn't technically be unit but rust strings are always UTF-8 so clients sort of need to deal with it
    (pbsz, Pbsz),
];

impl_command! {
    Auth | "AUTH" => auth,
    User | "USER" => user,
    Pass | "PASS" => pass,
    Cwd | "CWD" => cwd,
    Pwd | "PWD" => pwd,
    Type | "TYPE" => r#type,
    Pasv | "PASV" => pasv,
    List | "LIST" => list,
    Retr | "RETR" => retr,
    Syst | "SYST" => syst,
    Stor | "STOR" => stor,
    Feat | "FEAT" => feat,
    Opts | "OPTS" => opts,
    Utf8 | "UTF8" => utf8,
    Pbsz | "PBSZ" => pbsz,
    Rnfr | "RNFR" => rnfr,
    Rnto | "RNTO" => rnto,
}
