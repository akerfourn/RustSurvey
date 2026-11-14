use super::{SourceRuntimeFacts, issue};
use crate::xml::Document;
use rustsurvey_domain::ValidationIssue;

pub(super) fn projected(section: &str) -> Option<Vec<&'static str>> {
    let extra: &[&str] = match section {
        "surveys" => &[
            "owner_id",
            "gsid",
            "admin",
            "active",
            "adminemail",
            "expires",
            "startdate",
            "format",
            "template",
            "anonymized",
            "savetimings",
            "datestamp",
            "usecookie",
            "allowregister",
            "allowsave",
            "autonumber_start",
            "tokenlength",
            "autoredirect",
            "allowprev",
            "printanswers",
            "ipaddr",
            "ipanonymize",
            "refurl",
            "datecreated",
            "publicstatistics",
            "publicgraphs",
            "listpublic",
            "htmlemail",
            "sendconfirmation",
            "tokenanswerspersistence",
            "assessments",
            "usecaptcha",
            "usetokens",
            "bounce_email",
            "attributedescriptions",
            "emailresponseto",
            "emailnotificationto",
            "showxquestions",
            "showgroupinfo",
            "shownoanswer",
            "showqnumcode",
            "showwelcome",
            "showprogress",
            "questionindex",
            "navigationdelay",
            "nokeyboard",
            "alloweditaftercompletion",
            "showsurveypolicynotice",
            "bouncetime",
            "bounceprocessing",
            "bounceaccounttype",
            "bounceaccounthost",
            "bounceaccountpass",
            "bounceaccountencryption",
            "bounceaccountuser",
            "googleanalyticsstyle",
            "googleanalyticsapikey",
            "tokenencryptionoptions",
        ],
        "group_l10ns" => &["sid", "group_order", "randomization_group", "grelevance"],
        "questions" => &["same_default", "encrypted", "modulename", "same_script"],
        "subquestions" => &[
            "type",
            "other",
            "mandatory",
            "same_default",
            "preg",
            "encrypted",
            "question_theme_name",
            "modulename",
            "same_script",
        ],
        "question_l10ns" => &["script"],
        "answers" => &["assessment_value"],
        "surveys_languagesettings" => &[
            "surveyls_description",
            "surveyls_url",
            "surveyls_urldescription",
            "surveyls_policy_notice",
            "surveyls_policy_error",
            "surveyls_policy_notice_label",
            "surveyls_email_invite_subj",
            "surveyls_email_invite",
            "surveyls_email_remind_subj",
            "surveyls_email_remind",
            "surveyls_email_register_subj",
            "surveyls_email_register",
            "surveyls_email_confirm_subj",
            "surveyls_email_confirm",
            "surveyls_dateformat",
            "surveyls_attributecaptions",
            "surveyls_alias",
            "email_admin_notification_subj",
            "email_admin_notification",
            "email_admin_responses_subj",
            "email_admin_responses",
            "surveyls_numberformat",
            "attachments",
        ],
        "defaultvalues" => return Some(vec!["dvid", "qid", "sqid", "scale_id", "specialtype"]),
        "defaultvalue_l10ns" => return Some(vec!["id", "dvid", "language", "defaultvalue"]),
        _ => &[],
    };
    crate::execution::fields(section).map(|fields| fields.iter().chain(extra).copied().collect())
}

