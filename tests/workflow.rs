//! The M2 commands end to end: init, plan, draft, clean and law.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

const PASSPHRASE: &str = "present day, present time";

/// Run ghostsoap with `GHOSTSOAP_HOME` set to `home` and `input` on standard input.
fn ghostsoap(home: &Path, args: &[&str], input: &str) -> Output {
    ghostsoap_with(home, args, input, &[])
}

/// As `ghostsoap`, with the environment variables `vars` also set for the child only.
fn ghostsoap_with(home: &Path, args: &[&str], input: &str, vars: &[(&str, &Path)]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ghostsoap"))
        .args(args)
        .env("GHOSTSOAP_HOME", home)
        .envs(vars.iter().copied())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run ghostsoap");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Answers to `init`: name, other names, email, phone, date of birth, address, previous
/// addresses, postcode, then the passphrase twice.
fn init_answers(passphrase: &str) -> String {
    format!(
        "Lara Croft\n\nlara@croft-manor.example\n0400 000 001\n1968-02-14\n\
         1 Croft Manor Lane, Surrey Hills VIC 3127\n\n3127\n{passphrase}\n{passphrase}\n"
    )
}

fn init(home: &Path) {
    let out = ghostsoap(
        home,
        &["init", "--passphrase-stdin"],
        &init_answers(PASSPHRASE),
    );
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
}

#[test]
fn init_draft_and_clean() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let drafts = tmp.path().join("drafts");
    let drafts_arg = drafts.to_str().unwrap();
    init(&home);
    let state = fs::read(home.join("state.age")).unwrap();
    assert!(!text(&state).contains("Croft"), "the profile is encrypted");

    let again = ghostsoap(
        &home,
        &["init", "--passphrase-stdin"],
        &init_answers(PASSPHRASE),
    );
    assert_eq!(again.status.code(), Some(1));
    assert!(text(&again.stderr).contains("a profile already exists"));

    let args = [
        "draft",
        "--entry",
        "cotality-au",
        "--out",
        drafts_arg,
        "--passphrase-stdin",
    ];
    let out = ghostsoap(&home, &args, &format!("{PASSPHRASE}\n"));
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let stdout = text(&out.stdout);
    assert!(stdout.contains("draft   cotality-au  email to AU-Privacy@Cotality.com"));
    let files = emls(&drafts);
    assert_eq!(files.len(), 1);
    let eml = fs::read_to_string(&files[0]).unwrap();
    assert!(eml.starts_with("To: AU-Privacy@Cotality.com\r\nSubject: Privacy Act request [GS-"));
    assert!(eml.contains("Name: Lara Croft"));
    assert!(
        !eml.contains("1968-02-14"),
        "Cotality does not get the date of birth"
    );

    let wrong = ghostsoap(&home, &args, "not the passphrase\n");
    assert_eq!(wrong.status.code(), Some(1));
    assert!(text(&wrong.stderr).contains("error: wrong passphrase"));

    let out = ghostsoap(&home, &["clean", "--out", drafts_arg], "");
    assert_eq!(out.status.code(), Some(0));
    assert!(text(&out.stdout).contains("Deleted 1 drafts"));
    assert!(drafts.exists(), "a folder named with --out stays");
    assert_eq!(fs::read_dir(&drafts).unwrap().count(), 0);
}

/// The `.eml` drafts in `dir`.
fn emls(dir: &Path) -> Vec<std::path::PathBuf> {
    fs::read_dir(dir)
        .unwrap()
        .map(|f| f.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "eml"))
        .collect()
}

/// Draft one letter to `out` with the environment variables `vars`; `extra` adds flags.
fn draft_to(home: &Path, out: &Path, vars: &[(&str, &Path)], extra: &[&str]) -> Output {
    let mut args = vec![
        "draft",
        "--entry",
        "cotality-au",
        "--passphrase-stdin",
        "--out",
    ];
    args.push(out.to_str().unwrap());
    args.extend(extra);
    ghostsoap_with(home, &args, &format!("{PASSPHRASE}\n"), vars)
}

#[test]
fn draft_warns_about_a_repository_or_sync_folder_but_still_writes() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    init(&home);
    // A home folder with a Dropbox in it, a OneDrive folder and a linked Git worktree.
    let [user, onedrive, repo] = ["user", "OneDrive", "repo"].map(|d| tmp.path().join(d));
    for dir in [&user.join("Dropbox"), &onedrive, &repo] {
        fs::create_dir_all(dir).unwrap();
    }
    fs::write(
        repo.join(".git"),
        "gitdir: ../elsewhere/.git/worktrees/repo\n",
    )
    .unwrap();
    let empty = Path::new("");
    let vars = [
        ("HOME", user.as_path()),
        ("USERPROFILE", &user),
        ("OneDrive", &onedrive),
        ("OneDriveCommercial", empty),
        ("OneDriveConsumer", empty),
    ];
    let warning = "warning: the drafts folder is inside the";
    for (out, expected) in [
        (repo.join("drafts"), "Git working tree|be committed"),
        (onedrive.join("drafts"), "OneDrive folder|be uploaded"),
        (user.join("Dropbox/drafts"), "Dropbox folder|be uploaded"),
        (tmp.path().join("plain"), ""),
    ] {
        let run = draft_to(&home, &out, &vars, &[]);
        let stderr = text(&run.stderr);
        assert_eq!(run.status.code(), Some(0), "not a refusal: {stderr}");
        assert_eq!(emls(&out).len(), 1, "the draft is still written");
        assert_eq!(
            stderr.matches(warning).count(),
            usize::from(!expected.is_empty())
        );
        for part in expected.split('|').filter(|p| !p.is_empty()) {
            assert!(stderr.contains(part), "{part:?} in {stderr}");
        }
    }
    let copy = draft_to(&home, &repo, &vars, &["--copy"]);
    assert_eq!(copy.status.code(), Some(0));
    assert!(
        !text(&copy.stderr).contains(warning),
        "--copy writes nothing"
    );
}

