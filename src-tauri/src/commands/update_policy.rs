//! Who decides whether BridgeLab looks for new versions at startup.
//!
//! Three sources, strongest first:
//!
//! 1. **Machine policy**, for IT departments: the environment variable
//!    `BRIDGELAB_DISABLE_UPDATE_CHECK=1`, or a `policy.json` containing
//!    `{"disable_update_check": true}` in the machine-wide folder —
//!    `%ProgramData%\BridgeLab\` on Windows,
//!    `/Library/Application Support/BridgeLab/` on macOS,
//!    `/etc/bridgelab/` on Linux. The file turns the check off for every
//!    user of the machine, and the Settings checkbox is shown locked. The
//!    variable does the same for every BridgeLab started with it in its
//!    environment, which is the whole machine only where it is set
//!    system-wide (see the manual).
//!
//!    A policy file that cannot be used (not valid JSON, unreadable, or on
//!    Windows not owned by Administrators or SYSTEM, or changeable through
//!    its permissions by anyone but an administrator) is ignored, but not
//!    silently: it is logged and Settings → Privacy names the file and the
//!    reason, so an administrator does not believe a policy is in force
//!    when it is not.
//! 2. **The installer's choice**: the Windows setup asks, and writes
//!    `installer.json` (`{"update_check_on_startup": false}`) into the
//!    user's BridgeLab data folder. The app copies it into the preferences
//!    the first time, when the user has not chosen yet.
//! 3. **The user's preference** in Settings → Privacy, on by default.
//!
//! Only 1 and 2 are read here; the preference lives in the database.
//!
//! The same machine policy can also force the opt-in usage statistics off:
//! `BRIDGELAB_DISABLE_TELEMETRY=1`, or `"disable_telemetry": true` in the
//! same `policy.json`. Nothing is sent then, whatever the user ticked, and
//! the Settings checkbox is shown locked (see [`telemetry_policy`]).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const POLICY_ENV: &str = "BRIDGELAB_DISABLE_UPDATE_CHECK";
pub const TELEMETRY_POLICY_ENV: &str = "BRIDGELAB_DISABLE_TELEMETRY";

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct UpdatePolicy {
    /// True when a machine policy turns the startup check off.
    pub disabled_by_policy: bool,
    /// What turned it off (the environment variable's name or the policy file path), for the
    /// Settings hint.
    pub policy_source: Option<String>,
    /// The Windows installer's choice, when one was recorded.
    pub installer_choice: Option<bool>,
    /// A policy file that is there but was ignored, and why.
    pub policy_problem: Option<PolicyProblem>,
}

/// A machine policy file that exists but could not be used, for the notice
/// in Settings → Privacy.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PolicyProblem {
    pub path: String,
    /// `invalid_json`, `unreadable`, `untrusted_owner` or
    /// `untrusted_permissions` (both Windows), for the localized sentence.
    pub kind: &'static str,
    /// The parser's or the system's own words, or BridgeLab's in English.
    pub detail: String,
    /// For a reason BridgeLab words itself (the Windows trust checks): the
    /// i18n key under `settings.policyDetail.` and its parameters, so the
    /// notice is shown in the user's language. `None` for the parser's or
    /// the system's own words, shown as they are.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail_key: Option<&'static str>,
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub detail_args: std::collections::BTreeMap<&'static str, String>,
}

/// A machine policy that forces the usage statistics off.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TelemetryPolicy {
    pub disabled_by_policy: bool,
    /// The environment variable's name or the policy file path, for the Settings hint.
    pub policy_source: Option<String>,
}

struct PolicyFile {
    disable_update_check: bool,
    disable_telemetry: bool,
}

/// A policy switch, read leniently: IT tooling writes `true`, `"true"`,
/// `1` or `"yes"` alike, and an administrator who asked for "off" must get
/// it whichever they typed.
fn policy_flag(v: Option<&serde_json::Value>) -> bool {
    match v {
        Some(serde_json::Value::Bool(b)) => *b,
        Some(serde_json::Value::Number(n)) => n.as_i64() == Some(1),
        Some(serde_json::Value::String(s)) => env_says_disabled(Some(s.clone())),
        _ => false,
    }
}

/// What reading the machine policy file gave.
enum PolicyRead {
    /// No file: no policy, nothing to say.
    Absent,
    Applied(PolicyFile),
    /// A file is there but cannot be used: it is ignored, and said so.
    Rejected(PolicyProblem),
}

impl PolicyRead {
    fn file(&self) -> Option<&PolicyFile> {
        match self {
            PolicyRead::Applied(f) => Some(f),
            _ => None,
        }
    }

    fn problem(&self) -> Option<PolicyProblem> {
        match self {
            PolicyRead::Rejected(p) => Some(p.clone()),
            _ => None,
        }
    }
}

/// Whether a policy file may be trusted as the administrator's, beyond
/// being readable: on Windows, who owns it and who may change it (see
/// [`machine_trust_check`]). `Err` carries the reason.
type TrustCheck = fn(&Path) -> Result<(), TrustProblem>;

fn read_policy_file(policy_file: &Path, trust_check: TrustCheck) -> PolicyRead {
    let rejected_with = |kind: &'static str, detail: String, detail_key: Option<&'static str>, detail_args: std::collections::BTreeMap<&'static str, String>| {
        let problem = PolicyProblem { path: policy_file.display().to_string(), kind, detail, detail_key, detail_args };
        log_policy_problem(&problem);
        PolicyRead::Rejected(problem)
    };
    let rejected = |kind: &'static str, detail: String| {
        let problem = PolicyProblem { path: policy_file.display().to_string(), kind, detail, detail_key: None, detail_args: Default::default() };
        log_policy_problem(&problem);
        PolicyRead::Rejected(problem)
    };
    match std::fs::metadata(policy_file) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return PolicyRead::Absent,
        Err(e) => return rejected("unreadable", e.to_string()),
        Ok(_) => {}
    }
    if let Err(problem) = trust_check(policy_file) {
        return rejected_with(problem.kind, problem.detail, Some(problem.key), problem.args.into_iter().collect());
    }
    let text = match std::fs::read_to_string(policy_file) {
        Ok(text) => text,
        Err(e) => return rejected("unreadable", e.to_string()),
    };
    // PowerShell 5.1 (`Set-Content -Encoding UTF8`) writes a byte-order
    // mark, which a JSON parser rejects: the policy was then ignored.
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    match serde_json::from_str::<serde_json::Value>(text) {
        Ok(v) => PolicyRead::Applied(PolicyFile {
            disable_update_check: policy_flag(v.get("disable_update_check")),
            disable_telemetry: policy_flag(v.get("disable_telemetry")),
        }),
        Err(e) => rejected("invalid_json", e.to_string()),
    }
}

/// One line on standard error per distinct problem: the policy is read at
/// every start, by Settings and before every telemetry send, and the same
/// broken file would otherwise be reported each time.
fn log_policy_problem(problem: &PolicyProblem) {
    static LOGGED: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
    let line = format!(
        "BridgeLab: machine policy {} was ignored ({}): {}",
        problem.path, problem.kind, problem.detail
    );
    let Ok(mut last) = LOGGED.lock() else { return };
    if last.as_deref() != Some(line.as_str()) {
        eprintln!("{line}");
        *last = Some(line);
    }
}

