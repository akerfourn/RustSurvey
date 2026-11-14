use super::{SourceRuntimeFacts, flag, issue};
use crate::{GroupSettingsReport, SettingValue};
use rustsurvey_domain::{
    ValidationIssue,
    navigation::{NavigationSettings, PageFormat},
    participation::ParticipationPolicy,
};

pub(super) fn resolve(
    settings: &GroupSettingsReport,
    facts: &SourceRuntimeFacts,
    issues: &mut Vec<ValidationIssue>,
) -> Option<(bool, NavigationSettings, ParticipationPolicy)> {
    for (name, expected) in [
        ("savetimings", false),
        ("datestamp", true),
        ("allowregister", false),
        ("allowsave", true),
        ("autoredirect", false),
        ("allowprev", true),
        ("printanswers", false),
        ("ipaddr", false),
        ("refurl", false),
        ("publicstatistics", false),
        ("publicgraphs", false),
        ("listpublic", false),
        ("sendconfirmation", false),
        ("assessments", false),
        ("showxquestions", false),
        ("shownoanswer", true),
        ("nokeyboard", false),
    ] {
        if flag(settings, name).is_some_and(|value| value != expected) {
            issues.push(issue(
                format!("source.surveys.{name}"),
                "La valeur effective exige une fonction ou une politique absente de ce profil.",
            ));
        }
    }
    for (name, expected) in [
        ("showgroupinfo", "B"),
        ("showqnumcode", "X"),
        ("questionindex", "0"),
    ] {
        if code(settings, name).is_some_and(|value| value != expected) {
            issues.push(issue(
                format!("source.surveys.{name}"),
                "Cette présentation source n'est pas encore qualifiée.",
            ));
        }
    }
    if settings
        .settings
        .iter()
        .find(|item| item.name == "navigationdelay")
        .and_then(|item| item.value.as_ref())
        != Some(&SettingValue::Integer(0))
    {
        issues.push(issue(
            "source.surveys.navigationdelay",
            "Le profil exige un délai de navigation nul.",
        ));
    }
    let format = match code(settings, "format")? {
        "A" => PageFormat::All,
        "G" => PageFormat::Group,
        "S" => PageFormat::Question,
        _ => return None,
    };
    let policy = if facts.has_participants_table {
        let persistence =
            !flag(settings, "anonymized")? && flag(settings, "tokenanswerspersistence")?;
        let edit = flag(settings, "alloweditaftercompletion")?;
        if edit && !persistence {
            issues.push(issue(
                "source.surveys.alloweditaftercompletion",
                "La correction sans réponse liée au jeton reste à qualifier.",
            ));
        }
        ParticipationPolicy::Invitation {
            persistence,
            allow_edit_after_submit: edit && persistence,
        }
    } else if flag(settings, "usecookie")? {
        ParticipationPolicy::BrowserOnce
    } else {
        ParticipationPolicy::Open
    };
    Some((
        flag(settings, "showwelcome")?,
        NavigationSettings {
            format,
            show_progress: flag(settings, "showprogress")?,
        },
        policy,
    ))
}

fn code<'a>(settings: &'a GroupSettingsReport, name: &str) -> Option<&'a str> {
    match settings
        .settings
        .iter()
        .find(|item| item.name == name)?
        .value
        .as_ref()?
    {
        SettingValue::Code(code) => Some(code),
        _ => None,
    }
}