pub(super) fn validate(
    xml: &Document,
    facts: &SourceRuntimeFacts,
    issues: &mut Vec<ValidationIssue>,
) {
    check_metadata(xml, issues);
    for row in rows(xml, "subquestions") {
        if issues.len() >= 128 {
            break;
        }
        for (name, allowed) in [
            ("other", &["", "N"][..]),
            ("mandatory", &["", "N"][..]),
            ("same_default", &["", "0"][..]),
        ] {
            if xml
                .field(row, name)
                .is_some_and(|value| !allowed.contains(&value))
            {
                issues.push(issue(
                    format!("source.subquestions.{name}"),
                    "Une option de sous-question n'est pas représentée par ce profil.",
                ));
            }
        }
        let parent = xml.field(row, "parent_qid").and_then(|id| {
            rows(xml, "questions").find(|parent| xml.field(*parent, "qid") == Some(id))
        });
        if xml
            .field(row, "type")
            .is_some_and(|kind| parent.and_then(|parent| xml.field(parent, "type")) != Some(kind))
        {
            issues.push(issue(
                "source.subquestions.type",
                "Le type de sous-question diffère de son parent.",
            ));
        }
    }
    for row in rows(xml, "answers") {
        if issues.len() >= 128 {
            break;
        }
        if xml
            .field(row, "assessment_value")
            .is_some_and(|value| !["", "0"].contains(&value))
        {
            issues.push(issue(
                "source.answers.assessment_value",
                "Les évaluations de choix ne sont pas prises en charge.",
            ));
        }
    }
    for row in rows(xml, "surveys_languagesettings") {
        if issues.len() >= 128 {
            break;
        }
        for name in ["surveyls_description", "surveyls_url"] {
            if xml.field(row, name).is_some_and(|value| !value.is_empty()) {
                issues.push(issue(
                    format!("source.surveys_languagesettings.{name}"),
                    "Ce contenu de présentation ou lien doit encore être projeté.",
                ));
            }
        }
        if xml
            .field(row, "surveyls_numberformat")
            .is_some_and(|value| !["", "0"].contains(&value))
        {
            issues.push(issue(
                "source.surveys_languagesettings.surveyls_numberformat",
                "Ce format numérique source reste à prendre en charge.",
            ));
        }
    }
    let Some(table) = xml.children_named(xml.root, "surveys").next() else {
        return;
    };
    let Some(rows) = xml.children_named(table, "rows").next() else {
        return;
    };
    let Some(row) = xml.children_named(rows, "row").next() else {
        return;
    };
    for (name, expected) in [
        ("showsurveypolicynotice", "0"),
        ("bounceprocessing", "N"),
        ("googleanalyticsstyle", "0"),
    ] {
        if xml.field(row, name) != Some(expected) {
            issues.push(issue(format!("source.surveys.{name}"), "Une politique, un traitement de courriel ou un suivi externe reste actif ou inconnu."));
        }
    }
    for name in [
        "expires",
        "startdate",
        "emailresponseto",
        "emailnotificationto",
    ] {
        let declared = xml.children_named(table, "fields").any(|fields| {
            xml.children_named(fields, "fieldname")
                .any(|field| xml.elements[field].text == name)
        });
        if !declared || xml.field(row, name).is_some_and(|value| !value.is_empty()) {
            issues.push(issue(
                format!("source.surveys.{name}"),
                "Une colonne explicite, sans date d'accès ni notification active, est requise.",
            ));
        }
    }
    if xml
        .field(row, "template")
        .is_some_and(|value| !["", "inherit", "I", facts.effective_theme.as_str()].contains(&value))
    {
        issues.push(issue(
            "source.surveys.template",
            "Le thème explicite contredit le thème effectif fourni.",
        ));
    }
    if xml.field(row, "usecaptcha") != Some("N") {
        issues.push(issue(
            "source.surveys.usecaptcha",
            "Le CAPTCHA doit être explicitement désactivé dans ce profil.",
        ));
    }
}

fn check_metadata(xml: &Document, issues: &mut Vec<ValidationIssue>) {
    for row in rows(xml, "group_l10ns") {
        if issues.len() >= 128 {
            return;
        }
        let parent = xml
            .field(row, "gid")
            .and_then(|id| rows(xml, "groups").find(|group| xml.field(*group, "gid") == Some(id)));
        for name in ["sid", "group_order", "randomization_group", "grelevance"] {
            if xml.field(row, name).is_some_and(|value| {
                parent.and_then(|parent| xml.field(parent, name)) != Some(value)
            }) {
                issues.push(issue(
                    format!("source.group_l10ns.{name}"),
                    "Le champ dupliqué contredit les données du groupe.",
                ));
            }
        }
    }
    for section in ["questions", "subquestions"] {
        for row in rows(xml, section) {
            if issues.len() >= 128 {
                return;
            }
            for (name, allowed) in [
                ("encrypted", &["", "N", "0"][..]),
                ("modulename", &[""][..]),
                ("same_script", &["", "0", "1"][..]),
            ] {
                if xml
                    .field(row, name)
                    .is_some_and(|value| !allowed.contains(&value))
                {
                    issues.push(issue(
                        format!("source.{section}.{name}"),
                        "Une option de chiffrement, module ou script n'est pas prise en charge.",
                    ));
                }
            }
        }
    }
    for row in rows(xml, "question_l10ns") {
        if issues.len() >= 128 {
            return;
        }
        if xml
            .field(row, "script")
            .is_some_and(|value| !value.is_empty())
        {
            issues.push(issue(
                "source.question_l10ns.script",
                "Le JavaScript embarqué n'est jamais exécuté.",
            ));
        }
    }
    for row in rows(xml, "subquestions") {
        if issues.len() >= 128 {
            return;
        }
        if xml
            .field(row, "preg")
            .is_some_and(|value| !value.is_empty())
        {
            issues.push(issue(
                "source.subquestions.preg",
                "La validation regex d'une sous-question reste à implémenter.",
            ));
        }
        let parent = xml.field(row, "parent_qid").and_then(|id| {
            rows(xml, "questions").find(|parent| xml.field(*parent, "qid") == Some(id))
        });
        if xml.field(row, "question_theme_name").is_some_and(|value| {
            !value.is_empty()
                && parent.and_then(|parent| xml.field(parent, "question_theme_name")) != Some(value)
        }) {
            issues.push(issue(
                "source.subquestions.question_theme_name",
                "Un rendu propre à la sous-question reste à qualifier.",
            ));
        }
    }
}

fn rows<'a>(xml: &'a Document, name: &'a str) -> impl Iterator<Item = usize> + 'a {
    xml.children_named(xml.root, name)
        .flat_map(move |table| xml.children_named(table, "rows"))
        .flat_map(move |rows| xml.children_named(rows, "row"))
}