/// On Windows every user may create folders and files in %ProgramData%,
/// and owns what they create: where IT has not created
/// `BridgeLab\policy.json`, any standard user could, and so switch the
/// update notice and the usage statistics off for every other user of the
/// machine, shown as locked "by your administrator". The file is trusted
/// only when both it and its folder belong to Administrators or SYSTEM, as
/// they do when an administrator or a deployment tool (Intune, Group
/// Policy, Configuration Manager) creates them, and when no one but an
/// administrator may change them (see [`trust_decision`]).
#[cfg(target_os = "windows")]
fn machine_trust_check(policy_file: &Path) -> Result<(), TrustProblem> {
    let folder = policy_file.parent().unwrap_or(policy_file);
    let read = |path: &Path| {
        win_security::object_security(path).map_err(|e| {
            TrustProblem::new(
                "untrusted_owner",
                "unreadableSecurity",
                format!("the owner and permissions of {} could not be read: {}", path.display(), e),
                vec![("path", path.display().to_string()), ("error", e.to_string())],
            )
        })
    };
    let (file, dir) = (read(policy_file)?, read(folder)?);
    trust_decision(&file, &dir).map_err(|distrust| distrust.problem(policy_file, folder, win_security::account_name))
}

/// Elsewhere the machine-wide folders (`/etc`, `/Library/Application
/// Support`) can only be written by root.
#[cfg(not(target_os = "windows"))]
fn machine_trust_check(_policy_file: &Path) -> Result<(), TrustProblem> {
    Ok(())
}

/// The Windows trust rule, apart from the calls that read owners and
/// permissions: plain code over SID strings, so it is tested on every OS.
mod trust {
    // Only the Windows build and the tests call it.
    #![cfg_attr(not(target_os = "windows"), allow(dead_code))]
    use std::path::Path;

    /// Why a policy file is not trusted, for the notice: `untrusted_owner`
    /// or `untrusted_permissions`, and the details.
    #[derive(Debug, PartialEq)]
    pub(crate) struct TrustProblem {
        pub(crate) kind: &'static str,
        /// In English, for the log.
        pub(crate) detail: String,
        /// The same reason for the notice: the key under
        /// `settings.policyDetail.` and its parameters.
        pub(crate) key: &'static str,
        pub(crate) args: Vec<(&'static str, String)>,
    }

    impl TrustProblem {
        pub(crate) fn new(kind: &'static str, key: &'static str, detail: String, args: Vec<(&'static str, String)>) -> Self {
            TrustProblem { kind, detail, key, args }
        }
    }

    pub(crate) const ADMINISTRATORS: &str = "S-1-5-32-544";
    pub(crate) const SYSTEM: &str = "S-1-5-18";
    pub(crate) const TRUSTED_INSTALLER: &str = "S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464";
    /// CREATOR OWNER and OWNER RIGHTS stand for the owner, which the owner
    /// rule already requires to be an administrator; on an object they
    /// mostly appear as inherit-only entries, which do not apply to it.
    pub(crate) const CREATOR_OWNER: &str = "S-1-3-0";
    pub(crate) const OWNER_RIGHTS: &str = "S-1-3-4";

    /// The principals that may hold a right to change the policy: the
    /// machine's administrators and the system itself. Anyone else — a
    /// built-in group such as Users, a named account, a custom or domain
    /// group, an unknown SID — makes the file untrustworthy. An earlier
    /// version looked only for a few well-known groups, and a write right
    /// given to a named user went through.
    fn may_change(sid: &str) -> bool {
        [ADMINISTRATORS, SYSTEM, TRUSTED_INSTALLER, CREATOR_OWNER, OWNER_RIGHTS]
            .iter()
            .any(|trusted| trusted.eq_ignore_ascii_case(sid))
    }

    fn may_own(sid: &str) -> bool {
        [ADMINISTRATORS, SYSTEM].iter().any(|trusted| trusted.eq_ignore_ascii_case(sid))
    }

    /// One entry of a DACL, as read from the file system.
    #[derive(Clone, Debug)]
    pub(crate) struct Ace {
        /// The SID in string form (`S-1-5-32-545`); empty where it is not
        /// read (a conditional deny, an entry of unknown type).
        pub(crate) sid: String,
        pub(crate) kind: AceKind,
        pub(crate) mask: u32,
        /// Only passed down to new children: it does not apply to the object.
        pub(crate) inherit_only: bool,
    }

    /// What an entry does, as far as the rule can tell. Windows grants the
    /// rights of a conditional allow (a callback or object entry) whenever
    /// its condition holds, so it counts as an allow; a conditional deny may
    /// not apply, so it cancels nothing.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) enum AceKind {
        Allow,
        AllowConditional,
        Deny,
        DenyConditional,
        /// An entry whose rights cannot be told (a compound entry, an
        /// audit or label entry where an access entry should be, a type
        /// newer than this code): the policy is refused.
        Unknown(u8),
    }

    /// Owner and DACL of a file or folder. `dacl: None` is a NULL DACL,
    /// which grants everyone everything.
    #[derive(Debug)]
    pub(crate) struct ObjectSecurity {
        /// The owner's SID in string form; empty when there is none.
        pub(crate) owner: String,
        pub(crate) dacl: Option<Vec<Ace>>,
    }

    /// Everyone, for a NULL DACL.
    pub(crate) const EVERYONE: &str = "S-1-1-0";

    // Access rights (winnt.h), spelled out so the rule is tested on every OS.
    pub(crate) const FILE_WRITE_DATA: u32 = 0x0002; // FILE_ADD_FILE on a folder
    pub(crate) const FILE_APPEND_DATA: u32 = 0x0004; // FILE_ADD_SUBDIRECTORY on a folder
    pub(crate) const FILE_DELETE_CHILD: u32 = 0x0040;
    pub(crate) const DELETE: u32 = 0x0001_0000;
    pub(crate) const WRITE_DAC: u32 = 0x0004_0000;
    pub(crate) const WRITE_OWNER: u32 = 0x0008_0000;
    pub(crate) const GENERIC_ALL: u32 = 0x1000_0000;
    pub(crate) const GENERIC_EXECUTE: u32 = 0x2000_0000;
    pub(crate) const GENERIC_WRITE: u32 = 0x4000_0000;
    pub(crate) const GENERIC_READ: u32 = 0x8000_0000;

    /// Rights on the file that let their holder change what it says, or
    /// replace it, or give themselves that right.
    pub(crate) const FILE_CHANGE_RIGHTS: u32 = FILE_WRITE_DATA | FILE_APPEND_DATA | WRITE_DAC | WRITE_OWNER | DELETE;
    /// Rights on the folder that let their holder delete or replace the
    /// file in it, or take the folder over. Not "create files":
    /// %ProgramData% passes that down to Users on every folder created in
    /// it, and a file a user creates there is theirs, which the owner rule
    /// refuses.
    pub(crate) const FOLDER_CHANGE_RIGHTS: u32 = FILE_DELETE_CHILD | DELETE | WRITE_DAC | WRITE_OWNER;