#[test]
fn copy_prints_instead_of_writing() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let drafts = tmp.path().join("drafts");
    init(&home);
    let args = [
        "draft",
        "--entry",
        "experian-marketing-au",
        "--copy",
        "--out",
        drafts.to_str().unwrap(),
        "--passphrase-stdin",
    ];
    let out = ghostsoap(&home, &args, &format!("{PASSPHRASE}\n"));
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let stdout = text(&out.stdout);
    assert!(stdout.contains("==== experian-marketing-au (web form)"));
    assert!(stdout.contains("==== experian-marketing-au (post)"));
    assert!(stdout.contains("PO Box 7405, St Kilda Road VIC 3004"));
    assert!(!drafts.exists());
}

#[test]
fn drafts_default_to_the_data_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    init(&home);
    let args = ["draft", "--entry", "cotality-au", "--passphrase-stdin"];
    let out = ghostsoap(&home, &args, &format!("{PASSPHRASE}\n"));
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert!(!text(&out.stdout).contains("mailto:"), "links are opt-in");
    let with_link = [&args[..], &["--mailto"]].concat();
    let out = ghostsoap(&home, &with_link, &format!("{PASSPHRASE}\n"));
    assert!(
        text(&out.stdout).contains("too long for a mailto: link"),
        "a 3-request letter is too long for a link"
    );
    assert_eq!(emls(&home.join("drafts")).len(), 2);
    let out = ghostsoap(&home, &["clean"], "");
    assert_eq!(out.status.code(), Some(0));
    assert!(!home.join("drafts").exists());
    assert!(home.join("state.age").exists(), "clean keeps the profile");
}

#[test]
fn init_rejects_a_short_passphrase_and_bad_details() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let out = ghostsoap(
        &home,
        &["init", "--passphrase-stdin"],
        &init_answers("short"),
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("at least 12 characters"));
    let answers = init_answers(PASSPHRASE).replace("1968-02-14", "14/02/1968");
    let out = ghostsoap(&home, &["init", "--passphrase-stdin"], &answers);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("is not a date"));
    assert!(
        !text(&out.stderr).contains("14/02/1968"),
        "errors do not echo profile values"
    );
    assert!(!home.join("state.age").exists());
}

#[test]
fn draft_without_a_profile_says_to_run_init() {
    let tmp = tempfile::tempdir().unwrap();
    let args = [
        "draft",
        "--entry",
        "cotality-au",
        "--out",
        tmp.path().to_str().unwrap(),
        "--passphrase-stdin",
    ];
    let out = ghostsoap(&tmp.path().join("home"), &args, &format!("{PASSPHRASE}\n"));
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("run `ghostsoap init` first"));
}

#[test]
fn plan_needs_no_profile_and_shows_overseas_sites() {
    let tmp = tempfile::tempdir().unwrap();
    let out = ghostsoap(tmp.path(), &["plan", "--pack", "people-search"], "");
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let stdout = text(&out.stdout);
    assert!(stdout.contains("letter  acceleon-au  by email to privacy@acceleon.com.au"));
    assert!(stdout.contains(
        "    app7-no-marketing (APP 7.6(c), 7.7(a); a reasonable period, OAIC guide 30 days)"
    ));
    assert!(stdout.contains(
        "manual  locate-family-au: run from overseas; GhostSoap has not established that \
         Australian privacy law reaches it, so it drafts no letter. Search for yourself at"
    ));
    assert!(!tmp.path().join("state.age").exists());
}

#[test]
fn overseas_services_get_a_manual_step_without_a_search() {
    let tmp = tempfile::tempdir().unwrap();
    let out = ghostsoap(tmp.path(), &["plan", "--pack", "recruitment"], "");
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert!(text(&out.stdout).contains(
        "manual  linkedin-au: run from overseas; GhostSoap has not established that Australian \
         privacy law reaches it, so it drafts no letter. See \
         https://www.linkedin.com/help/linkedin/answer/63, then use its own opt-out or privacy \
         request form at https://www.linkedin.com/help/linkedin/ask/TSO-DPO"
    ));
}

#[test]
fn plan_refuses_access_levers_in_bulk() {
    let tmp = tempfile::tempdir().unwrap();
    let out = ghostsoap(
        tmp.path(),
        &["plan", "--pack", "credit", "--lever", "s20r-access"],
        "",
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("must be chosen for one entity at a time"));
    let out = ghostsoap(
        tmp.path(),
        &["plan", "--entry", "talefin-au", "--lever", "s20r-access"],
        "",
    );
    assert_eq!(out.status.code(), Some(0));
    assert!(text(&out.stdout).contains("warning: access requests return personal information"));
    let out = ghostsoap(
        tmp.path(),
        &["plan", "--entry", "talefin-au", "--lever", "app99"],
        "",
    );
    assert_eq!(out.status.code(), Some(2));
    assert!(text(&out.stderr).contains("unknown lever"));
}

#[test]
fn law_lists_every_lever_with_its_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let out = ghostsoap(tmp.path(), &["law"], "");
    assert_eq!(out.status.code(), Some(0));
    let stdout = text(&out.stdout);
    for line in [
        "s20r-access: Give me access to my credit reporting information",
        "    deadline: within a reasonable period, but not longer than 10 days, after the request is made (section 20R(3))",
        "spam-unsubscribe: Stop sending me commercial electronic messages",
        "    Privacy Act 1988 (Cth), compilation C2026C00227, in force from 4 June 2026",
    ] {
        assert!(stdout.contains(line), "missing {line:?}");
    }
}