    /// Generic rights as the file system maps them (FILE_GENERIC_*), so
    /// that a deny and an allow of the same right meet.
    fn map_generic(mask: u32) -> u32 {
        let mut mapped = mask & 0x0FFF_FFFF;
        if mask & GENERIC_READ != 0 {
            mapped |= 0x0012_0089;
        }
        if mask & GENERIC_WRITE != 0 {
            mapped |= 0x0012_0116;
        }
        if mask & GENERIC_EXECUTE != 0 {
            mapped |= 0x0012_00A0;
        }
        if mask & GENERIC_ALL != 0 {
            mapped |= 0x001F_01FF;
        }
        mapped
    }

    /// What in a DACL lets someone other than an administrator change the
    /// object.
    #[derive(Debug, PartialEq)]
    enum Finding {
        Holder(String),
        Unevaluable(u8),
    }

    /// The first principal outside [`may_change`] that `dacl` grants one of
    /// `rights`, or an entry whose rights cannot be told. Each SID's own
    /// entries are read in order: a right it is denied before it is allowed
    /// stays denied; a deny for one SID does not take away what another SID
    /// is allowed; a conditional allow counts as an allow and a conditional
    /// deny counts for nothing. Inherit-only entries do not apply to the
    /// object.
    fn untrusted_holder(dacl: Option<&[Ace]>, rights: u32) -> Option<Finding> {
        let Some(dacl) = dacl else { return Some(Finding::Holder(EVERYONE.to_string())) };
        let applying: Vec<&Ace> = dacl.iter().filter(|a| !a.inherit_only).collect();
        if let Some(ace_type) = applying.iter().find_map(|a| match a.kind {
            AceKind::Unknown(t) => Some(t),
            _ => None,
        }) {
            return Some(Finding::Unevaluable(ace_type));
        }
        let counted: Vec<&Ace> = applying
            .into_iter()
            .filter(|a| a.kind != AceKind::DenyConditional && !may_change(&a.sid))
            .collect();
        for (i, first) in counted.iter().enumerate() {
            // Each SID once, from its first entry.
            if counted[..i].iter().any(|a| a.sid.eq_ignore_ascii_case(&first.sid)) {
                continue;
            }
            let (mut allowed, mut denied) = (0u32, 0u32);
            for ace in counted[i..].iter().filter(|a| a.sid.eq_ignore_ascii_case(&first.sid)) {
                let mask = map_generic(ace.mask);
                if matches!(ace.kind, AceKind::Allow | AceKind::AllowConditional) {
                    allowed |= mask & !denied;
                } else {
                    denied |= mask & !allowed;
                }
            }
            if allowed & rights != 0 {
                return Some(Finding::Holder(first.sid.clone()));
            }
        }
        None
    }

    /// Why a policy file and its folder are not the administrator's.
    #[derive(Debug, PartialEq)]
    pub(crate) enum Distrust {
        Owner { folder: bool },
        /// `sid` may change the file (or, `folder`, delete or replace it).
        Writable { sid: String, folder: bool },
        /// An entry of type `ace_type` whose rights cannot be told.
        Unevaluable { ace_type: u8, folder: bool },
    }

    impl Distrust {
        /// The notice, naming the principal with `name_of` (the account or
        /// group name where Windows knows it, else the SID).
        pub(crate) fn problem(&self, file: &Path, folder: &Path, name_of: impl Fn(&str) -> String) -> TrustProblem {
            match self {
                Distrust::Owner { folder: is_folder } => {
                    let path = if *is_folder { folder } else { file }.display().to_string();
                    TrustProblem::new(
                        "untrusted_owner",
                        "notAdminOwned",
                        format!("{} is not owned by Administrators or SYSTEM", path),
                        vec![("path", path)],
                    )
                }
                Distrust::Writable { sid, folder: false } => {
                    let (who, path) = (name_of(sid), file.display().to_string());
                    TrustProblem::new(
                        "untrusted_permissions",
                        "writableFile",
                        format!("{} can change {} (its permissions let them write it)", who, path),
                        vec![("who", who), ("path", path)],
                    )
                }
                Distrust::Writable { sid, folder: true } => {
                    let (who, path) = (name_of(sid), folder.display().to_string());
                    TrustProblem::new(
                        "untrusted_permissions",
                        "writableFolder",
                        format!("{} can delete or replace the files in {}, or take it over (its permissions)", who, path),
                        vec![("who", who), ("path", path)],
                    )
                }
                Distrust::Unevaluable { ace_type, folder: is_folder } => {
                    let path = if *is_folder { folder } else { file }.display().to_string();
                    let ace = format!("{:#x}", ace_type);
                    TrustProblem::new(
                        "untrusted_permissions",
                        "unevaluable",
                        format!("{} has a permission entry BridgeLab cannot evaluate (ACE type {})", path, ace),
                        vec![("path", path), ("type", ace)],
                    )
                }
            }
        }
    }

    fn distrust(finding: Finding, folder: bool) -> Distrust {
        match finding {
            Finding::Holder(sid) => Distrust::Writable { sid, folder },
            Finding::Unevaluable(ace_type) => Distrust::Unevaluable { ace_type, folder },
        }
    }

    /// Both the file and its folder owned by Administrators or SYSTEM, and
    /// no one but Administrators, SYSTEM or TrustedInstaller able to change
    /// the file, through either. An owner check alone let through a file an
    /// administrator created with permissions (or under a folder) that let
    /// others write it.
    pub(crate) fn trust_decision(file: &ObjectSecurity, folder: &ObjectSecurity) -> Result<(), Distrust> {
        if !may_own(&folder.owner) {
            return Err(Distrust::Owner { folder: true });
        }
        if !may_own(&file.owner) {
            return Err(Distrust::Owner { folder: false });
        }
        if let Some(finding) = untrusted_holder(folder.dacl.as_deref(), FOLDER_CHANGE_RIGHTS) {
            return Err(distrust(finding, true));
        }
        if let Some(finding) = untrusted_holder(file.dacl.as_deref(), FILE_CHANGE_RIGHTS) {
            return Err(distrust(finding, false));
        }
        Ok(())
    }
}

use trust::*;

/// Reading the owner and the DACL of a file or folder, as SID strings, and
/// naming a SID for the notice. Kept to the calls; the rule is
/// [`trust_decision`].
#[cfg(target_os = "windows")]
mod win_security {
    use super::trust::{Ace, AceKind, ObjectSecurity};
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use windows_sys::Win32::Foundation::{LocalFree, ERROR_SUCCESS};
    use windows_sys::Win32::Security::Authorization::{
        ConvertSidToStringSidW, ConvertStringSidToSidW, GetNamedSecurityInfoW, SE_FILE_OBJECT,
    };
    use windows_sys::Win32::Security::{
        GetAce, GetAclInformation, GetLengthSid, IsValidSid, LookupAccountSidW, AclSizeInformation, ACE_HEADER, ACL,
        ACL_SIZE_INFORMATION, DACL_SECURITY_INFORMATION, INHERIT_ONLY_ACE, OWNER_SECURITY_INFORMATION,
        PSECURITY_DESCRIPTOR, PSID, SID_NAME_USE,
    };

    // ACE types and object-entry flags (winnt.h).
    const ACCESS_ALLOWED_ACE_TYPE: u8 = 0x0;
    const ACCESS_DENIED_ACE_TYPE: u8 = 0x1;
    const ACCESS_ALLOWED_OBJECT_ACE_TYPE: u8 = 0x5;
    const ACCESS_DENIED_OBJECT_ACE_TYPE: u8 = 0x6;
    const ACCESS_ALLOWED_CALLBACK_ACE_TYPE: u8 = 0x9;
    const ACCESS_DENIED_CALLBACK_ACE_TYPE: u8 = 0xA;
    const ACCESS_ALLOWED_CALLBACK_OBJECT_ACE_TYPE: u8 = 0xB;
    const ACCESS_DENIED_CALLBACK_OBJECT_ACE_TYPE: u8 = 0xC;
    const ACE_OBJECT_TYPE_PRESENT: u32 = 0x1;
    const ACE_INHERITED_OBJECT_TYPE_PRESENT: u32 = 0x2;

    fn wide(s: &std::ffi::OsStr) -> Vec<u16> {
        s.encode_wide().chain(Some(0)).collect()
    }

    pub fn object_security(path: &Path) -> std::io::Result<ObjectSecurity> {
        let path = wide(path.as_os_str());
        let mut owner: PSID = std::ptr::null_mut();
        let mut dacl: *mut ACL = std::ptr::null_mut();
        let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        // SAFETY: a NUL-terminated path that outlives the call; the owner
        // SID and the DACL point into the descriptor, freed below after use.
        let err = unsafe {
            GetNamedSecurityInfoW(
                path.as_ptr(),
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                &mut owner,
                std::ptr::null_mut(),
                &mut dacl,
                std::ptr::null_mut(),
                &mut descriptor,
            )
        };
        if err != ERROR_SUCCESS {
            return Err(std::io::Error::from_raw_os_error(err as i32));
        }
        // SAFETY: `owner` and `dacl` are valid until LocalFree below.
        let result = unsafe {
            let owner = if owner.is_null() { Ok(String::new()) } else { sid_string(owner) };
            owner.and_then(|owner| {
                let dacl = if dacl.is_null() { None } else { Some(read_dacl(dacl)?) };
                Ok(ObjectSecurity { owner, dacl })
            })
        };
        // SAFETY: allocated by GetNamedSecurityInfoW, freed once.
        unsafe { LocalFree(descriptor) };
        result
    }

    /// # Safety
    /// `dacl` is a valid ACL.
    unsafe fn read_dacl(dacl: *const ACL) -> std::io::Result<Vec<Ace>> {
        let mut size = ACL_SIZE_INFORMATION::default();
        if GetAclInformation(
            dacl,
            (&mut size as *mut ACL_SIZE_INFORMATION).cast(),
            std::mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
            AclSizeInformation,
        ) == 0
        {
            return Err(std::io::Error::last_os_error());
        }
        let mut aces = Vec::with_capacity(size.AceCount as usize);
        for i in 0..size.AceCount {
            let mut raw: *mut core::ffi::c_void = std::ptr::null_mut();
            if GetAce(dacl, i, &mut raw) == 0 {
                return Err(std::io::Error::last_os_error());
            }
            aces.push(read_ace(raw.cast::<u8>().cast_const())?);
        }
        Ok(aces)
    }

    /// One entry, read conservatively: anything that is not a plain allow
    /// or deny was skipped before, and a callback entry granting a user
    /// write access when its condition holds went unseen.
    ///
    /// # Safety
    /// `raw` points to an ACE of a valid ACL.
    unsafe fn read_ace(raw: *const u8) -> std::io::Result<Ace> {
        let header = &*(raw as *const ACE_HEADER);
        let inherit_only = u32::from(header.AceFlags) & INHERIT_ONLY_ACE != 0;
        let size = usize::from(header.AceSize);
        // Every access entry starts with the header and the mask.
        let mask = if size >= 8 { std::ptr::read_unaligned(raw.add(4) as *const u32) } else { 0 };
        let unknown = || Ace { sid: String::new(), kind: AceKind::Unknown(header.AceType), mask, inherit_only };
        let (kind, sid_offset) = match header.AceType {
            ACCESS_ALLOWED_ACE_TYPE => (AceKind::Allow, 8),
            ACCESS_DENIED_ACE_TYPE => (AceKind::Deny, 8),
            // Same layout as a plain allow up to the SID, with the
            // condition after it: the condition may hold.
            ACCESS_ALLOWED_CALLBACK_ACE_TYPE => (AceKind::AllowConditional, 8),
            // Object entries: a flags word, then up to two GUIDs, then the SID.
            ACCESS_ALLOWED_OBJECT_ACE_TYPE | ACCESS_ALLOWED_CALLBACK_OBJECT_ACE_TYPE if size >= 12 => {
                let flags = std::ptr::read_unaligned(raw.add(8) as *const u32);
                let guids = usize::from(flags & ACE_OBJECT_TYPE_PRESENT != 0) + usize::from(flags & ACE_INHERITED_OBJECT_TYPE_PRESENT != 0);
                (AceKind::AllowConditional, 12 + 16 * guids)
            }
            // A deny that may not apply cancels nothing: its SID is not needed.
            ACCESS_DENIED_CALLBACK_ACE_TYPE | ACCESS_DENIED_OBJECT_ACE_TYPE | ACCESS_DENIED_CALLBACK_OBJECT_ACE_TYPE => {
                return Ok(Ace { sid: String::new(), kind: AceKind::DenyConditional, mask, inherit_only });
            }
            // Compound, audit, alarm, label, attribute, policy entries in a
            // DACL, and types newer than this code.
            _ => return Ok(unknown()),
        };
        // The SID must lie wholly inside the entry; otherwise its rights
        // cannot be told.
        if sid_offset + 8 > size {
            return Ok(unknown());
        }
        let sid = raw.add(sid_offset) as PSID;
        if IsValidSid(sid) == 0 || sid_offset + GetLengthSid(sid) as usize > size {
            return Ok(unknown());
        }
        Ok(Ace { sid: sid_string(sid)?, kind, mask, inherit_only })
    }

    /// # Safety
    /// `sid` is a valid SID.
    unsafe fn sid_string(sid: PSID) -> std::io::Result<String> {
        let mut raw: windows_sys::core::PWSTR = std::ptr::null_mut();
        if ConvertSidToStringSidW(sid, &mut raw) == 0 {
            return Err(std::io::Error::last_os_error());
        }
        let len = (0..).take_while(|&i| *raw.add(i) != 0).count();
        let text = String::from_utf16_lossy(std::slice::from_raw_parts(raw, len));
        LocalFree(raw.cast());
        Ok(text)
    }

    /// `DOMAIN\name` of a SID given as a string, or the string itself when
    /// Windows cannot name it. Only asked for the principal a refusal
    /// names, so a domain account that needs the network is looked up at
    /// most once.
    pub fn account_name(sid_text: &str) -> String {
        let text = wide(std::ffi::OsStr::new(sid_text));
        let mut sid: PSID = std::ptr::null_mut();
        // SAFETY: a NUL-terminated string; the SID is freed below.
        if unsafe { ConvertStringSidToSidW(text.as_ptr(), &mut sid) } == 0 {
            return sid_text.to_string();
        }
        let (mut name, mut domain) = ([0u16; 256], [0u16; 256]);
        let (mut name_len, mut domain_len) = (name.len() as u32, domain.len() as u32);
        let mut kind: SID_NAME_USE = 0;
        // SAFETY: buffers and lengths match; the SID is valid until LocalFree.
        let ok = unsafe {
            LookupAccountSidW(
                std::ptr::null(),
                sid,
                name.as_mut_ptr(),
                &mut name_len,
                domain.as_mut_ptr(),
                &mut domain_len,
                &mut kind,
            )
        };
        // SAFETY: allocated by ConvertStringSidToSidW, freed once.
        unsafe { LocalFree(sid) };
        if ok == 0 {
            return sid_text.to_string();
        }
        let name = String::from_utf16_lossy(&name[..name_len as usize]);
        let domain = String::from_utf16_lossy(&domain[..domain_len as usize]);
        if domain.is_empty() { name } else { format!("{domain}\\{name}") }
    }
}

#[derive(Deserialize)]
struct InstallerFile {
    update_check_on_startup: Option<bool>,
}

fn env_says_disabled(value: Option<String>) -> bool {
    matches!(
        value.as_deref().map(|v| v.trim().to_ascii_lowercase()).as_deref(),
        Some("1" | "true" | "yes" | "on")
    )
}

/// The machine-wide policy file for this platform.
pub fn machine_policy_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        program_data_dir().join("BridgeLab").join("policy.json")
    }
    #[cfg(target_os = "macos")]
    {
        PathBuf::from("/Library/Application Support/BridgeLab/policy.json")
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        PathBuf::from("/etc/bridgelab/policy.json")
    }
}

/// %ProgramData% as Windows records it for the machine, not as this
/// process's `PROGRAMDATA` variable says: a user can point that variable at
/// a folder of their own and so step around the policy their administrator
/// set.
#[cfg(target_os = "windows")]
fn program_data_dir() -> PathBuf {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::Com::CoTaskMemFree;
    use windows_sys::Win32::UI::Shell::{FOLDERID_ProgramData, SHGetKnownFolderPath};
    let mut raw: windows_sys::core::PWSTR = std::ptr::null_mut();
    // SAFETY: a valid folder id and out-pointer, default flags, no access
    // token (the current user's view, which for this folder is the
    // machine's).
    let hr = unsafe { SHGetKnownFolderPath(&FOLDERID_ProgramData, 0, std::ptr::null_mut(), &mut raw) };
    let path = (hr >= 0 && !raw.is_null()).then(|| {
        // SAFETY: on success `raw` is a NUL-terminated wide string.
        let len = (0..).take_while(|&i| unsafe { *raw.add(i) } != 0).count();
        let wide = unsafe { std::slice::from_raw_parts(raw, len) };
        PathBuf::from(std::ffi::OsString::from_wide(wide))
    });
    // SAFETY: the string is ours to free, after success and failure alike
    // (CoTaskMemFree accepts null).
    unsafe { CoTaskMemFree(raw as *const core::ffi::c_void) };
    path.unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"))
}

/// Where the Windows installer records its choice (the same data folder
/// the database lives in).
pub fn installer_choice_path() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("BridgeLab").join("installer.json"))
}

/// Resolve the policy from explicit inputs, so it can be tested without
/// touching the real environment or file system. An unreadable or
/// malformed file is ignored: a typo must not change behaviour silently
/// in a direction nobody asked for, and "on" is the documented default.
/// It is reported in `policy_problem`, so Settings can say so.
pub fn resolve(env_value: Option<String>, policy_file: &Path, installer_file: Option<&Path>) -> UpdatePolicy {
    resolve_with(env_value, policy_file, installer_file, machine_trust_check)
}

fn resolve_with(
    env_value: Option<String>,
    policy_file: &Path,
    installer_file: Option<&Path>,
    trust_check: TrustCheck,
) -> UpdatePolicy {
    // Read even when the variable decides: a broken file must still be
    // reported, since it also carries the telemetry switch.
    let read = read_policy_file(policy_file, trust_check);
    let policy_problem = read.problem();
    if env_says_disabled(env_value) {
        return UpdatePolicy {
            disabled_by_policy: true,
            policy_source: Some(POLICY_ENV.to_string()),
            installer_choice: None,
            policy_problem,
        };
    }
    let from_file = read.file().is_some_and(|p| p.disable_update_check);
    let installer_choice = installer_file
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<InstallerFile>(&s).ok())
        .and_then(|f| f.update_check_on_startup);
    UpdatePolicy {
        disabled_by_policy: from_file,
        policy_source: from_file.then(|| policy_file.display().to_string()),
        installer_choice,
        policy_problem,
    }
}

#[tauri::command]
pub fn get_update_policy() -> UpdatePolicy {
    let installer = installer_choice_path();
    resolve(std::env::var(POLICY_ENV).ok(), &machine_policy_path(), installer.as_deref())
}

/// Telemetry counterpart of [`resolve`]: same environment/file rules, with
/// `BRIDGELAB_DISABLE_TELEMETRY` and `"disable_telemetry"`. A malformed file
/// is ignored here too, which can only leave the user's own choice in
/// place — telemetry is off unless the user turned it on.
pub fn resolve_telemetry(env_value: Option<String>, policy_file: &Path) -> TelemetryPolicy {
    resolve_telemetry_with(env_value, policy_file, machine_trust_check)
}

fn resolve_telemetry_with(env_value: Option<String>, policy_file: &Path, trust_check: TrustCheck) -> TelemetryPolicy {
    if env_says_disabled(env_value) {
        return TelemetryPolicy {
            disabled_by_policy: true,
            policy_source: Some(TELEMETRY_POLICY_ENV.to_string()),
        };
    }
    let from_file = read_policy_file(policy_file, trust_check).file().is_some_and(|p| p.disable_telemetry);
    TelemetryPolicy {
        disabled_by_policy: from_file,
        policy_source: from_file.then(|| policy_file.display().to_string()),
    }
}

/// The telemetry policy of this machine, read fresh on every call so a
/// policy deployed while the app is open takes effect at the next send.
pub fn telemetry_policy() -> TelemetryPolicy {
    resolve_telemetry(std::env::var(TELEMETRY_POLICY_ENV).ok(), &machine_policy_path())
}

#[cfg(test)]
mod tests {
    use super::*;

    // The temporary files below belong to whoever runs the tests, which
    // the Windows owner rule refuses; that rule has a test of its own.
    fn any_owner(_: &Path) -> Result<(), TrustProblem> {
        Ok(())
    }

    fn resolve(env_value: Option<String>, policy_file: &Path, installer_file: Option<&Path>) -> UpdatePolicy {
        resolve_with(env_value, policy_file, installer_file, any_owner)
    }

    fn resolve_telemetry(env_value: Option<String>, policy_file: &Path) -> TelemetryPolicy {
        resolve_telemetry_with(env_value, policy_file, any_owner)
    }

    fn tmp(name: &str, content: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bl-update-policy-{}-{}", std::process::id(), name));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("f.json");
        std::fs::write(&p, content).unwrap();
        p
    }

    #[test]
    fn nothing_configured_means_no_policy_and_no_installer_choice() {
        let missing = Path::new("/nonexistent/bridgelab/policy.json");
        assert_eq!(
            resolve(None, missing, None),
            UpdatePolicy { disabled_by_policy: false, policy_source: None, installer_choice: None, policy_problem: None }
        );
    }

    #[test]
    fn the_environment_variable_turns_it_off() {
        let missing = Path::new("/nonexistent/p.json");
        for v in ["1", "true", "YES", " on "] {
            assert!(resolve(Some(v.into()), missing, None).disabled_by_policy, "{v:?}");
        }
        for v in ["0", "false", "", "maybe"] {
            assert!(!resolve(Some(v.into()), missing, None).disabled_by_policy, "{v:?}");
        }
    }

    #[test]
    fn the_policy_file_turns_it_off_and_names_itself() {
        let p = tmp("on", r#"{"disable_update_check": true}"#);
        let r = resolve(None, &p, None);
        assert!(r.disabled_by_policy);
        assert_eq!(r.policy_source.as_deref(), Some(p.display().to_string().as_str()));
        let off = tmp("off", r#"{"disable_update_check": false}"#);
        assert!(!resolve(None, &off, None).disabled_by_policy);
    }

    #[test]
    fn a_malformed_policy_file_is_ignored_and_reported() {
        let p = tmp("bad", "disable_update_check = yes");
        let r = resolve(None, &p, None);
        assert!(!r.disabled_by_policy);
        let problem = r.policy_problem.expect("the rejected file is reported");
        assert_eq!(problem.path, p.display().to_string());
        assert_eq!(problem.kind, "invalid_json");
        assert!(!problem.detail.is_empty());

        // One trailing comma is enough to lose the whole policy.
        let comma = tmp("comma", r#"{"disable_update_check": true,}"#);
        let r = resolve(None, &comma, None);
        assert!(!r.disabled_by_policy);
        assert!(r.policy_problem.unwrap().detail.contains("trailing comma"));
        assert!(!resolve_telemetry(None, &comma).disabled_by_policy);

        // Still reported when the variable already turns the check off:
        // the same file carries the telemetry switch.
        let r = resolve(Some("1".into()), &comma, None);
        assert!(r.disabled_by_policy);
        assert_eq!(r.policy_problem.unwrap().kind, "invalid_json");
    }

    #[test]
    fn a_missing_or_valid_policy_file_reports_no_problem() {
        assert_eq!(resolve(None, Path::new("/nonexistent/p.json"), None).policy_problem, None);
        let p = tmp("valid", r#"{"disable_update_check": true}"#);
        assert_eq!(resolve(None, &p, None).policy_problem, None);
    }

    #[test]
    fn a_policy_file_whose_owner_is_not_trusted_is_ignored_and_reported() {
        fn refuse(_: &Path) -> Result<(), TrustProblem> {
            Err(TrustProblem::new(
                "untrusted_owner",
                "notAdminOwned",
                r"C:\ProgramData\BridgeLab is not owned by Administrators or SYSTEM".into(),
                vec![("path", r"C:\ProgramData\BridgeLab".into())],
            ))
        }
        let p = tmp("untrusted", r#"{"disable_update_check": true, "disable_telemetry": true}"#);
        let r = resolve_with(None, &p, None, refuse);
        assert!(!r.disabled_by_policy);
        assert_eq!(r.policy_source, None);
        let problem = r.policy_problem.unwrap();
        assert_eq!(problem.kind, "untrusted_owner");
        assert!(problem.detail.contains("Administrators or SYSTEM"));
        // The notice is worded by the app, in the user's language.
        assert_eq!(problem.detail_key, Some("notAdminOwned"));
        assert_eq!(problem.detail_args.get("path").map(String::as_str), Some(r"C:\ProgramData\BridgeLab"));
        assert!(!resolve_telemetry_with(None, &p, refuse).disabled_by_policy);
        // A missing file is not asked about at all.
        assert_eq!(resolve_with(None, Path::new("/nonexistent/p.json"), None, refuse).policy_problem, None);
    }

    #[cfg(unix)]
    #[test]
    fn an_unreadable_policy_file_is_reported() {
        // A folder where the file should be: it fails the same way for
        // root, which file permissions would not.
        let p = std::env::temp_dir().join(format!("bl-update-policy-{}-dir", std::process::id())).join("policy.json");
        std::fs::create_dir_all(&p).unwrap();
        let r = resolve(None, &p, None);
        assert!(!r.disabled_by_policy);
        assert_eq!(r.policy_problem.unwrap().kind, "unreadable");
    }

    /// The rule itself on Windows, on a file the test makes: it belongs to
    /// the account running the tests, which is neither SYSTEM nor (unless
    /// the tests run elevated) the Administrators group.
    #[cfg(target_os = "windows")]
    #[test]
    fn a_policy_file_made_by_a_standard_user_is_not_trusted() {
        let p = tmp("owner", r#"{"disable_update_check": true}"#);
        let security = win_security::object_security(&p).unwrap();
        if security.owner != ADMINISTRATORS && security.owner != SYSTEM {
            assert!(machine_trust_check(&p).is_err());
            assert!(super::resolve(None, &p, None).policy_problem.is_some_and(|x| x.kind == "untrusted_owner"));
        }
        assert_eq!(win_security::account_name("S-1-5-18"), r"NT AUTHORITY\SYSTEM");
        assert_eq!(win_security::account_name("not a sid"), "not a sid");
    }

    // The Windows rule on the owners and permissions it reads.

    const USERS: &str = "S-1-5-32-545";
    const AUTHENTICATED_USERS: &str = "S-1-5-11";
    const INTERACTIVE: &str = "S-1-5-4";
    /// A named local account and a custom group (machine domain SIDs).
    const ALICE: &str = "S-1-5-21-1004336348-1177238915-682003330-1001";
    const LAB_GROUP: &str = "S-1-5-21-1004336348-1177238915-682003330-1012";

    fn allow(sid: &str, mask: u32) -> Ace {
        Ace { sid: sid.into(), kind: AceKind::Allow, mask, inherit_only: false }
    }

    fn deny(sid: &str, mask: u32) -> Ace {
        Ace { sid: sid.into(), kind: AceKind::Deny, mask, inherit_only: false }
    }

    const FULL: u32 = 0x001F_01FF;
    const READ_EXECUTE: u32 = 0x0012_00A9;
    const MODIFY: u32 = 0x0013_01BF;

    /// %ProgramData%\BridgeLab as an administrator creates it: inherited
    /// SYSTEM and Administrators full control, CREATOR OWNER (inherit
    /// only), Users read and execute, and Users "create files / write
    /// data" and "create folders / append data" on the folder itself.
    fn default_folder() -> ObjectSecurity {
        ObjectSecurity {
            owner: ADMINISTRATORS.into(),
            dacl: Some(vec![
                allow(SYSTEM, FULL),
                allow(ADMINISTRATORS, FULL),
                Ace { sid: CREATOR_OWNER.into(), kind: AceKind::Allow, mask: GENERIC_ALL, inherit_only: true },
                allow(USERS, READ_EXECUTE),
                allow(USERS, FILE_WRITE_DATA | FILE_APPEND_DATA | 0x0010 | 0x0100),
            ]),
        }
    }

    /// policy.json an administrator created in it: what files inherit.
    fn default_file() -> ObjectSecurity {
        ObjectSecurity {
            owner: ADMINISTRATORS.into(),
            dacl: Some(vec![allow(SYSTEM, FULL), allow(ADMINISTRATORS, FULL), allow(USERS, READ_EXECUTE)]),
        }
    }

    fn with(mut object: ObjectSecurity, ace: Ace) -> ObjectSecurity {
        object.dacl.as_mut().unwrap().push(ace);
        object
    }

    #[test]
    fn the_default_program_data_permissions_are_trusted() {
        assert_eq!(trust_decision(&default_file(), &default_folder()), Ok(()));
        let system_owned = ObjectSecurity { owner: SYSTEM.into(), ..default_file() };
        assert_eq!(trust_decision(&system_owned, &default_folder()), Ok(()));
    }

    #[test]
    fn administrators_system_and_trusted_installer_may_hold_full_control() {
        let admin_only = || ObjectSecurity {
            owner: ADMINISTRATORS.into(),
            dacl: Some(vec![allow(ADMINISTRATORS, FULL), allow(SYSTEM, FULL), allow(TRUSTED_INSTALLER, FULL)]),
        };
        assert_eq!(trust_decision(&admin_only(), &admin_only()), Ok(()));
        // The owner's stand-ins applying to the object itself: the owner
        // is an administrator.
        let owner_entries = with(with(default_file(), allow(CREATOR_OWNER, FULL)), allow(OWNER_RIGHTS, FULL));
        assert_eq!(trust_decision(&owner_entries, &default_folder()), Ok(()));
        // An empty DACL grants nobody anything.
        let empty = ObjectSecurity { owner: SYSTEM.into(), dacl: Some(Vec::new()) };
        assert_eq!(trust_decision(&empty, &admin_only()), Ok(()));
    }

    #[test]
    fn a_file_or_folder_not_owned_by_an_administrator_is_refused() {
        let user_file = ObjectSecurity { owner: ALICE.into(), ..default_file() };
        assert_eq!(trust_decision(&user_file, &default_folder()), Err(Distrust::Owner { folder: false }));
        let user_folder = ObjectSecurity { owner: USERS.into(), ..default_folder() };
        assert_eq!(trust_decision(&default_file(), &user_folder), Err(Distrust::Owner { folder: true }));
        let no_owner = ObjectSecurity { owner: String::new(), ..default_file() };
        assert_eq!(trust_decision(&no_owner, &default_folder()), Err(Distrust::Owner { folder: false }));
    }

    #[test]
    fn anyone_else_who_may_change_the_file_is_refused() {
        for (sid, mask) in [
            (ALICE, FILE_WRITE_DATA), // a named local user
            (LAB_GROUP, MODIFY),      // a custom group
            ("S-1-5-21-3623811015-3361044348-30300820-1013", GENERIC_WRITE), // a domain account
            ("S-1-5-99-1", FILE_APPEND_DATA), // an unknown SID
            (USERS, MODIFY),
            (AUTHENTICATED_USERS, FILE_WRITE_DATA),
            (EVERYONE, GENERIC_WRITE),
            (INTERACTIVE, FILE_APPEND_DATA),
            (ALICE, WRITE_DAC),
            (USERS, WRITE_OWNER),
            (USERS, DELETE),
            (LAB_GROUP, GENERIC_ALL),
        ] {
            assert_eq!(
                trust_decision(&with(default_file(), allow(sid, mask)), &default_folder()),
                Err(Distrust::Writable { sid: sid.into(), folder: false }),
                "{sid} {mask:#x}"
            );
        }
        // A NULL DACL lets everyone do everything.
        let open = ObjectSecurity { owner: ADMINISTRATORS.into(), dacl: None };
        assert_eq!(
            trust_decision(&open, &default_folder()),
            Err(Distrust::Writable { sid: EVERYONE.into(), folder: false })
        );
        // Reading is fine, whoever reads.
        assert_eq!(trust_decision(&with(default_file(), allow(ALICE, READ_EXECUTE)), &default_folder()), Ok(()));
    }

    #[test]
    fn a_folder_that_lets_anyone_else_delete_or_replace_the_file_is_refused() {
        for (sid, mask) in [
            (LAB_GROUP, FILE_DELETE_CHILD), // a custom group with delete-child
            (USERS, FILE_DELETE_CHILD),
            (ALICE, DELETE),
            (USERS, WRITE_DAC),
            (EVERYONE, WRITE_OWNER),
            (USERS, GENERIC_ALL),
            (AUTHENTICATED_USERS, MODIFY | FILE_DELETE_CHILD),
        ] {
            assert_eq!(
                trust_decision(&default_file(), &with(default_folder(), allow(sid, mask))),
                Err(Distrust::Writable { sid: sid.into(), folder: true }),
                "{sid} {mask:#x}"
            );
        }
        // Creating files there is fine: a file a user creates is theirs.
        assert_eq!(trust_decision(&default_file(), &with(default_folder(), allow(ALICE, GENERIC_WRITE))), Ok(()));
    }

    #[test]
    fn deny_entries_cancel_only_their_own_sid_and_inherit_only_entries_do_not_apply() {
        let change = FILE_WRITE_DATA | FILE_APPEND_DATA | DELETE | WRITE_DAC | WRITE_OWNER;
        // A deny before the allow, for the same SID, wins.
        let mut file = default_file();
        let dacl = file.dacl.as_mut().unwrap();
        dacl.insert(0, deny(USERS, change));
        dacl.push(allow(USERS, MODIFY));
        assert_eq!(trust_decision(&file, &default_folder()), Ok(()));
        // A deny for another SID does not take away this one's right.
        let mut other = default_file();
        let dacl = other.dacl.as_mut().unwrap();
        dacl.insert(0, deny(EVERYONE, change));
        dacl.push(allow(ALICE, FILE_WRITE_DATA));
        assert_eq!(
            trust_decision(&other, &default_folder()),
            Err(Distrust::Writable { sid: ALICE.into(), folder: false })
        );
        // A deny after the allow comes too late.
        let late = with(with(default_file(), allow(USERS, FILE_WRITE_DATA)), deny(USERS, FILE_WRITE_DATA));
        assert_eq!(
            trust_decision(&late, &default_folder()),
            Err(Distrust::Writable { sid: USERS.into(), folder: false })
        );
        // An entry only passed down to new children does not apply.
        let inherited = with(default_file(), Ace { sid: ALICE.into(), kind: AceKind::Allow, mask: FULL, inherit_only: true });
        assert_eq!(trust_decision(&inherited, &default_folder()), Ok(()));
    }

    #[test]
    fn the_refusal_names_who_can_change_it() {
        let (file, folder) = (Path::new(r"C:\ProgramData\BridgeLab\policy.json"), Path::new(r"C:\ProgramData\BridgeLab"));
        let name = |sid: &str| if sid == ALICE { r"PC\alice".to_string() } else { sid.to_string() };
        let p = Distrust::Writable { sid: ALICE.into(), folder: false }.problem(file, folder, name);
        assert_eq!(p.kind, "untrusted_permissions");
        assert!(p.detail.starts_with(r"PC\alice can change"), "{}", p.detail);
        assert!(p.detail.contains("policy.json"));
        // Unnamed: the SID itself.
        let p = Distrust::Writable { sid: LAB_GROUP.into(), folder: true }.problem(file, folder, name);
        assert!(p.detail.starts_with(&format!("{LAB_GROUP} can delete or replace")), "{}", p.detail);
        assert_eq!(Distrust::Owner { folder: true }.problem(file, folder, name).kind, "untrusted_owner");
        let p = Distrust::Unevaluable { ace_type: 0x4, folder: false }.problem(file, folder, name);
        assert_eq!(p.kind, "untrusted_permissions");
        assert!(p.detail.contains("cannot evaluate (ACE type 0x4)"), "{}", p.detail);
    }

    fn entry(sid: &str, kind: AceKind, mask: u32) -> Ace {
        Ace { sid: sid.into(), kind, mask, inherit_only: false }
    }

    /// A callback (or object) allow grants its rights whenever its
    /// condition holds: it counts as an allow.
    #[test]
    fn a_conditional_allow_for_anyone_else_is_refused() {
        // A callback allow giving Users write access.
        let callback = with(default_file(), entry(USERS, AceKind::AllowConditional, FILE_WRITE_DATA));
        assert_eq!(
            trust_decision(&callback, &default_folder()),
            Err(Distrust::Writable { sid: USERS.into(), folder: false })
        );
        // An object allow giving a named user write access.
        let object = with(default_file(), entry(ALICE, AceKind::AllowConditional, MODIFY));
        assert_eq!(
            trust_decision(&object, &default_folder()),
            Err(Distrust::Writable { sid: ALICE.into(), folder: false })
        );
        // On the folder too.
        let folder = with(default_folder(), entry(LAB_GROUP, AceKind::AllowConditional, FILE_DELETE_CHILD));
        assert_eq!(
            trust_decision(&default_file(), &folder),
            Err(Distrust::Writable { sid: LAB_GROUP.into(), folder: true })
        );
        // For an administrator it is fine.
        let admin = with(default_file(), entry(ADMINISTRATORS, AceKind::AllowConditional, FULL));
        assert_eq!(trust_decision(&admin, &default_folder()), Ok(()));
    }

    /// A conditional deny may not apply, so it takes nothing away, before
    /// or after the allow.
    #[test]
    fn a_conditional_deny_cancels_nothing() {
        let before = {
            let mut file = default_file();
            file.dacl.as_mut().unwrap().insert(0, entry(USERS, AceKind::DenyConditional, FULL));
            with(file, allow(USERS, FILE_WRITE_DATA))
        };
        assert_eq!(
            trust_decision(&before, &default_folder()),
            Err(Distrust::Writable { sid: USERS.into(), folder: false })
        );
        let after = with(with(default_file(), allow(ALICE, FILE_WRITE_DATA)), entry(ALICE, AceKind::DenyConditional, FULL));
        assert_eq!(
            trust_decision(&after, &default_folder()),
            Err(Distrust::Writable { sid: ALICE.into(), folder: false })
        );
        // On its own it is ignored, not a reason to refuse.
        let alone = with(default_file(), entry("", AceKind::DenyConditional, FULL));
        assert_eq!(trust_decision(&alone, &default_folder()), Ok(()));
    }

    /// An entry whose rights cannot be told refuses the policy, unless it
    /// does not apply to the object.
    #[test]
    fn an_entry_of_unknown_type_is_refused() {
        for ace_type in [0x4, 0x2, 0x11, 0x16] {
            let file = with(default_file(), entry("", AceKind::Unknown(ace_type), 0));
            assert_eq!(
                trust_decision(&file, &default_folder()),
                Err(Distrust::Unevaluable { ace_type, folder: false }),
                "{ace_type:#x}"
            );
            let folder = with(default_folder(), entry("", AceKind::Unknown(ace_type), 0));
            assert_eq!(
                trust_decision(&default_file(), &folder),
                Err(Distrust::Unevaluable { ace_type, folder: true }),
                "{ace_type:#x}"
            );
        }
        let inherit_only = with(
            default_file(),
            Ace { sid: String::new(), kind: AceKind::Unknown(0x4), mask: 0, inherit_only: true },
        );
        assert_eq!(trust_decision(&inherit_only, &default_folder()), Ok(()));
        // The default %ProgramData% permissions still pass.
        assert_eq!(trust_decision(&default_file(), &default_folder()), Ok(()));
    }

    #[test]
    fn telemetry_policy_from_environment_or_file() {
        let missing = Path::new("/nonexistent/p.json");
        assert_eq!(
            resolve_telemetry(None, missing),
            TelemetryPolicy { disabled_by_policy: false, policy_source: None }
        );
        let env = resolve_telemetry(Some("1".into()), missing);
        assert!(env.disabled_by_policy);
        assert!(env.policy_source.unwrap().contains(TELEMETRY_POLICY_ENV));
        assert!(!resolve_telemetry(Some("0".into()), missing).disabled_by_policy);

        let p = tmp("tel-on", r#"{"disable_telemetry": true}"#);
        let r = resolve_telemetry(None, &p);
        assert!(r.disabled_by_policy);
        assert_eq!(r.policy_source.as_deref(), Some(p.display().to_string().as_str()));
        assert!(!resolve(None, &p, None).disabled_by_policy, "telemetry key must not touch the update check");

        let upd = tmp("tel-upd", r#"{"disable_update_check": true}"#);
        assert!(!resolve_telemetry(None, &upd).disabled_by_policy, "update key must not touch telemetry");
        let both = tmp("tel-both", r#"{"disable_update_check": true, "disable_telemetry": true}"#);
        assert!(resolve_telemetry(None, &both).disabled_by_policy);
        assert!(resolve(None, &both, None).disabled_by_policy);
        let bad = tmp("tel-bad", "disable_telemetry = yes");
        assert!(!resolve_telemetry(None, &bad).disabled_by_policy);
    }

    #[test]
    fn the_installer_choice_is_reported_both_ways() {
        let missing = Path::new("/nonexistent/p.json");
        let no = tmp("inst-no", r#"{"update_check_on_startup": false}"#);
        let yes = tmp("inst-yes", r#"{"update_check_on_startup": true}"#);
        assert_eq!(resolve(None, missing, Some(&no)).installer_choice, Some(false));
        assert_eq!(resolve(None, missing, Some(&yes)).installer_choice, Some(true));
        let junk = tmp("inst-junk", "{}");
        assert_eq!(resolve(None, missing, Some(&junk)).installer_choice, None);
    }

    #[test]
    fn a_policy_file_with_a_bom_or_string_values_still_applies() {
        let bom = tmp("bom", "\u{feff}{\"disable_telemetry\": true, \"disable_update_check\": true}");
        assert!(resolve(None, &bom, None).disabled_by_policy);
        assert!(resolve_telemetry(None, &bom).disabled_by_policy);
        let strings = tmp("strings", r#"{"disable_telemetry": "true", "disable_update_check": 1}"#);
        assert!(resolve(None, &strings, None).disabled_by_policy);
        assert!(resolve_telemetry(None, &strings).disabled_by_policy);
        let off = tmp("off", r#"{"disable_telemetry": "no"}"#);
        assert!(!resolve_telemetry(None, &off).disabled_by_policy);
    }
}
